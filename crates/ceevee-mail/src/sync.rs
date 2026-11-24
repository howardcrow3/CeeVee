//! One sync of one account: fetch, then run every candidate through the pipeline.

use ceevee_ai::ModelWorker;
use ceevee_core::{Error, Result};
use ceevee_db::Db;

use crate::filter::classify;
use crate::imap::{fetch_new, ImapConfig, SyncState};
use crate::pipeline::{process, Context, Processed};
use crate::status::Outcome;
use crate::store;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Messages looked at on the server.
    pub seen: usize,
    /// Employer messages stored.
    pub employer: usize,
    pub status_changes: usize,
    /// Jobs started from an email, waiting for the user to confirm.
    pub new_jobs: usize,
    pub waiting_for_you: usize,
    pub drafts: usize,
}

impl Summary {
    pub fn add(&mut self, p: &Processed) {
        if p.kind.is_some() {
            self.employer += 1;
        }
        match p.status {
            Some(Outcome::Applied(_)) => self.status_changes += 1,
            Some(Outcome::Proposed(_)) => self.waiting_for_you += 1,
            _ => {}
        }
        if p.created {
            self.new_jobs += 1;
        }
        if p.drafted {
            self.drafts += 1;
        }
    }
}

/// Every message not sent by the account itself is read: whether it is about a job search is
/// decided from its text, not from who sent it (see `detect`).
fn not_from(m: &crate::parse::ParsedMail, own: &str) -> bool {
    m.from_address.as_deref() != Some(own)
}

/// Sync an IMAP account. `now_secs` is Unix time; `now` the same instant as ISO-8601. `worker` is
/// asked for only when there is mail to read, so a quiet inbox never wakes the model.
pub async fn sync_imap(
    db: &Db,
    account_id: i64,
    password: &str,
    worker: impl FnOnce() -> Option<ModelWorker>,
    now_secs: i64,
    now: &str,
) -> Result<Summary> {
    let account = store::get_account(db, account_id).await?;
    let cfg: ImapConfig = serde_json::from_value(account.config.clone())
        .map_err(|e| Error::Invalid(format!("account settings are not readable: {e}")))?;
    let state: SyncState = serde_json::from_value(account.sync_state.clone()).unwrap_or_default();
    let mut ctx = Context::load(db).await?;
    let rules = std::mem::take(&mut ctx.rules);
    let own = account.address.to_ascii_lowercase();
    let fetched = fetch_new(&cfg, password, &state, now_secs, |m| {
        classify(m, &rules).is_some() || not_from(m, &own)
    })
    .await?;
    ctx.rules = rules;
    let mut summary = Summary {
        seen: fetched.seen,
        ..Default::default()
    };
    let worker = if fetched.messages.is_empty() {
        None
    } else {
        worker()
    };
    for raw in &fetched.messages {
        if let Some(p) = process(db, &mut ctx, worker.as_ref(), account_id, raw, now).await? {
            summary.add(&p);
        }
    }
    let state = serde_json::to_value(&fetched.state).unwrap_or_default();
    store::set_sync_state(db, account_id, &state, now).await?;
    Ok(summary)
}

/// Where the last Gmail sync stopped.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GmailState {
    /// Unix seconds of the start of the last sync.
    pub after: Option<i64>,
}

/// Sync a Gmail account through the Gmail API. Headers are read first, and a whole message is
/// downloaded only when the candidate filter wants it. A day of overlap covers mail that arrives
/// late; storing a message twice is harmless.
pub async fn sync_gmail<A: ceevee_google::MailApi>(
    db: &Db,
    account_id: i64,
    api: &A,
    worker: impl FnOnce() -> Option<ModelWorker>,
    now_secs: i64,
    now: &str,
) -> Result<Summary> {
    use crate::imap::MAX_PER_SYNC;
    let account = store::get_account(db, account_id).await?;
    let state: GmailState = serde_json::from_value(account.sync_state.clone()).unwrap_or_default();
    // Promotions, social and forum tabs are bulk mail; everything else may be a recruiter.
    const SCOPE: &str = "in:inbox -category:promotions -category:social -category:forums";
    let query = match state.after {
        Some(t) => format!("{SCOPE} after:{}", t - 86_400),
        None => format!("{SCOPE} newer_than:{}d", crate::imap::FIRST_SYNC_DAYS),
    };
    let own = account.address.to_ascii_lowercase();
    let ids = api.list_ids(&query, MAX_PER_SYNC).await?;
    let mut ctx = Context::load(db).await?;
    tracing::info!(
        "gmail sync: query \"{query}\" returned {} messages; {} company domains, {} company names, {} sent ids",
        ids.len(),
        ctx.rules.company_domains.len(),
        ctx.rules.company_names.len(),
        ctx.rules.sent_message_ids.len()
    );
    let mut summary = Summary {
        seen: ids.len(),
        ..Default::default()
    };
    let mut raws = vec![];
    for id in &ids {
        let head = api.headers(id).await?;
        let Ok(m) = crate::parse::parse(&head) else {
            tracing::warn!("gmail sync: could not read the headers of a message");
            continue;
        };
        // Mail already stored, or already set aside as not about a job, is not fetched again.
        if let Some(mid) = &m.message_id {
            if store::is_seen(db, account_id, mid).await? {
                continue;
            }
        }
        let candidate = classify(&m, &ctx.rules);
        tracing::info!(
            "gmail sync: message from {} -> {:?}",
            m.from_domain().unwrap_or_else(|| "unknown domain".into()),
            candidate
        );
        if candidate.is_some() || not_from(&m, &own) {
            raws.push(api.raw(id).await?);
        }
    }
    let worker = if raws.is_empty() { None } else { worker() };
    // Oldest first, so a thread's first message is stored before its replies are linked.
    for raw in raws.iter().rev() {
        if let Some(p) = process(db, &mut ctx, worker.as_ref(), account_id, raw, now).await? {
            summary.add(&p);
        }
    }
    tracing::info!(
        "gmail sync done: seen {}, employer {}, status changes {}, waiting {}, drafts {}",
        summary.seen,
        summary.employer,
        summary.status_changes,
        summary.waiting_for_you,
        summary.drafts
    );
    let state = serde_json::to_value(GmailState {
        after: Some(now_secs),
    })
    .unwrap_or_default();
    store::set_sync_state(db, account_id, &state, now).await?;
    Ok(summary)
}
