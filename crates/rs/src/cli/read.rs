use crate::error::AppResult;
use crate::protocol::messages::ReadRequest;
use std::io::Write;

pub fn run(port: u16, studio: Option<String>, path: String, depth: u32) -> AppResult<()> {
    let value: serde_json::Value = crate::cli::request::post(
        port,
        "read",
        "/read",
        &ReadRequest {
            studio,
            path,
            depth,
        },
        35,
    )?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    std::io::stdout().flush()?;
    Ok(())
}
