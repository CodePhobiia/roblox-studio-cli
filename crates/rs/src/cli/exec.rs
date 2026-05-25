use crate::error::{AppError, AppResult};
use crate::protocol::messages::ExecRequest;
use std::io::Write;

pub fn run(
    port: u16,
    studio: Option<String>,
    lua: String,
    allow_dangerous_exec: bool,
) -> AppResult<()> {
    if !allow_dangerous_exec {
        return Err(AppError::Other(
            "`rs exec` runs arbitrary Luau in Studio. Re-run with --allow-dangerous-exec only for trusted code.".into(),
        ));
    }
    let value: serde_json::Value = crate::cli::request::post(
        port,
        "exec",
        "/exec",
        &ExecRequest {
            studio,
            lua,
            allow_dangerous_exec,
        },
        35,
    )?;
    print_json(value)?;
    Ok(())
}

fn print_json(value: serde_json::Value) -> AppResult<()> {
    match value {
        serde_json::Value::String(s) => println!("{}", serde_json::to_string(&s)?),
        other => println!("{}", serde_json::to_string_pretty(&other)?),
    }
    std::io::stdout().flush()?;
    Ok(())
}
