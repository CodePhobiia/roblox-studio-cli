use crate::bridge::registry::Registry;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::TransferRequest;
use std::time::Duration;

pub async fn run_transfer(
    registry: &Registry,
    req: TransferRequest,
) -> AppResult<serde_json::Value> {
    let src_token = registry.resolve_token(Some(&req.from_studio)).await?;
    let dst_token = registry.resolve_token(Some(&req.to_studio)).await?;

    let rx_serialize = registry
        .enqueue(
            &src_token,
            "serialize",
            serde_json::json!({ "path": req.from_path }),
        )
        .await?;
    let serialize_result = tokio::time::timeout(Duration::from_secs(120), rx_serialize)
        .await
        .map_err(|_| AppError::CommandTimeout {
            timeout_ms: 120_000,
        })?
        .map_err(|_| {
            AppError::Other("source plugin dropped the serialize result channel".into())
        })?;
    if !serialize_result.ok {
        return Err(AppError::PluginError(format!(
            "serialize failed: {}",
            serialize_result
                .error
                .unwrap_or_else(|| "unknown error".into())
        )));
    }
    let blob = serialize_result
        .data
        .ok_or_else(|| AppError::Other("serialize returned no data".into()))?;

    let rx_deserialize = registry
        .enqueue(
            &dst_token,
            "deserialize",
            serde_json::json!({
                "parentPath": req.to_parent_path,
                "blob": blob,
            }),
        )
        .await?;
    let deserialize_result = tokio::time::timeout(Duration::from_secs(120), rx_deserialize)
        .await
        .map_err(|_| AppError::CommandTimeout {
            timeout_ms: 120_000,
        })?
        .map_err(|_| {
            AppError::Other("target plugin dropped the deserialize result channel".into())
        })?;
    if !deserialize_result.ok {
        return Err(AppError::PluginError(format!(
            "deserialize failed: {}",
            deserialize_result
                .error
                .unwrap_or_else(|| "unknown error".into())
        )));
    }

    Ok(deserialize_result
        .data
        .unwrap_or_else(|| serde_json::json!({})))
}
