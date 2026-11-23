//! Mail in the database: accounts (settings only, secrets stay in the keychain) and messages.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde_json::Value;
use sqlx::Row;

fn db_err(e: impl std::fmt::Display) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: i64,
    /// `gmail`, `outlook` or `imap`.
    pub provider: String,
    pub address: String,
    pub config: Value,
    pub sync_state: Value,
    pub last_synced_at: Option<String>,
}

fn account_from(r: &sqlx::sqlite::SqliteRow) -> Result<Account> {
    let text = |c: &str| -> Result<Value> {
        let s: String = r.get(c);
        serde_json::from_str(&s).map_err(|e| Error::Database(format!("bad {c}: {e}")))
    };
    Ok(Account {
        id: r.get("id"),
        provider: r.get("provider"),
        address: r.get("address"),
        config: text("config")?,
        sync_state: text("sync_state")?,
        last_synced_at: r.get("last_synced_at"),
    })
}

/// Add an account, or update its settings when the address is already connected.
pub async fn add_account(
    db: &Db,
    provider: &str,
    address: &str,
    config: &Value,
    secret_ref: &str,
) -> Result<Account> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO mail_account(provider, address, config, secret_ref) VALUES (?, ?, ?, ?) \
         ON CONFLICT(address) DO UPDATE SET provider = excluded.provider, \
         config = excluded.config, secret_ref = excluded.secret_ref RETURNING id",
    )
    .bind(provider)
    .bind(address.trim().to_ascii_lowercase())
    .bind(config.to_string())
    .bind(secret_ref)
    .fetch_one(db.pool())
    .await
    .map_err(db_err)?;
    get_account(db, id).await
}

pub async fn get_account(db: &Db, id: i64) -> Result<Account> {
    let row = sqlx::query("SELECT * FROM mail_account WHERE id = ?")
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("mail account {id}")))?;
    account_from(&row)
}

pub async fn list_accounts(db: &Db) -> Result<Vec<Account>> {
    let rows = sqlx::query("SELECT * FROM mail_account ORDER BY id")
        .fetch_all(db.pool())
        .await
        .map_err(db_err)?;
    rows.iter().map(account_from).collect()
}

/// The keychain reference for an account's secret.
pub async fn secret_ref(db: &Db, id: i64) -> Result<Option<String>> {
    sqlx::query_scalar("SELECT secret_ref FROM mail_account WHERE id = ?")
        .bind(id)
        .fetch_one(db.pool())
        .await
        .map_err(db_err)
}

/// Remove an account and, through the foreign key, the mail it brought in.
pub async fn remove_account(db: &Db, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM mail_account WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn set_sync_state(db: &Db, id: i64, state: &Value, now: &str) -> Result<()> {
    sqlx::query("UPDATE mail_account SET sync_state = ?, last_synced_at = ? WHERE id = ?")
        .bind(state.to_string())
        .bind(now)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: i64,
    pub account_id: i64,
    pub message_id: String,
    pub thread_id: Option<String>,
    /// `in` or `out`.
    pub direction: String,
    /// `employer` or `alert`.
    pub kind: String,
    pub application_id: Option<i64>,
    pub from_name: Option<String>,
    pub from_addr: Option<String>,
    pub to_addr: Option<String>,
    pub subject: String,
    pub body: String,
    pub received_at: Option<String>,
    /// `interview`, `assessment`, `offer`, `rejection`, `question`, `acknowledgement`, `other`.
    pub label: Option<String>,
    pub label_with: Option<String>,
    pub summary: Option<String>,
    pub suggested_status: Option<String>,
    pub interview_at: Option<String>,
    pub needs_reply: bool,
    pub is_read: bool,
    pub draft_document_id: Option<i64>,
}

fn message_from(r: &sqlx::sqlite::SqliteRow) -> Message {
    Message {
        id: r.get("id"),
        account_id: r.get("account_id"),
        message_id: r.get("message_id"),
        thread_id: r.get("thread_id"),
        direction: r.get("direction"),
        kind: r.get("kind"),
        application_id: r.get("application_id"),
        from_name: r.get("from_name"),
        from_addr: r.get("from_addr"),
        to_addr: r.get("to_addr"),
        subject: r.get::<Option<String>, _>("subject").unwrap_or_default(),
        body: r.get::<Option<String>, _>("body").unwrap_or_default(),
        received_at: r.get("received_at"),
        label: r.get("label"),
        label_with: r.get("label_with"),
        summary: r.get("summary"),
        suggested_status: r.get("suggested_status"),
        interview_at: r.get("interview_at"),
        needs_reply: r.get::<i64, _>("needs_reply") != 0,
        is_read: r.get::<i64, _>("is_read") != 0,
        draft_document_id: r.get("draft_document_id"),
    }
}

#[derive(Debug, Clone)]
pub struct NewMessage<'a> {
    pub account_id: i64,
    pub message_id: &'a str,
    pub thread_id: Option<&'a str>,
    pub direction: &'a str,
    pub kind: &'a str,
    pub from_name: Option<&'a str>,
    pub from_addr: Option<&'a str>,
    pub to_addr: Option<&'a str>,
    pub subject: &'a str,
    pub body: &'a str,
    pub received_at: Option<&'a str>,
    pub refs: &'a [String],
}

/// Store a message. Returns `None` when the account already has it, so syncing twice is safe.
pub async fn insert_message(db: &Db, m: &NewMessage<'_>) -> Result<Option<i64>> {
    let id: Option<i64> = sqlx::query_scalar(
        "INSERT INTO email_message(account_id, message_id, thread_id, direction, kind, from_name, \
         from_addr, to_addr, subject, body, received_at, refs) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(account_id, message_id) DO NOTHING RETURNING id",
    )
    .bind(m.account_id)
    .bind(m.message_id)
    .bind(m.thread_id)
    .bind(m.direction)
    .bind(m.kind)
    .bind(m.from_name)
    .bind(m.from_addr)
    .bind(m.to_addr)
    .bind(m.subject)
    .bind(m.body)
    .bind(m.received_at)
    .bind(serde_json::to_string(m.refs).unwrap_or_else(|_| "[]".into()))
    .fetch_optional(db.pool())
    .await
    .map_err(db_err)?;
    Ok(id)
}

pub async fn get_message(db: &Db, id: i64) -> Result<Message> {
    let row = sqlx::query("SELECT * FROM email_message WHERE id = ?")
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(db_err)?
        .ok_or_else(|| Error::NotFound(format!("message {id}")))?;
    Ok(message_from(&row))
}

#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub application_id: Option<i64>,
    pub unread_only: bool,
    pub needs_reply_only: bool,
}

/// Employer mail, newest first. Job-alert mail is not shown here.
pub async fn list_messages(db: &Db, f: &Filter) -> Result<Vec<Message>> {
    let rows = sqlx::query(
        "SELECT * FROM email_message WHERE kind = 'employer' \
         AND (?1 IS NULL OR application_id = ?1) \
         AND (?2 = 0 OR is_read = 0) AND (?3 = 0 OR needs_reply = 1) \
         ORDER BY COALESCE(received_at, '') DESC, id DESC",
    )
    .bind(f.application_id)
    .bind(f.unread_only as i64)
    .bind(f.needs_reply_only as i64)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows.iter().map(message_from).collect())
}

pub async fn unread_count(db: &Db) -> Result<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM email_message WHERE kind = 'employer' AND direction = 'in' AND is_read = 0",
    )
    .fetch_one(db.pool())
    .await
    .map_err(db_err)
}

pub async fn mark_read(db: &Db, id: i64, read: bool) -> Result<()> {
    sqlx::query("UPDATE email_message SET is_read = ? WHERE id = ?")
        .bind(read as i64)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn set_application(db: &Db, id: i64, application_id: Option<i64>) -> Result<()> {
    sqlx::query("UPDATE email_message SET application_id = ? WHERE id = ?")
        .bind(application_id)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn set_draft(db: &Db, id: i64, document_id: i64) -> Result<()> {
    sqlx::query("UPDATE email_message SET draft_document_id = ? WHERE id = ?")
        .bind(document_id)
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(db_err)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelRow<'a> {
    pub label: &'a str,
    pub label_with: &'a str,
    pub summary: &'a str,
    pub suggested_status: Option<&'a str>,
    pub interview_at: Option<&'a str>,
    pub needs_reply: bool,
}

pub async fn save_label(db: &Db, id: i64, l: &LabelRow<'_>) -> Result<()> {
    sqlx::query(
        "UPDATE email_message SET label = ?, label_with = ?, summary = ?, suggested_status = ?, \
         interview_at = ?, needs_reply = ? WHERE id = ?",
    )
    .bind(l.label)
    .bind(l.label_with)
    .bind(l.summary)
    .bind(l.suggested_status)
    .bind(l.interview_at)
    .bind(l.needs_reply as i64)
    .bind(id)
    .execute(db.pool())
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Lowercase names and aliases of companies you have an open application with.
pub async fn open_company_names(db: &Db) -> Result<std::collections::HashSet<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT c.name, c.aliases FROM company c \
         JOIN application a ON a.company_id = c.id \
         WHERE a.status NOT IN ('accepted','rejected','declined','withdrawn')",
    )
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    let mut out = std::collections::HashSet::new();
    for r in rows {
        out.insert(r.get::<String, _>("name").to_ascii_lowercase());
        if let Ok(Value::Array(a)) = serde_json::from_str::<Value>(&r.get::<String, _>("aliases")) {
            out.extend(
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(str::to_ascii_lowercase),
            );
        }
    }
    Ok(out)
}

/// Domains of companies CeeVee tracks: their stored mail domains plus their website host.
pub async fn company_domains(db: &Db) -> Result<std::collections::HashSet<String>> {
    let rows = sqlx::query("SELECT website, email_domains FROM company")
        .fetch_all(db.pool())
        .await
        .map_err(db_err)?;
    let mut out = std::collections::HashSet::new();
    for r in rows {
        let list: String = r.get("email_domains");
        if let Ok(Value::Array(a)) = serde_json::from_str::<Value>(&list) {
            out.extend(
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(str::to_ascii_lowercase),
            );
        }
        if let Some(host) = r
            .get::<Option<String>, _>("website")
            .and_then(|w| host_of(&w))
        {
            out.insert(host);
        }
    }
    Ok(out)
}

/// Host of a URL or bare domain, without `www.`.
pub fn host_of(url: &str) -> Option<String> {
    let t = url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = t.split(['/', '?', '#']).next()?.trim().to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    (host.contains('.')).then_some(host)
}

/// Message IDs of mail the user sent or that is already tied to an application, so replies to
/// them are recognized as employer mail whatever domain they come from.
pub async fn sent_message_ids(db: &Db) -> Result<std::collections::HashSet<String>> {
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT message_id FROM email_message WHERE direction = 'out' OR application_id IS NOT NULL")
            .fetch_all(db.pool())
            .await
            .map_err(db_err)?;
    Ok(ids.into_iter().collect())
}

/// `Company, Job title` for every application, for lists and pickers.
pub async fn application_names(db: &Db) -> Result<std::collections::HashMap<i64, String>> {
    let rows = sqlx::query(
        "SELECT a.id, c.name AS company, p.title FROM application a \
         LEFT JOIN company c ON c.id = a.company_id LEFT JOIN posting p ON p.id = a.posting_id",
    )
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows
        .iter()
        .map(|r| {
            let c: Option<String> = r.get("company");
            let t: Option<String> = r.get("title");
            let name = [c, t].into_iter().flatten().collect::<Vec<_>>().join(", ");
            (
                r.get::<i64, _>("id"),
                if name.is_empty() {
                    "Application".into()
                } else {
                    name
                },
            )
        })
        .collect())
}
