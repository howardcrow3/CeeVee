//! Commands the UI calls that sit on top of the feature crates.

use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Serialize, Type)]
pub struct LinkedInImport {
    pub added: u32,
    pub updated: u32,
}

/// Import the text of LinkedIn's Connections.csv.
#[tauri::command]
#[specta::specta]
pub async fn import_linkedin_connections(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    csv_text: String,
) -> Result<LinkedInImport, String> {
    crate::license_cmd::require_write(&lic).await?;
    let connections =
        ceevee_network::linkedin::parse_connections(&csv_text).map_err(|e| e.to_string())?;
    let s = ceevee_network::linkedin::import(&db, &connections)
        .await
        .map_err(|e| e.to_string())?;
    Ok(LinkedInImport {
        added: s.added as u32,
        updated: s.updated as u32,
    })
}

/// Render resume JSON to PDF and write it to `path`.
#[tauri::command]
#[specta::specta]
pub async fn export_resume_pdf(resume_json: String, path: String, a4: bool) -> Result<(), String> {
    let resume: serde_json::Value =
        serde_json::from_str(&resume_json).map_err(|e| e.to_string())?;
    let paper = if a4 {
        ceevee_docs::render::Paper::A4
    } else {
        ceevee_docs::render::Paper::UsLetter
    };
    let pdf = tauri::async_runtime::spawn_blocking(move || {
        ceevee_docs::render::resume_pdf(&resume, paper)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    std::fs::write(path, pdf).map_err(|e| e.to_string())
}

/// One line of the jobs list.
#[derive(Debug, Clone, Serialize, Type)]
pub struct JobLine {
    pub id: u32,
    pub title: String,
    pub company: Option<String>,
    pub location: Option<String>,
    pub state: String,
    pub score: Option<f64>,
    pub url: String,
    /// How many of your LinkedIn connections currently work at this company.
    pub known: u32,
}

/// Jobs found by searches that you have not dismissed, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_jobs(db: tauri::State<'_, ceevee_db::Db>) -> Result<Vec<JobLine>, String> {
    let rows = ceevee_search::postings::recent(&db, 200)
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        let known = match r.company_id {
            Some(c) => ceevee_network::linkedin::connections_at(&db, c)
                .await
                .map(|p| p.len() as u32)
                .unwrap_or(0),
            None => 0,
        };
        out.push(JobLine {
            id: r.id as u32,
            title: r.title,
            company: r.company,
            location: r.location,
            state: r.state,
            score: r.score,
            url: r.url,
            known,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct KnownLine {
    pub name: String,
    pub title: Option<String>,
    pub profile_url: Option<String>,
}

/// People you know at the company that posted a job.
#[tauri::command]
#[specta::specta]
pub async fn people_at_job(
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
) -> Result<Vec<KnownLine>, String> {
    let p = ceevee_search::postings::get(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    let Some(c) = p.company_id else {
        return Ok(Vec::new());
    };
    let people = ceevee_network::linkedin::connections_at(&db, c)
        .await
        .map_err(|e| e.to_string())?;
    Ok(people
        .into_iter()
        .map(|k| KnownLine {
            name: k.full_name,
            title: k.title,
            profile_url: k.profile_url,
        })
        .collect())
}

/// Hide a job you are not interested in.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_job(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
) -> Result<(), String> {
    crate::license_cmd::require_write(&lic).await?;
    ceevee_search::postings::set_state(
        &db,
        i64::from(id),
        ceevee_search::postings::State::Dismissed,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Open a job posting in your default browser. Only web addresses are opened.
#[tauri::command]
#[specta::specta]
pub fn open_posting(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("That is not a web address.".into());
    }
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| e.to_string())
}

/// How many proposals are waiting for you to approve.
#[tauri::command]
#[specta::specta]
pub async fn pending_approval_count(db: tauri::State<'_, ceevee_db::Db>) -> Result<u32, String> {
    let n = ceevee_agent::approvals::pending_count(&db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(n as u32)
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Type)]
pub struct BoardLine {
    /// `greenhouse` or `lever`.
    pub kind: String,
    pub token: String,
    pub company: String,
}

fn to_board(b: BoardLine) -> Result<ceevee_search::sources::Board, String> {
    use ceevee_search::sources::{Board, BoardKind};
    let kind = match b.kind.as_str() {
        "greenhouse" => BoardKind::Greenhouse,
        "lever" => BoardKind::Lever,
        other => return Err(format!("unknown job board type \"{other}\"")),
    };
    Ok(Board {
        kind,
        token: b.token.trim().to_owned(),
        company: b.company.trim().to_owned(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn list_boards(db: tauri::State<'_, ceevee_db::Db>) -> Result<Vec<BoardLine>, String> {
    use ceevee_search::sources::BoardKind;
    let boards = ceevee_search::sources::load_boards(&db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(boards
        .into_iter()
        .map(|b| BoardLine {
            kind: match b.kind {
                BoardKind::Greenhouse => "greenhouse".into(),
                BoardKind::Lever => "lever".into(),
            },
            token: b.token,
            company: b.company,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn save_boards(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    boards: Vec<BoardLine>,
) -> Result<(), String> {
    crate::license_cmd::require_write(&lic).await?;
    let boards = boards
        .into_iter()
        .map(to_board)
        .collect::<Result<Vec<_>, _>>()?;
    ceevee_search::sources::save_boards(&db, &boards)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Type)]
pub struct CategoryLine {
    pub id: u32,
    pub name: String,
    pub titles: Vec<String>,
    pub keywords: Vec<String>,
    pub locations: Vec<String>,
    pub remote_ok: bool,
    pub avoid: Vec<String>,
    pub active: bool,
}

fn category_line(c: ceevee_search::categories::Category) -> CategoryLine {
    CategoryLine {
        id: c.id as u32,
        name: c.name,
        titles: c.criteria.titles,
        keywords: c.criteria.keywords,
        locations: c.criteria.locations,
        remote_ok: c.criteria.remote_ok,
        avoid: c.criteria.avoid,
        active: c.active,
    }
}

/// The kinds of job you are looking for, highest priority first.
#[tauri::command]
#[specta::specta]
pub async fn list_categories(
    db: tauri::State<'_, ceevee_db::Db>,
) -> Result<Vec<CategoryLine>, String> {
    let all = ceevee_search::categories::list(&db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(all.into_iter().map(category_line).collect())
}

/// Add a kind of job to look for. The id in `line` is ignored.
#[tauri::command]
#[specta::specta]
pub async fn create_category(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    line: CategoryLine,
) -> Result<CategoryLine, String> {
    crate::license_cmd::require_write(&lic).await?;
    let criteria = ceevee_search::categories::Criteria {
        titles: line.titles,
        keywords: line.keywords,
        locations: line.locations,
        remote_ok: line.remote_ok,
        avoid: line.avoid,
    };
    let c = ceevee_search::categories::create(&db, &line.name, &criteria)
        .await
        .map_err(|e| e.to_string())?;
    Ok(category_line(c))
}

/// Turn a category on or off. Off categories are not used to score jobs.
#[tauri::command]
#[specta::specta]
pub async fn set_category_active(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
    active: bool,
) -> Result<(), String> {
    crate::license_cmd::require_write(&lic).await?;
    ceevee_search::categories::set_active(&db, i64::from(id), active)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ProfileChange {
    /// The change itself, to hand back to `apply_profile_changes` if accepted.
    pub change: String,
    /// What it does, in one sentence.
    pub text: String,
}

/// Read resume text with the local model and list what it would change in your profile. Nothing is
/// saved until you accept.
#[tauri::command]
#[specta::specta]
pub async fn preview_resume_import(
    app: tauri::AppHandle,
    engine: tauri::State<'_, std::sync::Arc<crate::models::Engine>>,
    db: tauri::State<'_, ceevee_db::Db>,
    text: String,
) -> Result<Vec<ProfileChange>, String> {
    if text.trim().is_empty() {
        return Err("Paste the text of your resume first.".into());
    }
    let worker = crate::models::worker_for(&app, &engine)?;
    let out: ceevee_ai::task::TaskOutput<ceevee_profile::ResumeDraft> = ceevee_ai::task::run(
        &worker,
        &ceevee_ai::task::IMPORT_RESUME,
        &[("resume", text.as_str())],
        ceevee_ai::Priority::Interactive,
    )
    .await
    .map_err(|e| e.to_string())?;
    let plan = ceevee_profile::plan_import(&db, &out.value)
        .await
        .map_err(|e| e.to_string())?;
    plan.iter()
        .map(|c| {
            Ok(ProfileChange {
                change: serde_json::to_string(c).map_err(|e| e.to_string())?,
                text: ceevee_profile::describe(c),
            })
        })
        .collect()
}

/// Save the changes you accepted. Returns how many were applied.
#[tauri::command]
#[specta::specta]
pub async fn apply_profile_changes(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    changes: Vec<String>,
) -> Result<u32, String> {
    crate::license_cmd::require_write(&lic).await?;
    let mut n = 0;
    for raw in changes {
        let c: ceevee_profile::Change =
            serde_json::from_str(&raw).map_err(|e| format!("Could not read a change: {e}"))?;
        ceevee_profile::repo::apply(&db, &c)
            .await
            .map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct SearchResult {
    pub boards_ok: u32,
    pub new: u32,
    pub duplicates: u32,
    pub failures: Vec<String>,
}

/// Read every watched board once and store what is new.
#[tauri::command]
#[specta::specta]
pub async fn run_search(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    analytics: tauri::State<'_, crate::analytics_cmd::Handle>,
) -> Result<SearchResult, String> {
    crate::license_cmd::require_write(&lic).await?;
    analytics
        .track(ceevee_analytics::Event::FeatureUsed(
            ceevee_analytics::Feature::JobSearchRun,
        ))
        .await;
    let fetcher = ceevee_search::sources::HttpFetcher::new().map_err(|e| e.to_string())?;
    let now = ceevee_search::sources::now_iso();
    let s = ceevee_search::starter::run_all(&db, &fetcher, &now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(SearchResult {
        boards_ok: s.boards_ok,
        new: s.new,
        duplicates: s.duplicates,
        failures: s.failures,
    })
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ProposalLine {
    pub id: u32,
    /// What CeeVee wants to do, from the tool's name, such as `apply.submit`.
    pub action: String,
    /// CeeVee's reason, in its own words.
    pub reason: String,
    pub created_at: String,
}

/// Everything waiting for your approval, oldest first.
#[tauri::command]
#[specta::specta]
pub async fn list_proposals(
    db: tauri::State<'_, ceevee_db::Db>,
) -> Result<Vec<ProposalLine>, String> {
    let pending = ceevee_agent::approvals::pending(&db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(pending
        .into_iter()
        .map(|p| {
            let text = |k: &str| p.payload.get(k).and_then(|v| v.as_str()).map(str::to_owned);
            let action = if p.kind == "mail_status" {
                format!("Mark application as {}", text("to").unwrap_or_default())
            } else {
                text("tool").unwrap_or(p.kind.clone())
            };
            ProposalLine {
                id: p.id as u32,
                action,
                reason: text("reason").unwrap_or_default(),
                created_at: p.created_at,
            }
        })
        .collect())
}

/// Skip a proposal. Approving needs the tools that carry the action out, which come later.
#[tauri::command]
#[specta::specta]
pub async fn skip_proposal(db: tauri::State<'_, ceevee_db::Db>, id: u32) -> Result<(), String> {
    ceevee_agent::approvals::reject(&db, i64::from(id))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Approve a waiting proposal and carry it out once.
#[tauri::command]
#[specta::specta]
pub async fn approve_proposal(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    registry: tauri::State<'_, ceevee_agent::tools::Registry>,
    id: u32,
) -> Result<(), String> {
    crate::license_cmd::require_write(&lic).await?;
    let p = ceevee_agent::approvals::get(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    if p.kind == "mail_status" {
        let p = ceevee_agent::approvals::approve_only(&db, i64::from(id))
            .await
            .map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        return ceevee_mail::status::apply_proposal(&db, &p.payload, &now)
            .await
            .map_err(|e| e.to_string());
    }
    ceevee_agent::approvals::approve_and_run(&db, &registry, i64::from(id))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct DocLine {
    pub id: u32,
    /// `base_resume`, `resume_variant`, `tailored_resume`, `cover_letter` or `email`.
    pub kind: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct DocView {
    pub id: u32,
    pub kind: String,
    pub title: String,
    /// Resume JSON or letter Markdown.
    pub content: String,
    pub versions: u32,
}

fn doc_kind(s: &str) -> Result<ceevee_docs::store::DocKind, String> {
    use ceevee_docs::store::DocKind;
    Ok(match s {
        "base_resume" => DocKind::BaseResume,
        "resume_variant" => DocKind::ResumeVariant,
        "tailored_resume" => DocKind::TailoredResume,
        "cover_letter" => DocKind::CoverLetter,
        "email" => DocKind::Email,
        other => return Err(format!("unknown document type \"{other}\"")),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn list_documents(db: tauri::State<'_, ceevee_db::Db>) -> Result<Vec<DocLine>, String> {
    let docs = ceevee_docs::store::list(&db, None, None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(docs
        .into_iter()
        .map(|d| DocLine {
            id: d.id as u32,
            kind: d.kind.as_str().to_owned(),
            title: d.title,
        })
        .collect())
}

async fn view(db: &ceevee_db::Db, id: i64) -> Result<DocView, String> {
    let d = ceevee_docs::store::get(db, id)
        .await
        .map_err(|e| e.to_string())?;
    let cur = ceevee_docs::store::current(db, id)
        .await
        .map_err(|e| e.to_string())?;
    let versions = ceevee_docs::store::versions(db, id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(DocView {
        id: d.id as u32,
        kind: d.kind.as_str().to_owned(),
        title: d.title,
        content: cur.content,
        versions: versions.len() as u32,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn get_document(db: tauri::State<'_, ceevee_db::Db>, id: u32) -> Result<DocView, String> {
    view(&db, i64::from(id)).await
}

/// Make a new document. Resumes must be valid JSON.
#[tauri::command]
#[specta::specta]
pub async fn create_document(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    kind: String,
    title: String,
    content: String,
) -> Result<u32, String> {
    crate::license_cmd::require_write(&lic).await?;
    let kind = doc_kind(&kind)?;
    if title.trim().is_empty() {
        return Err("Give the document a title.".into());
    }
    check_content(kind, &content)?;
    let id = ceevee_docs::store::create(&db, kind, title.trim(), None, None, &content, "created")
        .await
        .map_err(|e| e.to_string())?;
    Ok(id as u32)
}

fn check_content(kind: ceevee_docs::store::DocKind, content: &str) -> Result<(), String> {
    use ceevee_docs::store::DocKind;
    if matches!(
        kind,
        DocKind::BaseResume | DocKind::ResumeVariant | DocKind::TailoredResume
    ) {
        serde_json::from_str::<serde_json::Value>(content)
            .map_err(|e| format!("A resume has to be valid JSON: {e}"))?;
    }
    Ok(())
}

/// Save your edit as a new version. Nothing changes if the text is the same.
#[tauri::command]
#[specta::specta]
pub async fn save_document(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
    content: String,
) -> Result<DocView, String> {
    crate::license_cmd::require_write(&lic).await?;
    let d = ceevee_docs::store::get(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    check_content(d.kind, &content)?;
    ceevee_docs::store::add_version(&db, i64::from(id), &content, "edited by you", None)
        .await
        .map_err(|e| e.to_string())?;
    view(&db, i64::from(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn undo_document(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
) -> Result<DocView, String> {
    crate::license_cmd::require_write(&lic).await?;
    ceevee_docs::store::undo(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    view(&db, i64::from(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn redo_document(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
) -> Result<DocView, String> {
    crate::license_cmd::require_write(&lic).await?;
    ceevee_docs::store::redo(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    view(&db, i64::from(id)).await
}

/// The outcome of asking the assistant to change a document.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct ChatEdit {
    /// True when the change was saved as a new version.
    pub applied: bool,
    /// Names or numbers in the change that were in neither the document nor your request. When
    /// there are any, nothing is saved and you decide what to do.
    pub flagged: Vec<String>,
    /// The proposed text, so it can be shown when it was held back.
    pub proposed: String,
    pub doc: DocView,
}

/// Ask the assistant to change a document. The change becomes a new version, so undo works.
#[tauri::command]
#[specta::specta]
pub async fn chat_edit_document(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    app: tauri::AppHandle,
    db: tauri::State<'_, ceevee_db::Db>,
    engine: tauri::State<'_, std::sync::Arc<crate::models::Engine>>,
    id: u32,
    instruction: String,
    accept_flagged: bool,
) -> Result<ChatEdit, String> {
    crate::license_cmd::require_write(&lic).await?;
    use ceevee_docs::chat;
    let d = ceevee_docs::store::get(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    let cur = ceevee_docs::store::current(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    let prompt = chat::edit_prompt(d.kind, &cur.content, &instruction);
    let raw = crate::models::generate(&app, &engine, prompt).await?;
    let proposed = chat::clean_reply(&raw);
    if proposed.is_empty() {
        return Err("The assistant did not return a document.".into());
    }
    check_content(d.kind, &proposed)?;
    let flagged: Vec<String> = chat::check_edit(&cur.content, &instruction, &proposed)
        .into_iter()
        .map(|f| f.text)
        .collect();
    let applied = flagged.is_empty() || accept_flagged;
    if applied {
        let cause = format!("chat: {}", instruction.chars().take(80).collect::<String>());
        ceevee_docs::store::add_version(&db, i64::from(id), &proposed, &cause, Some("local model"))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(ChatEdit {
        applied,
        flagged,
        proposed,
        doc: view(&db, i64::from(id)).await?,
    })
}

/// Save the current version of a document as a PDF in your Documents folder, under "CeeVee".
/// Returns the full path of the file.
#[tauri::command]
#[specta::specta]
pub async fn export_document_pdf(
    app: tauri::AppHandle,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
    a4: bool,
) -> Result<String, String> {
    use tauri::Manager;
    let d = ceevee_docs::store::get(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    let cur = ceevee_docs::store::current(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    let paper = if a4 {
        ceevee_docs::render::Paper::A4
    } else {
        ceevee_docs::render::Paper::UsLetter
    };
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        ceevee_docs::export::pdf_for(d.kind, &cur.content, paper).map(|b| (b, d.title))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let (bytes, title) = bytes;
    let dir = app
        .path()
        .document_dir()
        .map_err(|e| e.to_string())?
        .join("CeeVee");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.pdf", ceevee_docs::export::file_stem(&title)));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

/// A cover letter or tailored resume written for one job, not saved yet.
#[derive(Debug, Clone, Serialize, Type)]
pub struct JobDraft {
    /// `cover_letter` or `tailored_resume`.
    pub kind: String,
    /// Letter Markdown or resume JSON.
    pub content: String,
    /// Things the draft says that your profile and the posting do not. Review these before saving.
    pub flagged: Vec<String>,
}

async fn posting_text(db: &ceevee_db::Db, id: i64) -> Result<(String, String), String> {
    let p = ceevee_search::postings::get(db, id)
        .await
        .map_err(|e| e.to_string())?;
    let company = match p.company_id {
        Some(c) => ceevee_search::companies::get(db, c)
            .await
            .ok()
            .map(|c| c.name),
        None => None,
    };
    let mut text = format!("{}\n", p.title);
    if let Some(c) = &company {
        text.push_str(&format!("Company: {c}\n"));
    }
    if let Some(l) = &p.location {
        text.push_str(&format!("Location: {l}\n"));
    }
    if let Some(b) = &p.body {
        text.push_str(b);
    }
    let name = match company {
        Some(c) => format!("{} at {c}", p.title),
        None => p.title,
    };
    Ok((text, name))
}

/// Write a cover letter or tailored resume for a job with the local model. Nothing is saved until
/// you call `save_job_draft`.
#[tauri::command]
#[specta::specta]
pub async fn draft_for_job(
    app: tauri::AppHandle,
    engine: tauri::State<'_, std::sync::Arc<crate::models::Engine>>,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
    kind: String,
    instructions: String,
) -> Result<JobDraft, String> {
    let (posting, _) = posting_text(&db, i64::from(id)).await?;
    let worker = crate::models::worker_for(&app, &engine)?;
    let draft = match kind.as_str() {
        "cover_letter" => {
            let profile = ceevee_mail::reply::profile_text(&db)
                .await
                .map_err(|e| e.to_string())?;
            ceevee_docs::draft::draft_cover_letter(&worker, &profile, &posting, &instructions).await
        }
        "tailored_resume" => {
            let base =
                ceevee_docs::store::list(&db, Some(ceevee_docs::store::DocKind::BaseResume), None)
                    .await
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .next()
                    .ok_or("Add a base resume in Documents first.")?;
            let cur = ceevee_docs::store::current(&db, base.id)
                .await
                .map_err(|e| e.to_string())?;
            let json: serde_json::Value = serde_json::from_str(&cur.content)
                .map_err(|e| format!("Your base resume is not valid JSON: {e}"))?;
            ceevee_docs::draft::draft_tailored_resume(&worker, &json, &posting).await
        }
        other => return Err(format!("Cannot draft a \"{other}\".")),
    }
    .map_err(|e| e.to_string())?;
    Ok(JobDraft {
        kind,
        content: draft.content,
        flagged: draft.flags.iter().map(|f| f.text.clone()).collect(),
    })
}

/// Save a draft you reviewed as a new document. Returns the document id.
#[tauri::command]
#[specta::specta]
pub async fn save_job_draft(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
    kind: String,
    content: String,
) -> Result<u32, String> {
    crate::license_cmd::require_write(&lic).await?;
    let k = doc_kind(&kind)?;
    check_content(k, &content)?;
    let (_, name) = posting_text(&db, i64::from(id)).await?;
    // A draft belongs to the application for this job, so applying attaches it.
    let application = track_posting(&db, i64::from(id)).await?;
    let label = if kind == "cover_letter" {
        "Cover letter"
    } else {
        "Resume"
    };
    let doc = ceevee_docs::store::create(
        &db,
        k,
        &format!("{label}: {name}"),
        Some(application),
        None,
        &content,
        "drafted",
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(doc as u32)
}

/// Show a file you saved in your computer's file manager. Only files, never programs, are shown.
#[tauri::command]
#[specta::specta]
pub fn show_in_folder(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.is_file() {
        return Err("That file is not there any more.".into());
    }
    tauri_plugin_opener::reveal_item_in_dir(&p).map_err(|e| e.to_string())
}

/// What happened when CeeVee filled in an application.
#[derive(Debug, Clone, Serialize, Type)]
pub struct PrepareResult {
    pub message: String,
    pub filled: u32,
    /// Required questions CeeVee could not answer; you finish these in the browser.
    pub missing: Vec<String>,
    pub resume: Option<String>,
    /// Title of the cover letter attached, when one was saved for this job.
    pub cover_letter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ApplicationLine {
    pub id: u32,
    pub title: String,
    pub company: Option<String>,
    pub url: Option<String>,
    pub status: String,
    pub applied_at: Option<String>,
}

async fn application_line(
    db: &ceevee_db::Db,
    a: ceevee_search::tracking::Application,
) -> ApplicationLine {
    let posting = match a.posting_id {
        Some(p) => ceevee_search::postings::get(db, p).await.ok(),
        None => None,
    };
    let company = match a.company_id.or(posting.as_ref().and_then(|p| p.company_id)) {
        Some(c) => ceevee_search::companies::get(db, c)
            .await
            .ok()
            .map(|c| c.name),
        None => None,
    };
    ApplicationLine {
        id: a.id as u32,
        title: posting
            .as_ref()
            .map_or_else(|| "Application".to_string(), |p| p.title.clone()),
        company,
        url: posting.map(|p| p.url),
        status: a.status.as_str().to_string(),
        applied_at: a.applied_at,
    }
}

/// Every application you are tracking, in board order.
#[tauri::command]
#[specta::specta]
pub async fn list_applications(
    db: tauri::State<'_, ceevee_db::Db>,
) -> Result<Vec<ApplicationLine>, String> {
    let all = ceevee_search::tracking::list(&db, &ceevee_search::tracking::Filter::default())
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for a in all {
        out.push(application_line(&db, a).await);
    }
    Ok(out)
}

/// Add an application by hand: a job you applied to outside CeeVee.
#[tauri::command]
#[specta::specta]
pub async fn add_application(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    company: String,
    title: String,
    status: String,
) -> Result<(), String> {
    crate::license_cmd::require_write(&lic).await?;
    let status = ceevee_search::tracking::Status::parse(&status).map_err(|e| e.to_string())?;
    ceevee_search::tracking::add_manual(
        &db,
        &company,
        &title,
        status,
        &ceevee_search::sources::now_iso(),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Move an application to a new status, for example after you submit it.
#[tauri::command]
#[specta::specta]
pub async fn set_application_status(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
    status: String,
) -> Result<(), String> {
    crate::license_cmd::require_write(&lic).await?;
    let to = ceevee_search::tracking::Status::parse(&status).map_err(|e| e.to_string())?;
    ceevee_search::tracking::set_status(
        &db,
        i64::from(id),
        to,
        ceevee_search::tracking::Cause::Manual,
        &ceevee_search::sources::now_iso(),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Start tracking a posting as a draft application, unless it is tracked already.
async fn track_posting(db: &ceevee_db::Db, posting_id: i64) -> Result<i64, String> {
    let all = ceevee_search::tracking::list(db, &ceevee_search::tracking::Filter::default())
        .await
        .map_err(|e| e.to_string())?;
    if let Some(a) = all.iter().find(|a| a.posting_id == Some(posting_id)) {
        return Ok(a.id);
    }
    let posting = ceevee_search::postings::get(db, posting_id)
        .await
        .map_err(|e| e.to_string())?;
    ceevee_search::tracking::create(
        db,
        &ceevee_search::tracking::NewApplication {
            posting_id: Some(posting_id),
            company_id: posting.company_id,
            category_id: None,
        },
        &ceevee_search::sources::now_iso(),
    )
    .await
    .map(|a| a.id)
    .map_err(|e| e.to_string())
}

/// Open a posting in your browser and fill in its application form. You review and submit.
#[tauri::command]
#[specta::specta]
pub async fn prepare_application(
    lic: tauri::State<'_, std::sync::Arc<crate::license_cmd::Service>>,
    db: tauri::State<'_, ceevee_db::Db>,
    registry: tauri::State<'_, ceevee_agent::tools::Registry>,
    id: u32,
) -> Result<PrepareResult, String> {
    crate::license_cmd::require_write(&lic).await?;
    track_posting(&db, i64::from(id)).await?;
    let v = registry
        .call(crate::apply_tool::ACTION, serde_json::json!({ "id": id }))
        .await
        .map_err(|e| e.to_string())?;
    let list = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default()
    };
    Ok(PrepareResult {
        message: v["message"].as_str().unwrap_or_default().to_string(),
        filled: list("filled").len() as u32,
        missing: list("missing_required")
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        resume: v["resume"].as_str().map(str::to_string),
        cover_letter: v["cover_letter"].as_str().map(str::to_string),
    })
}
