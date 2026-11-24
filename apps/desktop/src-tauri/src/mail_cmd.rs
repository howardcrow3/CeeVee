//! Email tracking: connect an account, sync it, read the Inbox, link mail to applications and
//! draft replies. The work happens in `ceevee-mail`; this file is the command surface and the
//! 10-minute background sync.

use std::sync::Arc;

use ceevee_agent::notifications::Kind;
use ceevee_core::{Event, EventBus};
use ceevee_db::Db;
use ceevee_mail::imap::ImapConfig;
use ceevee_mail::secrets::{account_key, Keychain, SecretStore};
use ceevee_mail::store;
use chrono::Utc;
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::license_cmd::{require_write, Service};
use crate::models::{worker_for, Engine};
use crate::notifications::notify;

#[derive(Debug, Clone, Serialize, Type)]
pub struct MailAccountLine {
    pub id: u32,
    /// `imap` or `gmail`.
    pub provider: String,
    pub address: String,
    pub last_synced_at: Option<String>,
}

fn account_line(a: store::Account) -> MailAccountLine {
    MailAccountLine {
        id: a.id as u32,
        provider: a.provider,
        address: a.address,
        last_synced_at: a.last_synced_at,
    }
}

fn e(err: impl std::fmt::Display) -> String {
    err.to_string()
}

/// Server settings CeeVee knows for an address, so the form can fill them in.
#[derive(Debug, Clone, Serialize, Type)]
pub struct ImapPreset {
    pub host: String,
    pub port: u16,
}

#[tauri::command]
#[specta::specta]
pub fn imap_preset(address: String) -> Option<ImapPreset> {
    ceevee_mail::imap::preset_for(&address).map(|(host, port)| ImapPreset {
        host: host.into(),
        port,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn list_mail_accounts(db: tauri::State<'_, Db>) -> Result<Vec<MailAccountLine>, String> {
    Ok(store::list_accounts(&db)
        .await
        .map_err(e)?
        .into_iter()
        .map(account_line)
        .collect())
}

/// Check the password with the mail server, then save the account. The password goes to the
/// operating system's keychain, never to the database.
#[tauri::command]
#[specta::specta]
pub async fn connect_imap(
    lic: tauri::State<'_, Arc<Service>>,
    db: tauri::State<'_, Db>,
    address: String,
    host: String,
    port: u16,
    password: String,
) -> Result<MailAccountLine, String> {
    require_write(&lic).await?;
    let address = address.trim().to_ascii_lowercase();
    if !address.contains('@') {
        return Err("Enter your full email address.".into());
    }
    if host.trim().is_empty() || password.is_empty() {
        return Err("Enter the mail server and the app password.".into());
    }
    let cfg = ImapConfig {
        host: host.trim().into(),
        port,
        username: address.clone(),
        tls: true,
    };
    ceevee_mail::imap::test_login(&cfg, &password)
        .await
        .map_err(|err| ceevee_mail::imap::explain(&err))?;
    let key = account_key(&address);
    Keychain.set(&key, &password)?;
    let cfg = serde_json::to_value(&cfg).map_err(e)?;
    let account = store::add_account(&db, "imap", &address, &cfg, &key)
        .await
        .map_err(e)?;
    Ok(account_line(account))
}

/// Disconnect an account and remove the mail CeeVee kept from it. For a Google account this also
/// tells Google to revoke CeeVee's access; if Google cannot be reached the account is still
/// removed here and the person is told to remove access at myaccount.google.com/permissions.
#[tauri::command]
#[specta::specta]
pub async fn disconnect_mail_account(db: tauri::State<'_, Db>, id: u32) -> Result<(), String> {
    let provider = store::get_account(&db, i64::from(id))
        .await
        .map_err(e)?
        .provider;
    let token = ceevee_mail::google_account::forget_account(&db, &Keychain, i64::from(id)).await?;
    if provider == "gmail" {
        if let (Some(token), Some(client)) = (token, google_client()) {
            if let Err(err) = client.revoke(&token).await {
                tracing::warn!("could not revoke Google access: {err}");
                return Err("Signed out of CeeVee, but Google could not be reached to remove its access. You can remove it at myaccount.google.com/permissions.".into());
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, Type)]
pub struct SyncLine {
    pub employer: u32,
    pub status_changes: u32,
    pub waiting_for_you: u32,
    pub drafts: u32,
    /// Messages looked at on the server, whether or not they were about a job.
    pub seen: u32,
}

/// Sync every connected account now.
pub async fn sync_all(app: &AppHandle, db: &Db) -> Result<SyncLine, String> {
    let mut total = SyncLine::default();
    for account in store::list_accounts(db).await.map_err(e)? {
        let key = store::secret_ref(db, account.id)
            .await
            .map_err(e)?
            .unwrap_or_else(|| account_key(&account.address));
        let Some(secret) = Keychain.get(&key) else {
            return Err(format!(
                "The saved sign-in for {} is missing from the keychain. Connect the account again.",
                account.address
            ));
        };
        let engine = app.state::<Arc<Engine>>().inner().clone();
        let handle = app.clone();
        let worker = move || worker_for(&handle, &engine).ok();
        let now = Utc::now();
        let iso = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let s = match account.provider.as_str() {
            "gmail" => {
                let client = google_client().ok_or(NOT_SET_UP)?;
                let token = client
                    .refresh(&secret)
                    .await
                    .map_err(|err| format!("{err}. Connect Google again in Settings."))?;
                let api = ceevee_google::Gmail::new(&token);
                ceevee_mail::sync::sync_gmail(db, account.id, &api, worker, now.timestamp(), &iso)
                    .await
                    .map_err(|err| err.to_string())?
            }
            _ => {
                ceevee_mail::sync::sync_imap(db, account.id, &secret, worker, now.timestamp(), &iso)
                    .await
                    .map_err(|err| ceevee_mail::imap::explain(&err))?
            }
        };
        total.employer += s.employer as u32;
        total.status_changes += s.status_changes as u32;
        total.waiting_for_you += s.waiting_for_you as u32;
        total.drafts += s.drafts as u32;
        total.seen += s.seen as u32;
        tracing::info!(
            "mail check finished for a {} account: looked at {}, employer mail {}, status changes {}",
            account.provider,
            s.seen,
            s.employer,
            s.status_changes
        );
    }
    Ok(total)
}

#[tauri::command]
#[specta::specta]
pub async fn sync_mail(
    lic: tauri::State<'_, Arc<Service>>,
    app: AppHandle,
    db: tauri::State<'_, Db>,
) -> Result<SyncLine, String> {
    require_write(&lic).await?;
    let s = sync_all(&app, &db).await?;
    notify_new(&app, &db, &s).await;
    Ok(s)
}

async fn notify_new(app: &AppHandle, db: &Db, s: &SyncLine) {
    if s.employer > 0 {
        let body = if s.employer == 1 {
            "An employer wrote to you. Open the Inbox to read it.".to_string()
        } else {
            format!("{} employer messages are in your Inbox.", s.employer)
        };
        notify(app, db, Kind::EmployerReplied, "New employer mail", &body).await;
    }
}

/// Runs a sync when the scheduler says `mail.sync` is due.
pub fn spawn(app: AppHandle, db: Db, bus: EventBus) {
    let mut rx = bus.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(Event::TaskDue { task }) if task == "mail.sync" => {
                    if let Some(lic) = app.try_state::<Arc<Service>>() {
                        if require_write(&lic).await.is_err() {
                            continue;
                        }
                    }
                    match sync_all(&app, &db).await {
                        Ok(s) => {
                            notify_new(&app, &db, &s).await;
                            if s.employer > 0 {
                                bus.publish(Event::SettingChanged {
                                    key: "mail.last_sync".into(),
                                });
                            }
                        }
                        Err(err) => tracing::warn!("mail sync failed: {err}"),
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct InboxLine {
    pub id: u32,
    pub from: String,
    pub subject: String,
    pub summary: String,
    pub received_at: Option<String>,
    /// `interview`, `assessment`, `offer`, `rejection`, `question`, `acknowledgement`, `other`.
    pub label: Option<String>,
    pub needs_reply: bool,
    pub unread: bool,
    pub has_draft: bool,
    pub application_id: Option<u32>,
    /// `Company, Job title` of the linked application.
    pub application: Option<String>,
}

async fn application_names(db: &Db) -> Result<std::collections::HashMap<i64, String>, String> {
    store::application_names(db).await.map_err(e)
}

fn line(m: &store::Message, names: &std::collections::HashMap<i64, String>) -> InboxLine {
    InboxLine {
        id: m.id as u32,
        from: m
            .from_name
            .clone()
            .or_else(|| m.from_addr.clone())
            .unwrap_or_default(),
        subject: m.subject.clone(),
        summary: m.summary.clone().unwrap_or_default(),
        received_at: m.received_at.clone(),
        label: m.label.clone(),
        needs_reply: m.needs_reply,
        unread: !m.is_read,
        has_draft: m.draft_document_id.is_some(),
        application_id: m.application_id.map(|a| a as u32),
        application: m.application_id.and_then(|a| names.get(&a).cloned()),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn list_inbox(db: tauri::State<'_, Db>) -> Result<Vec<InboxLine>, String> {
    let names = application_names(&db).await?;
    let all = store::list_messages(&db, &Default::default())
        .await
        .map_err(e)?;
    Ok(all.iter().map(|m| line(m, &names)).collect())
}

#[tauri::command]
#[specta::specta]
pub async fn unread_mail_count(db: tauri::State<'_, Db>) -> Result<u32, String> {
    Ok(store::unread_count(&db).await.map_err(e)? as u32)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct MessageView {
    pub line: InboxLine,
    pub from_address: String,
    pub body: String,
    pub interview_at: Option<String>,
    pub suggested_status: Option<String>,
    /// Document id of the drafted reply, if there is one.
    pub draft_id: Option<u32>,
    /// What the draft says now.
    pub draft: Option<String>,
}

/// Open one message; this marks it read.
#[tauri::command]
#[specta::specta]
pub async fn open_message(db: tauri::State<'_, Db>, id: u32) -> Result<MessageView, String> {
    let m = store::get_message(&db, i64::from(id)).await.map_err(e)?;
    store::mark_read(&db, m.id, true).await.map_err(e)?;
    let names = application_names(&db).await?;
    let draft = match m.draft_document_id {
        Some(d) => Some(
            ceevee_docs::store::current(&db, d)
                .await
                .map_err(e)?
                .content,
        ),
        None => None,
    };
    let mut l = line(&m, &names);
    l.unread = false;
    Ok(MessageView {
        line: l,
        from_address: m.from_addr.unwrap_or_default(),
        body: m.body,
        interview_at: m.interview_at,
        suggested_status: m.suggested_status,
        draft_id: m.draft_document_id.map(|d| d as u32),
        draft,
    })
}

/// Tie a message to an application the user chose (or `None` to untie it).
#[tauri::command]
#[specta::specta]
pub async fn link_message(
    db: tauri::State<'_, Db>,
    id: u32,
    application_id: Option<u32>,
) -> Result<(), String> {
    store::set_application(&db, i64::from(id), application_id.map(i64::from))
        .await
        .map_err(e)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ReplyResult {
    pub document_id: u32,
    /// Names, numbers or acronyms in the draft that the profile, posting and email do not contain.
    pub unverified: Vec<String>,
}

/// Write (or rewrite) a reply draft. Changing it afterwards goes through the document chat.
#[tauri::command]
#[specta::specta]
pub async fn draft_mail_reply(
    lic: tauri::State<'_, Arc<Service>>,
    app: AppHandle,
    db: tauri::State<'_, Db>,
    engine: tauri::State<'_, Arc<Engine>>,
    id: u32,
    instructions: String,
    again: bool,
) -> Result<ReplyResult, String> {
    require_write(&lic).await?;
    let worker = worker_for(&app, &engine)?;
    let r = ceevee_mail::reply::draft_reply(&db, &worker, i64::from(id), &instructions, again)
        .await
        .map_err(e)?;
    Ok(ReplyResult {
        document_id: r.document_id as u32,
        unverified: r.flags.into_iter().map(|f| f.text).collect(),
    })
}

/// Applications the user can pick from when linking a message.
#[derive(Debug, Clone, Serialize, Type)]
pub struct ApplicationChoice {
    pub id: u32,
    pub name: String,
}

#[tauri::command]
#[specta::specta]
pub async fn list_application_choices(
    db: tauri::State<'_, Db>,
) -> Result<Vec<ApplicationChoice>, String> {
    let mut v: Vec<ApplicationChoice> = application_names(&db)
        .await?
        .into_iter()
        .map(|(id, name)| ApplicationChoice {
            id: id as u32,
            name,
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(v)
}

const NOT_SET_UP: &str =
    "Google sign-in is not set up in this build of CeeVee. Use an app password instead.";

/// CeeVee's Google OAuth client, set when the release is built (or in the environment while
/// developing). Without it the Google button stays hidden.
fn google_client() -> Option<ceevee_google::OAuthClient> {
    let id = option_env!("CEEVEE_GOOGLE_CLIENT_ID")
        .map(str::to_string)
        .or_else(|| std::env::var("CEEVEE_GOOGLE_CLIENT_ID").ok())
        .filter(|v| !v.is_empty())?;
    let secret = option_env!("CEEVEE_GOOGLE_CLIENT_SECRET")
        .map(str::to_string)
        .or_else(|| std::env::var("CEEVEE_GOOGLE_CLIENT_SECRET").ok())
        .filter(|v| !v.is_empty());
    Some(ceevee_google::OAuthClient::google(&id, secret.as_deref()))
}

/// Whether Google sign-in is available in this build.
#[tauri::command]
#[specta::specta]
pub fn google_available() -> bool {
    google_client().is_some()
}

/// The sign-in that is waiting for the browser, if any, so it can be cancelled or replaced.
#[derive(Default)]
pub struct GoogleSignIn(std::sync::Mutex<Option<Arc<tokio::sync::Notify>>>);

/// Stop a Google sign-in that is still waiting for the browser (for example after the person closed
/// the tab). Does nothing when none is running.
#[tauri::command]
#[specta::specta]
pub fn cancel_google_sign_in(pending: tauri::State<'_, GoogleSignIn>) {
    if let Some(n) = pending.0.lock().ok().and_then(|mut p| p.take()) {
        n.notify_one();
    }
}

/// Sign in with Google in the browser, then save the account. CeeVee asks for read-only access to
/// Gmail; the refresh token goes to the keychain.
#[tauri::command]
#[specta::specta]
pub async fn connect_google(
    lic: tauri::State<'_, Arc<Service>>,
    app: AppHandle,
    db: tauri::State<'_, Db>,
    pending: tauri::State<'_, GoogleSignIn>,
) -> Result<MailAccountLine, String> {
    use tauri_plugin_opener::OpenerExt;
    require_write(&lic).await?;
    let client = google_client().ok_or(NOT_SET_UP)?;
    let mut scopes = ceevee_google::oauth::SIGN_IN_SCOPES.to_vec();
    scopes.push(ceevee_google::oauth::GMAIL_READONLY);
    let opener = app.clone();
    // A new attempt replaces a stuck one: the old wait ends and its port is freed.
    let cancel = Arc::new(tokio::sync::Notify::new());
    if let Some(old) = pending
        .0
        .lock()
        .ok()
        .and_then(|mut p| p.replace(cancel.clone()))
    {
        old.notify_one();
    }
    let saver = db.inner().clone();
    // The browser only says "connected" once the account is saved, so a failed keychain write
    // shows up in the browser and in Settings instead of a false success.
    let result = client
        .sign_in_then(
            &scopes,
            move |url| {
                if let Err(err) = opener.opener().open_url(url, None::<&str>) {
                    tracing::warn!("could not open the browser: {err}");
                }
            },
            &cancel,
            ceevee_google::oauth::SIGN_IN_WAIT,
            move |tokens| async move {
                let address = match tokens.email.clone() {
                    Some(a) => a,
                    None => ceevee_google::Gmail::new(&tokens.access_token)
                        .address()
                        .await
                        .map_err(|err| err.to_string())?,
                };
                ceevee_mail::google_account::save_google_account(
                    &saver, &Keychain, &tokens, &address,
                )
                .await
            },
        )
        .await;
    if let Ok(mut p) = pending.0.lock() {
        if p.as_ref().is_some_and(|c| Arc::ptr_eq(c, &cancel)) {
            *p = None;
        }
    }
    let account = result.map_err(|err| err.to_string())?;
    Ok(account_line(account))
}
