//! The tools every agent can be given. They sit on the feature crates and are registered once at
//! start-up. Reading tools run freely; anything that changes your data is a draft you can undo.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use ceevee_search::postings::{self, State};
use serde_json::{json, Value};

use crate::tools::{FnTool, Registry, Risk, ToolSpec};

fn id_of(v: &Value) -> Result<i64> {
    v.get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| Error::Invalid("id must be a whole number".into()))
}

/// Register the built-in tools on `registry`.
pub fn register_all(registry: &mut Registry, db: &Db) -> Result<()> {
    let id_schema = json!({
        "type": "object",
        "required": ["id"],
        "properties": {"id": {"type": "integer", "minimum": 1}}
    });

    let d = db.clone();
    registry.register(FnTool::new(
        ToolSpec {
            name: "search.list_jobs",
            description:
                "List the newest jobs that have not been dismissed, best match first among equals.",
            risk: Risk::Read,
            input_schema: json!({
                "type": "object",
                "properties": {"limit": {"type": "integer", "minimum": 1, "maximum": 100}}
            }),
            action: None,
            untrusted_output: true,
        },
        move |v| {
            let d = d.clone();
            Box::pin(async move {
                let limit = v.get("limit").and_then(Value::as_i64).unwrap_or(25);
                let rows = postings::recent(&d, limit).await?;
                serde_json::to_value(rows).map_err(|e| Error::Other(e.to_string()))
            })
        },
    ))?;

    let d = db.clone();
    registry.register(FnTool::new(
        ToolSpec {
            name: "search.get_posting",
            description: "Read one job posting in full, including its description.",
            risk: Risk::Read,
            input_schema: id_schema.clone(),
            action: None,
            untrusted_output: true,
        },
        move |v| {
            let d = d.clone();
            Box::pin(async move {
                let p = postings::get(&d, id_of(&v)?).await?;
                serde_json::to_value(p).map_err(|e| Error::Other(e.to_string()))
            })
        },
    ))?;

    for (name, description, state) in [
        ("search.save_posting", "Mark a job as saved.", State::Saved),
        (
            "search.dismiss_posting",
            "Hide a job from the list. It can be brought back.",
            State::Dismissed,
        ),
    ] {
        let d = db.clone();
        registry.register(FnTool::new(
            ToolSpec {
                name,
                description,
                risk: Risk::Draft,
                input_schema: id_schema.clone(),
                action: None,
                untrusted_output: false,
            },
            move |v| {
                let d = d.clone();
                Box::pin(async move {
                    let id = id_of(&v)?;
                    postings::set_state(&d, id, state).await?;
                    Ok(json!({"id": id, "state": format!("{state:?}").to_lowercase()}))
                })
            },
        ))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
