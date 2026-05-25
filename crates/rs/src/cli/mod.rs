pub mod apply_plan;
pub mod auth;
pub mod autopilot;
pub mod batch;
pub mod bridge;
pub mod create;
pub mod deps;
pub mod diff;
pub mod doctor;
pub mod exec;
pub mod export;
pub mod history;
pub mod import_asset;
pub mod import_audio;
pub mod import_image;
pub mod import_ui_pack;
pub mod import_uploaded;
pub mod install_plugin;
pub mod list;
pub mod package;
pub mod publish_check;
pub mod read;
pub mod rehost_images;
pub mod repair_tool;
pub mod request;
pub mod smoke;
pub mod snapshot;
pub mod sync_folder;
pub mod sync_pull;
pub mod transaction;
pub mod transfer;
pub mod upload;
pub mod validate;

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
