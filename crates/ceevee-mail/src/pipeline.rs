//! What happens to a message after it is fetched: filter, store, label, link, change status,
//! draft a reply. Everything here works from raw bytes, so IMAP and Gmail feed the same path.

use ceevee_ai::ModelWorker;
use ceevee_core::Result;
use ceevee_db::Db;
use ceevee_search::tracking::{self, Status};

use crate::filter::{classify, Candidate, Rules};
use crate::label::{label, parse_when, Kind};
use crate::parse::parse;
use crate::status::{self, Outcome};
use crate::store::{self, LabelRow, NewMessage};
use crate::{create, detect, link, reply};

/// What one message turned into.
#[derive(Debug, Clone, PartialEq)]
pub struct Processed {
    pub message_id: i64,
    pub kind: Option<String>,
    pub application_id: Option<i64>,
    pub status: Option<Outcome>,
    pub drafted: bool,
    /// This message started a job that now waits for the user to confirm.
    pub created: bool,
}

/// The mailbox context a run needs, loaded once per sync rather than per message.
pub struct Context {
    pub rules: Rules,
}

impl Context {
    pub async fn load(db: &Db) -> Result<Self> {
        Ok(Self {
            rules: Rules {
                company_domains: store::company_domains(db).await?,
                sent_message_ids: store::sent_message_ids(db).await?,
                company_names: store::open_company_names(db).await?,
            },
        })
    }
}

/// Handle one fetched message. Returns `None` for mail that is not about a job search, or that
/// was already stored. `worker` is the local model when one is running.
pub async fn process(
    db: &Db,
    ctx: &mut Context,
    worker: Option<&ModelWorker>,
    account_id: i64,
    raw: &[u8],
    now: &str,
) -> Result<Option<Processed>> {
    let mail = parse(raw)?;
    let message_id = mail
        .message_id
        .clone()
        .unwrap_or_else(|| format!("no-id-{}-{}", mail.sent_at.unwrap_or(0), mail.subject));
    // Mail from someone CeeVee knows is taken as it is. Anything else is read for signs of a
    // job search (a recruiter writing from a personal address, a company never added), and set
    // aside, ID only, when it is not one.
    let mut early = None;
    let candidate = match classify(&mail, &ctx.rules) {
        Some(c) => c,
        None => {
            if store::is_seen(db, account_id, &message_id).await? {
                tracing::info!("mail: already handled, skipped");
                return Ok(None);
            }
            let verdict = detect::verdict(&mail);
            tracing::info!(
                "mail from {}: not from a known sender; job-search score {} ({:?}), model {}",
                mail.from_domain()
                    .unwrap_or_else(|| "unknown domain".into()),
                detect::score(&mail),
                verdict,
                if worker.is_some() {
                    "available"
                } else {
                    "not loaded"
                }
            );
            if verdict == detect::Verdict::No {
                tracing::info!("mail: set aside, no sign of a job search");
                store::mark_seen(db, account_id, &message_id).await?;
                return Ok(None);
            }
            // A thin hint needs the model to settle it; the rules alone do not guess.
            // It is not set aside for good: with a model, a later check can settle it.
            if verdict == detect::Verdict::Maybe && worker.is_none() {
                tracing::info!("mail: unclear and no model loaded, left for a later check");
                return Ok(None);
            }
            let lab = label(worker, &mail).await;
            if lab.kind == Kind::NotJob
                || (lab.kind == Kind::Other && verdict == detect::Verdict::Maybe)
            {
                tracing::info!("mail: set aside, the model says {} ", lab.kind.as_str());
                store::mark_seen(db, account_id, &message_id).await?;
                return Ok(None);
            }
            early = Some(lab);
            Candidate::Employer
        }
    };
    let received = mail.sent_at.map(unix_to_iso);
    let thread = mail
        .references
        .first()
        .cloned()
        .unwrap_or_else(|| message_id.clone());
    let kind = match candidate {
        Candidate::Employer => "employer",
        Candidate::Alert => "alert",
    };
    let Some(id) = store::insert_message(
        db,
        &NewMessage {
            account_id,
            message_id: &message_id,
            thread_id: Some(&thread),
            direction: "in",
            kind,
            from_name: mail.from_name.as_deref(),
            from_addr: mail.from_address.as_deref(),
            to_addr: None,
            subject: &mail.subject,
            body: &mail.body,
            received_at: received.as_deref(),
            refs: &mail.references,
        },
    )
    .await?
    else {
        tracing::info!("mail: already stored, skipped");
        return Ok(None);
    };
    if candidate == Candidate::Alert {
        // Alert emails go to the job-alert parsers, which are a separate step.
        return Ok(Some(Processed {
            message_id: id,
            kind: None,
            application_id: None,
            status: None,
            drafted: false,
            created: false,
        }));
    }
    // A reply to this message later is recognized by its ID.
    ctx.rules.sent_message_ids.insert(message_id.clone());

    let lab = match early {
        Some(l) => l,
        None => label(worker, &mail).await,
    };
    let mut application = link::find_application(db, &mail, lab.company.as_deref()).await?;
    // No job matches: this mail may be the first sign of one. It is created for the user to
    // confirm, and a job they already dismissed is left alone.
    let mut created = false;
    if application.is_none() {
        if let Some(c) = create::from_mail(db, &mail, &lab, now).await? {
            application = Some(c.application_id);
            created = c.new;
        }
    }
    if let Some(app) = application {
        store::set_application(db, id, Some(app)).await?;
    }
    let current = match application {
        Some(a) => tracking::get(db, a).await.ok(),
        None => None,
    };
    let suggestion = current.and_then(|a| status::suggest(lab.kind, a.status));
    let when = lab.interview_at.as_deref().and_then(parse_when);
    store::save_label(
        db,
        id,
        &LabelRow {
            label: lab.kind.as_str(),
            label_with: &lab.with,
            summary: &lab.summary,
            suggested_status: suggestion.map(Status::as_str),
            interview_at: lab.interview_at.as_deref(),
            needs_reply: lab.needs_reply,
        },
    )
    .await?;

    let mut outcome = None;
    if let (Some(app), Some(to)) = (application, suggestion) {
        let reason = format!("{} ({})", lab.summary, mail.subject);
        outcome = Some(status::apply(db, app, id, to, &reason, now).await?);
    }
    tracing::info!(
        "mail {id}: labelled {} by {}, linked to application {:?}, suggested {:?}, outcome {:?}",
        lab.kind.as_str(),
        lab.with,
        application,
        suggestion.map(Status::as_str),
        outcome
    );
    if let (Some(app), Some(when)) = (application, when) {
        if lab.kind == crate::label::Kind::Interview {
            tracking::set_next_interview(db, app, Some(&when)).await?;
        }
    }

    let mut drafted = false;
    if let (true, Some(w)) = (lab.needs_reply, worker) {
        drafted = reply::draft_reply(db, w, id, "", false).await.is_ok();
    }
    Ok(Some(Processed {
        message_id: id,
        kind: Some(lab.kind.as_str().to_string()),
        application_id: application,
        status: outcome,
        drafted,
        created,
    }))
}

/// Unix seconds as `YYYY-MM-DDTHH:MM:SSZ` (civil-from-days, no time crate needed).
pub fn unix_to_iso(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}
