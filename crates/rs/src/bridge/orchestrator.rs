use crate::bridge::registry::{QueuedCommand, Registry};
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{TransferRequest, CLI_VERSION, PLUGIN_PROTOCOL_VERSION};
use std::time::Duration;

pub async fn run_transfer(
    registry: &Registry,
    req: TransferRequest,
) -> AppResult<serde_json::Value> {
    let src_token = registry.resolve_token(Some(&req.from_studio)).await?;
    let dst_token = registry.resolve_token(Some(&req.to_studio)).await?;
    registry
        .ensure_protocol(&src_token, PLUGIN_PROTOCOL_VERSION)
        .await?;
    registry
        .ensure_protocol(&dst_token, PLUGIN_PROTOCOL_VERSION)
        .await?;
    registry.ensure_capability(&src_token, "serialize").await?;
    registry
        .ensure_capability(&dst_token, "deserialize")
        .await?;

    let serialize = registry
        .enqueue_command(
            &src_token,
            "serialize",
            serde_json::json!({
                "path": req.from_path,
                "_rs": {
                    "cliVersion": CLI_VERSION,
                    "protocolVersion": PLUGIN_PROTOCOL_VERSION
                }
            }),
        )
        .await?;
    let serialize_result = await_queued_result(
        registry,
        serialize,
        Duration::from_secs(120),
        "source plugin dropped the serialize result channel",
    )
    .await?;
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
    let external_refs = blocking_external_refs(&blob);
    if !req.allow_external_refs && !external_refs.is_empty() {
        return Err(AppError::PluginError(format!(
            "transfer has external rigid references outside the selected source root: {}. Transfer a common parent or pass --allow-external-refs.",
            summarize_external_refs(&external_refs)
        )));
    }

    let deserialize = registry
        .enqueue_command(
            &dst_token,
            "deserialize",
            transfer_deserialize_payload(&req, blob),
        )
        .await?;
    let deserialize_result = await_queued_result(
        registry,
        deserialize,
        Duration::from_secs(120),
        "target plugin dropped the deserialize result channel",
    )
    .await?;
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

async fn await_queued_result(
    registry: &Registry,
    queued: QueuedCommand,
    timeout: Duration,
    dropped_message: &'static str,
) -> AppResult<crate::protocol::messages::CommandResult> {
    match tokio::time::timeout(timeout, queued.result).await {
        Ok(result) => result.map_err(|_| AppError::Other(dropped_message.into())),
        Err(_) => {
            registry.cancel_pending_command(&queued.command_id).await;
            Err(AppError::CommandTimeout {
                timeout_ms: timeout.as_millis() as u64,
            })
        }
    }
}

fn transfer_deserialize_payload(
    req: &TransferRequest,
    blob: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "parentPath": req.to_parent_path.clone(),
        "blob": blob,
        "conflictMode": req.conflict_mode.clone(),
        "dryRun": req.dry_run,
        "rollbackOnError": req.rollback_on_error,
        "failOnExternalRefs": !req.allow_external_refs,
        "validateRules": ["refs", "welds", "tool"],
        "failOnValidationFailure": true,
        "_rs": {
            "cliVersion": CLI_VERSION,
            "protocolVersion": PLUGIN_PROTOCOL_VERSION
        }
    })
}

fn blocking_external_refs(blob: &serde_json::Value) -> Vec<String> {
    blob.get("externalReferences")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|value| {
            value
                .get("blocking")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
        .map(|value| {
            let path = value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<unknown>");
            let property = value
                .get("property")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<property>");
            let target = value
                .get("targetPath")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<external>");
            format!("{path}.{property} -> {target}")
        })
        .collect()
}

fn summarize_external_refs(refs: &[String]) -> String {
    let mut shown = refs.iter().take(5).cloned().collect::<Vec<_>>();
    if refs.len() > shown.len() {
        shown.push(format!("... {} more", refs.len() - shown.len()));
    }
    shown.join("; ")
}

#[cfg(test)]
mod tests {
    use super::{blocking_external_refs, transfer_deserialize_payload};
    use crate::protocol::messages::{TransferRequest, PLUGIN_PROTOCOL_VERSION};

    #[test]
    fn blocking_external_refs_extracts_only_rigid_refs() {
        let blob = serde_json::json!({
            "externalReferences": [
                {
                    "path": "Workspace.Model.Weld",
                    "property": "Part0",
                    "targetPath": "Workspace.Other",
                    "blocking": true
                },
                {
                    "path": "Workspace.Model.BillboardGui",
                    "property": "Adornee",
                    "targetPath": "Workspace.CameraPart",
                    "blocking": false
                }
            ]
        });

        let refs = blocking_external_refs(&blob);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0], "Workspace.Model.Weld.Part0 -> Workspace.Other");
    }

    #[test]
    fn transfer_deserialize_payload_forwards_guards_and_protocol_metadata() {
        let req = TransferRequest {
            from_studio: "Source".into(),
            from_path: "Workspace.Tool".into(),
            to_studio: "Target".into(),
            to_parent_path: "ServerStorage".into(),
            conflict_mode: Some("replace".into()),
            dry_run: true,
            rollback_on_error: true,
            allow_external_refs: false,
        };
        let payload = transfer_deserialize_payload(&req, serde_json::json!({"version": 1}));

        assert_eq!(payload["parentPath"], "ServerStorage");
        assert_eq!(payload["conflictMode"], "replace");
        assert_eq!(payload["dryRun"], true);
        assert_eq!(payload["rollbackOnError"], true);
        assert_eq!(payload["failOnExternalRefs"], true);
        assert_eq!(payload["validateRules"][0], "refs");
        assert_eq!(payload["failOnValidationFailure"], true);
        assert_eq!(payload["_rs"]["protocolVersion"], PLUGIN_PROTOCOL_VERSION);
    }
}
