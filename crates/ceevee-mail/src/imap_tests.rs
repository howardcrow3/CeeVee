//! IMAP against a tiny in-process server that speaks just enough of the protocol.

use std::sync::{Arc, Mutex};

use ceevee_db::Db;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::imap::{imap_date, ImapConfig, SyncState};
use crate::{store, sync};

struct Server {
    port: u16,
    /// Every command line the client sent.
    log: Arc<Mutex<Vec<String>>>,
}

fn mail(from: &str, subject: &str, body: &str, id: &str) -> Vec<u8> {
    format!(
        "From: Dana <{from}>\r\nTo: me@example.org\r\nSubject: {subject}\r\nMessage-ID: <{id}>\r\n\
         Date: Fri, 13 Nov 2026 10:00:00 +0000\r\n\r\n{body}\r\n"
    )
    .into_bytes()
}

fn split(raw: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let at = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    (raw[..at].to_vec(), raw.to_vec())
}

async fn serve(messages: Vec<(u32, Vec<u8>)>, password: &'static str) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(vec![]));
    let log2 = log.clone();
    tokio::spawn(async move {
        loop {
            let Ok((sock, _)) = listener.accept().await else {
                return;
            };
            let messages = messages.clone();
            let log = log2.clone();
            tokio::spawn(async move {
                let (r, mut w) = sock.into_split();
                let mut lines = BufReader::new(r).lines();
                w.write_all(b"* OK ready\r\n").await.unwrap();
                while let Ok(Some(line)) = lines.next_line().await {
                    log.lock().unwrap().push(line.clone());
                    let (tag, cmd) = line.split_once(' ').unwrap();
                    let up = cmd.to_ascii_uppercase();
                    if up.starts_with("LOGIN") {
                        let ok = cmd.contains(password);
                        let reply = if ok {
                            format!("{tag} OK done\r\n")
                        } else {
                            format!("{tag} NO [AUTHENTICATIONFAILED] Invalid credentials\r\n")
                        };
                        w.write_all(reply.as_bytes()).await.unwrap();
                    } else if up.starts_with("SELECT") {
                        let next = messages.last().map_or(1, |m| m.0 + 1);
                        w.write_all(
                            format!("* {} EXISTS\r\n* OK [UIDVALIDITY 7] ok\r\n* OK [UIDNEXT {next}] ok\r\n{tag} OK [READ-WRITE] done\r\n", messages.len()).as_bytes(),
                        )
                        .await
                        .unwrap();
                    } else if up.starts_with("UID SEARCH") {
                        // "UID n:*" returns at least the last message, like a real server.
                        let from = up
                            .strip_prefix("UID SEARCH UID ")
                            .and_then(|r| r.split(':').next())
                            .and_then(|n| n.parse::<u32>().ok());
                        let ids: Vec<String> = messages
                            .iter()
                            .filter(|m| {
                                from.is_none_or(|f| m.0 >= f)
                                    || Some(m.0) == messages.last().map(|l| l.0)
                            })
                            .map(|m| m.0.to_string())
                            .collect();
                        w.write_all(
                            format!("* SEARCH {}\r\n{tag} OK done\r\n", ids.join(" ")).as_bytes(),
                        )
                        .await
                        .unwrap();
                    } else if up.starts_with("UID FETCH") {
                        let set = cmd.split_whitespace().nth(2).unwrap();
                        let want_body = up.contains("BODY.PEEK[]");
                        for id in set.split(',').map(|n| n.parse::<u32>().unwrap()) {
                            let Some((_, raw)) = messages.iter().find(|m| m.0 == id) else {
                                continue;
                            };
                            let (head, full) = split(raw);
                            let (name, data) = if want_body {
                                ("BODY[]", full)
                            } else {
                                ("BODY[HEADER]", head)
                            };
                            w.write_all(
                                format!("* {id} FETCH (UID {id} {name} {{{}}}\r\n", data.len())
                                    .as_bytes(),
                            )
                            .await
                            .unwrap();
                            w.write_all(&data).await.unwrap();
                            w.write_all(b")\r\n").await.unwrap();
                        }
                        w.write_all(format!("{tag} OK done\r\n").as_bytes())
                            .await
                            .unwrap();
                    } else if up.starts_with("LOGOUT") {
                        w.write_all(format!("* BYE\r\n{tag} OK done\r\n").as_bytes())
                            .await
                            .unwrap();
                        return;
                    } else {
                        w.write_all(format!("{tag} OK done\r\n").as_bytes())
                            .await
                            .unwrap();
                    }
                }
            });
        }
    });
    Server { port, log }
}

fn config(port: u16) -> serde_json::Value {
    serde_json::to_value(ImapConfig {
        host: "127.0.0.1".into(),
        port,
        username: "me@example.org".into(),
        tls: false,
    })
    .unwrap()
}

#[test]
fn dates_for_the_first_sync() {
    assert_eq!(imap_date(1_794_564_000, 0), "13-Nov-2026");
    assert_eq!(imap_date(1_794_564_000, 90), "15-Aug-2026");
}

#[tokio::test]
async fn a_sync_downloads_only_employer_mail_and_does_not_repeat() {
    let server = serve(
        vec![
            (
                1,
                mail(
                    "dana@acme.com",
                    "Interview",
                    "Can we schedule an interview Tuesday?",
                    "i1@acme",
                ),
            ),
            (
                2,
                mail("friend@gmail.com", "Dinner", "Free Friday?", "f1@x"),
            ),
            (
                3,
                mail(
                    "noreply@greenhouse.io",
                    "Thanks",
                    "Thank you for applying.",
                    "g1@gh",
                ),
            ),
        ],
        "app-password",
    )
    .await;
    let db = Db::open_in_memory().await.unwrap();
    let acct = store::add_account(&db, "imap", "me@example.org", &config(server.port), "k")
        .await
        .unwrap();
    sqlx::query("INSERT INTO company(name, email_domains) VALUES ('Acme', '[\"acme.com\"]')")
        .execute(db.pool())
        .await
        .unwrap();

    let s = sync::sync_imap(
        &db,
        acct.id,
        "app-password",
        || None,
        1_794_564_000,
        "2026-11-13T10:00:00Z",
    )
    .await
    .unwrap();
    assert_eq!(s.seen, 3);
    assert_eq!(s.employer, 2);
    assert_eq!(
        store::list_messages(&db, &Default::default())
            .await
            .unwrap()
            .len(),
        2
    );

    // The friend's body was never requested: only the header fetch named uid 2.
    let log = server.log.lock().unwrap().clone();
    let body_fetches: Vec<&String> = log.iter().filter(|l| l.contains("BODY.PEEK[]")).collect();
    assert_eq!(body_fetches.len(), 1);
    assert!(body_fetches[0].contains("1,3"), "{body_fetches:?}");
    assert!(log
        .iter()
        .all(|l| !l.to_ascii_uppercase().contains("STORE")));

    // Second sync starts after the last uid and finds nothing new.
    let state = store::get_account(&db, acct.id).await.unwrap().sync_state;
    assert_eq!(
        serde_json::from_value::<SyncState>(state).unwrap().last_uid,
        Some(3)
    );
    let s2 = sync::sync_imap(
        &db,
        acct.id,
        "app-password",
        || None,
        1_794_564_100,
        "2026-11-13T10:01:40Z",
    )
    .await
    .unwrap();
    assert_eq!(s2.employer, 0);
    assert_eq!(
        store::list_messages(&db, &Default::default())
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn a_wrong_password_gives_a_plain_message() {
    let server = serve(vec![], "right").await;
    let cfg: ImapConfig = serde_json::from_value(config(server.port)).unwrap();
    let err = crate::imap::test_login(&cfg, "wrong").await.unwrap_err();
    assert!(crate::imap::explain(&err).contains("app password"), "{err}");
    crate::imap::test_login(&cfg, "right").await.unwrap();
}

#[tokio::test]
async fn an_unreachable_server_is_explained() {
    let cfg = ImapConfig {
        host: "127.0.0.1".into(),
        port: 1,
        username: "a".into(),
        tls: false,
    };
    let err = crate::imap::test_login(&cfg, "x").await.unwrap_err();
    assert!(
        crate::imap::explain(&err).contains("could not reach"),
        "{err}"
    );
}

#[test]
fn presets_cover_the_common_providers() {
    use crate::imap::preset_for;
    assert_eq!(preset_for("a@gmail.com"), Some(("imap.gmail.com", 993)));
    assert_eq!(
        preset_for("a@Outlook.com"),
        Some(("outlook.office365.com", 993))
    );
    assert_eq!(preset_for("a@example.org"), None);
}
