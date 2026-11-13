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
