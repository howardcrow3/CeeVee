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
