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
        if p.drafted {
            self.drafts += 1;
        }
    }
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
    let fetched = fetch_new(&cfg, password, &state, now_secs, |m| {
        classify(m, &rules).is_some()
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
