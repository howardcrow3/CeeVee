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
    db: tauri::State<'_, ceevee_db::Db>,
    csv_text: String,
) -> Result<LinkedInImport, String> {
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
}

/// Jobs found by searches that you have not dismissed, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_jobs(db: tauri::State<'_, ceevee_db::Db>) -> Result<Vec<JobLine>, String> {
    let rows = ceevee_search::postings::recent(&db, 200)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| JobLine {
            id: r.id as u32,
            title: r.title,
            company: r.company,
            location: r.location,
            state: r.state,
            score: r.score,
            url: r.url,
        })
        .collect())
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
    db: tauri::State<'_, ceevee_db::Db>,
    boards: Vec<BoardLine>,
) -> Result<(), String> {
    let boards = boards
        .into_iter()
        .map(to_board)
        .collect::<Result<Vec<_>, _>>()?;
    ceevee_search::sources::save_boards(&db, &boards)
        .await
        .map_err(|e| e.to_string())
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
pub async fn run_search(db: tauri::State<'_, ceevee_db::Db>) -> Result<SearchResult, String> {
    let boards = ceevee_search::sources::load_boards(&db)
        .await
        .map_err(|e| e.to_string())?;
    let fetcher = ceevee_search::sources::HttpFetcher::new().map_err(|e| e.to_string())?;
    let now = ceevee_search::sources::now_iso();
    let s = ceevee_search::sources::run(&db, &fetcher, &boards, &now)
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
            ProposalLine {
                id: p.id as u32,
                action: text("tool").unwrap_or(p.kind),
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
    db: tauri::State<'_, ceevee_db::Db>,
    registry: tauri::State<'_, ceevee_agent::tools::Registry>,
    id: u32,
) -> Result<(), String> {
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
    db: tauri::State<'_, ceevee_db::Db>,
    kind: String,
    title: String,
    content: String,
) -> Result<u32, String> {
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
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
    content: String,
) -> Result<DocView, String> {
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
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
) -> Result<DocView, String> {
    ceevee_docs::store::undo(&db, i64::from(id))
        .await
        .map_err(|e| e.to_string())?;
    view(&db, i64::from(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn redo_document(
    db: tauri::State<'_, ceevee_db::Db>,
    id: u32,
) -> Result<DocView, String> {
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
    app: tauri::AppHandle,
    db: tauri::State<'_, ceevee_db::Db>,
    engine: tauri::State<'_, std::sync::Arc<crate::models::Engine>>,
    id: u32,
    instruction: String,
    accept_flagged: bool,
) -> Result<ChatEdit, String> {
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
