//! The `apply.prepare` tool: open a job posting in the person's own Chrome or Edge, fill in the
//! form from their profile and resume, and leave the window open for them to check and submit.
//! It never submits. It runs only when the person approves it in the approvals queue.

use std::sync::{Arc, Mutex};

use ceevee_agent::tools::{FnTool, Registry, Risk, ToolSpec};
use ceevee_apply::chrome::{find_browser, ChromeDriver};
use ceevee_apply::engine::fill_application;
use ceevee_core::{Error, Result};
use ceevee_db::Db;
use serde_json::{json, Value};
use tauri::Manager;

/// Windows that were opened and are waiting for the person. Dropping one would close it.
#[derive(Default)]
pub struct OpenWindows(Mutex<Vec<ChromeDriver>>);

pub const ACTION: &str = "apply.prepare";

pub fn register(registry: &mut Registry, app: &tauri::AppHandle, db: &Db) -> Result<()> {
    let windows = Arc::new(OpenWindows::default());
    let app = app.clone();
    let db = db.clone();
    let keep = windows.clone();
    registry.register(FnTool::new(
        ToolSpec {
            name: ACTION,
            description: "Open a job posting in the browser and fill in the application form. \
                          Stops before the final submit; the person reviews and submits.",
            risk: Risk::Act,
            input_schema: json!({
                "type": "object",
                "required": ["id"],
                "properties": {"id": {"type": "integer", "minimum": 1}}
            }),
            action: Some(ACTION),
            untrusted_output: true,
        },
        move |v| {
            let (app, db, keep) = (app.clone(), db.clone(), keep.clone());
            Box::pin(async move { prepare(&app, &db, &keep, &v).await })
        },
    ))?;
    drop(windows);
    Ok(())
}

async fn prepare(
    app: &tauri::AppHandle,
    db: &Db,
    keep: &OpenWindows,
    input: &Value,
) -> Result<Value> {
    let id = input
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| Error::Invalid("id must be a whole number".into()))?;
    let exe = find_browser().ok_or_else(|| {
        Error::Invalid("Install Google Chrome or Microsoft Edge to apply from CeeVee.".into())
    })?;
    let data = app
        .path()
        .app_data_dir()
        .map_err(|e| Error::Other(e.to_string()))?;
    let inputs = ceevee_agent::apply_prep::gather(
        db,
        id,
        &data.join("apply").join(id.to_string()),
        ceevee_docs::render::Paper::UsLetter,
    )
    .await?;
    let mut driver = ChromeDriver::launch(&exe, &data.join("browser-profile"), false).await?;
    let report = match fill_application(&mut driver, &inputs.url, &inputs.answers).await {
        Ok(r) => r,
        Err(e) => {
            let _ = driver.close().await;
            return Err(e);
        }
    };
    keep.0
        .lock()
        .map_err(|e| Error::Other(e.to_string()))?
        .push(driver);
    Ok(json!({
        "ats": report.ats.as_str(),
        "url": report.url,
        "resume": inputs.resume_title,
        "cover_letter": inputs.cover_letter_title,
        "filled": report.filled.iter().map(|f| json!({
            "label": f.label, "value": f.value, "review": f.review
        })).collect::<Vec<_>>(),
        "skipped": report.skipped.iter().map(|s| json!({
            "label": s.label, "required": s.required, "reason": s.reason
        })).collect::<Vec<_>>(),
        "missing_required": report.missing_required(),
        "message": if inputs.cover_letter_title.is_some() {
            "The form is filled in, with your resume and cover letter for this job, and open in your browser. Check it and press Submit yourself."
        } else {
            "The form is filled in and open in your browser. Check it and press Submit yourself."
        }
    }))
}
