pub mod bridge;
pub mod exec;
pub mod export;
pub mod import_asset;
pub mod import_image;
pub mod list;
pub mod read;
pub mod transfer;

use crate::error::AppError;

pub fn envelope_error(
    operation: &'static str,
    error: Option<String>,
    code: Option<String>,
) -> AppError {
    AppError::BridgeResponse {
        operation: operation.to_string(),
        error: error.unwrap_or_else(|| "unknown error".into()),
        code: code.unwrap_or_else(|| "unknown".into()),
    }
}
