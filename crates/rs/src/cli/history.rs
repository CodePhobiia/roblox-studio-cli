use crate::error::{AppError, AppResult};
use crate::protocol::messages::HistoryRequest;
use serde_json::Value;

pub fn list(port: u16, studio: Option<String>, json: bool) -> AppResult<()> {
    let response = request(port, studio, "list", None)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let records = response
            .get("records")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if records.is_empty() {
            println!("No rs command history recorded in this Studio session.");
            return Ok(());
        }
        println!("Recent rs commands:");
        for record in records {
            let id = record
                .get("commandId")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");
            let kind = record
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");
            let status = record
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");
            let at = record
                .get("startedAt")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");
            println!("  {id}  {kind}  {status}  {at}");
        }
    }
    Ok(())
}

pub fn show(port: u16, studio: Option<String>, id: String, json: bool) -> AppResult<()> {
    let response = request(port, studio, "show", Some(id))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&response)?);
    }
    Ok(())
}

pub fn undo(port: u16, studio: Option<String>, id: String, yes: bool, json: bool) -> AppResult<()> {
    if !yes {
        return Err(AppError::Other(
            "undo mutates Studio; pass --yes to restore a recorded rollback snapshot".into(),
        ));
    }
    let response = request(port, studio, "undo", Some(id))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "Undo {}",
            response["status"].as_str().unwrap_or("completed")
        );
        if let Some(path) = response.get("rootPath").and_then(Value::as_str) {
            println!("Restored: {path}");
        }
        if let Some(warnings) = response.get("warnings").and_then(Value::as_array) {
            for warning in warnings {
                println!("  - {}", warning.as_str().unwrap_or("<warning>"));
            }
        }
    }
    Ok(())
}

fn request(
    port: u16,
    studio: Option<String>,
    action: &str,
    command_id: Option<String>,
) -> AppResult<Value> {
    crate::cli::request::post(
        port,
        "history",
        "/history",
        &HistoryRequest {
            studio,
            action: action.to_string(),
            command_id,
        },
        150,
    )
}
