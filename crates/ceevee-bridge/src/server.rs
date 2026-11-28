//! A very small HTTP/1.1 server for the few calls the extension makes. It is written by hand so
//! the app carries no extra web framework, and it answers only on the loopback address.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use ceevee_apply::check::State;
use ceevee_apply::engine::{plan_fields, Action, Answers, FillReport, FormField, Input};
use ceevee_apply::{check_navigation, detect};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::protocol::{
    CheckLine, Document, PairRequest, PairStarted, PairStatus, Ping, PlanReply, PlanRequest, Step,
};

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// The first port tried, and how many follow it. The extension looks for the app on these.
pub const FIRST_PORT: u16 = 47831;
pub const PORT_COUNT: u16 = 10;

const MAX_HEADERS: usize = 16 * 1024;
const MAX_BODY: usize = 1024 * 1024;

/// What the app gives the bridge to answer with.
pub struct Backend {
    /// The person's answers for filling forms.
    pub answers: Box<dyn Fn() -> BoxFuture<Result<Answers, String>> + Send + Sync>,
    /// A document to attach: `resume` or `cover_letter`. `None` when there is none.
    #[allow(clippy::type_complexity)]
    pub document:
        Box<dyn Fn(String) -> BoxFuture<Result<Option<(String, Vec<u8>)>, String>> + Send + Sync>,
    /// Called with the full list of approved tokens whenever it changes, so the app can keep it.
    pub tokens_changed: Box<dyn Fn(Vec<String>) + Send + Sync>,
}

/// A browser asking to be connected, waiting for the person to approve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPair {
    pub id: String,
    pub browser: String,
}

#[derive(Default)]
struct Pairing {
    tokens: HashSet<String>,
    pending: Option<PendingPair>,
    /// The outcome of the last request, kept until the extension has read it.
    outcome: Option<(String, Outcome)>,
}

enum Outcome {
    Approved(String),
    Denied,
}

struct Shared {
    backend: Backend,
    pairing: Mutex<Pairing>,
    port: u16,
}

/// The running server. Dropping it does not stop it; call [`Bridge::stop`].
pub struct Bridge {
    shared: Arc<Shared>,
    task: tokio::task::JoinHandle<()>,
}

fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("the operating system can give random bytes");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

impl Bridge {
    /// Start listening on the first free port in the range. `tokens` are the extensions the
    /// person approved earlier.
    pub async fn start(backend: Backend, tokens: Vec<String>) -> Result<Self, String> {
        let mut listener = None;
        for p in FIRST_PORT..FIRST_PORT + PORT_COUNT {
            if let Ok(l) = TcpListener::bind(("127.0.0.1", p)).await {
                listener = Some(l);
                break;
            }
        }
        let Some(listener) = listener else {
            return Err("no free port for the browser extension connection".into());
        };
        Self::start_on(listener, backend, tokens)
    }

    /// Start on an existing listener (tests use port 0).
    pub fn start_on(
        listener: TcpListener,
        backend: Backend,
        tokens: Vec<String>,
    ) -> Result<Self, String> {
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let shared = Arc::new(Shared {
            backend,
            pairing: Mutex::new(Pairing {
                tokens: tokens.into_iter().collect(),
                ..Pairing::default()
            }),
            port,
        });
        let s = shared.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                let s = s.clone();
                tokio::spawn(async move {
                    let _ = tokio::time::timeout(Duration::from_secs(15), handle(stream, s)).await;
                });
            }
        });
        Ok(Self { shared, task })
    }

    pub fn port(&self) -> u16 {
        self.shared.port
    }

    pub fn stop(&self) {
        self.task.abort();
    }

    /// The browser waiting for approval, if any.
    pub fn pending(&self) -> Option<PendingPair> {
        self.shared.pairing.lock().ok()?.pending.clone()
    }

    /// How many browsers are connected.
    pub fn connected(&self) -> usize {
        self.shared.pairing.lock().map_or(0, |p| p.tokens.len())
    }

    /// The person approved the waiting browser. The extension collects its token next.
    pub fn approve(&self, id: &str) -> bool {
        let tokens = {
            let Ok(mut p) = self.shared.pairing.lock() else {
                return false;
            };
            if p.pending.as_ref().is_none_or(|x| x.id != id) {
                return false;
            }
            p.pending = None;
            let token = random_hex(32);
            p.tokens.insert(token.clone());
            p.outcome = Some((id.to_string(), Outcome::Approved(token)));
            p.tokens.iter().cloned().collect::<Vec<_>>()
        };
        (self.shared.backend.tokens_changed)(tokens);
        true
    }

    pub fn deny(&self, id: &str) -> bool {
        let Ok(mut p) = self.shared.pairing.lock() else {
            return false;
        };
        if p.pending.as_ref().is_none_or(|x| x.id != id) {
            return false;
        }
        p.pending = None;
        p.outcome = Some((id.to_string(), Outcome::Denied));
        true
    }

    /// Disconnect every browser. They have to be approved again.
    pub fn disconnect_all(&self) {
        if let Ok(mut p) = self.shared.pairing.lock() {
            p.tokens.clear();
        }
        (self.shared.backend.tokens_changed)(Vec::new());
    }
}

struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

struct Response {
    status: u16,
    body: String,
}

fn json<T: serde::Serialize>(status: u16, v: &T) -> Response {
    Response {
        status,
        body: serde_json::to_string(v).unwrap_or_else(|_| "{}".into()),
    }
}

fn error(status: u16, message: &str) -> Response {
    json(status, &serde_json::json!({ "error": message }))
}

async fn read_request(stream: &mut TcpStream) -> Result<Request, Response> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > MAX_HEADERS {
            return Err(error(431, "headers too large"));
        }
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|_| error(400, "unreadable request"))?;
        if n == 0 {
            return Err(error(400, "incomplete request"));
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let (Some(method), Some(path)) = (parts.next(), parts.next()) else {
        return Err(error(400, "bad request line"));
    };
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let len: usize = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    if len > MAX_BODY {
        return Err(error(413, "body too large"));
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < len {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|_| error(400, "unreadable body"))?;
        if n == 0 {
            return Err(error(400, "incomplete body"));
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    Ok(Request {
        method: method.to_string(),
        path: path.split('?').next().unwrap_or(path).to_string(),
        headers,
        body,
    })
}

async fn handle(mut stream: TcpStream, shared: Arc<Shared>) {
    let resp = match read_request(&mut stream).await {
        Ok(req) => route(&req, &shared).await,
        Err(r) => r,
    };
    let reason = match resp.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let out = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        resp.status,
        reason,
        resp.body.len(),
        resp.body
    );
    let _ = stream.write_all(out.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// Only extensions may call: a web page always sends an `http` or `https` origin.
fn origin_ok(origin: Option<&str>) -> bool {
    match origin {
        None => true,
        Some(o) => [
            "chrome-extension://",
            "moz-extension://",
            "safari-web-extension://",
        ]
        .iter()
        .any(|p| o.starts_with(p)),
    }
}

/// Only this computer's own address: stops a web page reaching the app by DNS rebinding.
fn host_ok(host: Option<&str>, port: u16) -> bool {
    matches!(host, Some(h) if h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}"))
}

fn token_ok(req: &Request, shared: &Shared) -> bool {
    let Some(t) = req
        .header("authorization")
        .and_then(|v| v.strip_prefix("Bearer "))
    else {
        return false;
    };
    shared
        .pairing
        .lock()
        .is_ok_and(|p| p.tokens.contains(t.trim()))
}

async fn route(req: &Request, shared: &Arc<Shared>) -> Response {
    if !host_ok(req.header("host"), shared.port) {
        return error(403, "wrong host");
    }
    if !origin_ok(req.header("origin")) {
        return error(403, "only the browser extension may call this");
    }
    let path = req.path.as_str();
    match (req.method.as_str(), path) {
        ("GET", "/ping") => json(
            200,
            &Ping {
                app: "CeeVee".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        ),
        ("POST", "/pair") => {
            let Ok(r) = serde_json::from_slice::<PairRequest>(&req.body) else {
                return error(400, "expected {\"browser\": ...}");
            };
            let browser: String = r
                .browser
                .chars()
                .filter(|c| !c.is_control())
                .take(40)
                .collect();
            let id = random_hex(8);
            if let Ok(mut p) = shared.pairing.lock() {
                p.pending = Some(PendingPair {
                    id: id.clone(),
                    browser,
                });
                p.outcome = None;
            }
            json(200, &PairStarted { id })
        }
        ("GET", p) if p.starts_with("/pair/") => {
            let id = &p["/pair/".len()..];
            let Ok(mut pairing) = shared.pairing.lock() else {
                return error(400, "unavailable");
            };
            if pairing.pending.as_ref().is_some_and(|x| x.id == id) {
                return json(
                    200,
                    &PairStatus {
                        state: "pending".into(),
                        token: None,
                    },
                );
            }
            match pairing.outcome.take() {
                Some((oid, Outcome::Approved(token))) if oid == id => json(
                    200,
                    &PairStatus {
                        state: "approved".into(),
                        token: Some(token),
                    },
                ),
                Some((oid, Outcome::Denied)) if oid == id => json(
                    200,
                    &PairStatus {
                        state: "denied".into(),
                        token: None,
                    },
                ),
                other => {
                    pairing.outcome = other;
                    error(404, "unknown request")
                }
            }
        }
        (_, "/ping" | "/pair" | "/plan") | (_, "/document/resume" | "/document/cover_letter") => {
            if matches!(
                path,
                "/plan" | "/document/resume" | "/document/cover_letter"
            ) && !token_ok(req, shared)
            {
                return error(401, "not connected");
            }
            match (req.method.as_str(), path) {
                ("POST", "/plan") => plan(req, shared).await,
                ("GET", "/document/resume") => document(shared, "resume").await,
                ("GET", "/document/cover_letter") => document(shared, "cover_letter").await,
                _ => error(405, "wrong method"),
            }
        }
        _ => error(404, "not found"),
    }
}

async fn document(shared: &Arc<Shared>, kind: &str) -> Response {
    match (shared.backend.document)(kind.to_string()).await {
        Ok(Some((file_name, bytes))) => json(
            200,
            &Document {
                file_name,
                data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
            },
        ),
        Ok(None) => error(404, "no document yet"),
        Err(e) => error(400, &e),
    }
}

fn input_of(f: &crate::protocol::Field) -> Input {
    match f.kind.as_str() {
        "textarea" => Input::Textarea,
        "select" => Input::Select(f.options.clone()),
        "checkbox" => Input::Checkbox,
        "file" => Input::File,
        _ => Input::Text,
    }
}

async fn plan(req: &Request, shared: &Arc<Shared>) -> Response {
    let Ok(r) = serde_json::from_slice::<PlanRequest>(&req.body) else {
        return error(400, "expected a page description");
    };
    // Pages automation must never touch get no answers at all.
    if let Err(refusal) = check_navigation(&r.url, Some(&r.page_text)) {
        return json(
            200,
            &PlanReply {
                refused: Some(refusal.reason().to_string()),
                ..PlanReply::default()
            },
        );
    }
    let answers = match (shared.backend.answers)().await {
        Ok(a) => a,
        Err(e) => return error(400, &e),
    };
    let fields: Vec<FormField> = r
        .fields
        .iter()
        .map(|f| FormField {
            id: f.id.clone(),
            label: f.label.clone(),
            input: input_of(f),
            required: f.required,
        })
        .collect();
    let plan = plan_fields(&fields, &answers);
    let steps = plan
        .steps
        .iter()
        .map(|s| match &s.action {
            Action::Fill(v) => Step {
                field_id: s.field_id.clone(),
                action: "fill".into(),
                value: v.clone(),
            },
            Action::Upload(path) => Step {
                field_id: s.field_id.clone(),
                action: "upload".into(),
                value: if answers.resume_pdf.as_deref() == Some(path.as_str()) {
                    "resume".into()
                } else {
                    "cover_letter".into()
                },
            },
        })
        .collect();
    let report = FillReport {
        ats: detect(&r.url),
        url: r.url.clone(),
        filled: plan.filled,
        skipped: plan.skipped,
    };
    let list = report.checklist();
    json(
        200,
        &PlanReply {
            refused: None,
            steps,
            checklist: list
                .items
                .into_iter()
                .map(|i| CheckLine {
                    label: i.label,
                    state: match i.state {
                        State::Filled => "filled",
                        State::CheckThis => "check_this",
                        State::NeedsYou => "needs_you",
                        State::LeftBlank => "left_blank",
                    }
                    .into(),
                    detail: i.detail,
                })
                .collect(),
            ready_to_submit: list.ready_to_submit,
        },
    )
}
