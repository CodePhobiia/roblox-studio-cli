use crate::bridge::orchestrator::run_transfer;
use crate::bridge::registry::Registry;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{
    ApplyPlanRequest, AutopilotReviewRequest, CommandResult, CreateInstanceRequest, DepsRequest,
    DeserializeRequest, Envelope, ExecRequest, ExportRequest, HistoryRequest, ImportAssetRequest,
    ImportAudioRequest, ImportImageRequest, ImportUiPackRequest, ImportUploadedRequest,
    PackageUpdateRequest, PluginCommand, ReadRequest, RegisterRequest, RegisterResponse,
    RepairToolRequest, SerializeRequest, SnapshotRequest, TransferRequest, UpsertFilesRequest,
    ValidateRequest, CLI_VERSION, PLUGIN_PROTOCOL_VERSION,
};
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    registry: Registry,
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    auth_token: Arc<String>,
}

pub async fn serve(port: u16) -> AppResult<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let registry = Registry::new();
    let auth_token = crate::bridge::auth::load_or_create_token()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let state = AppState {
        registry,
        shutdown: Arc::new(Mutex::new(Some(shutdown_tx))),
        auth_token: Arc::new(auth_token),
    };
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!(%port, "rs bridge listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await?;
    Ok(())
}

fn build_router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/studios", get(studios))
        .route("/exec", post(exec))
        .route("/read", post(read))
        .route("/export", post(export))
        .route("/import-asset", post(import_asset))
        .route("/import-image", post(import_image))
        .route("/import-uploaded", post(import_uploaded))
        .route("/import-ui-pack", post(import_ui_pack))
        .route("/import-audio", post(import_audio))
        .route("/validate", post(validate))
        .route("/repair-tool", post(repair_tool))
        .route("/snapshot", post(snapshot))
        .route("/create", post(create_instance))
        .route("/upsert-files", post(upsert_files))
        .route("/apply-plan", post(apply_plan))
        .route("/package-update", post(package_update))
        .route("/history", post(history))
        .route("/deps", post(deps))
        .route("/serialize", post(serialize))
        .route("/deserialize", post(deserialize))
        .route("/autopilot-review", post(autopilot_review))
        .route("/transfer", post(transfer))
        .route("/shutdown", post(shutdown))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_bridge_auth,
        ));

    Router::new()
        .route("/healthz", get(healthz))
        .route("/register", post(register))
        .route("/poll/:session_token", get(poll))
        .route("/result/:command_id", post(result))
        .route("/heartbeat/:session_token", post(heartbeat))
        .merge(protected)
        .with_state(state)
}

async fn require_bridge_auth(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let authorized = bridge_auth_matches(request.headers(), state.auth_token.as_str());

    if authorized {
        return next.run(request).await;
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(Envelope::<serde_json::Value>::err(
            "bridge request missing or invalid auth token; use the rs CLI or set RS_BRIDGE_TOKEN for trusted local tooling",
            "unauthorized",
        )),
    )
        .into_response()
}

fn bridge_auth_matches(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(crate::bridge::auth::TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|token| token == expected)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Json<Envelope<RegisterResponse>> {
    let session_token = state.registry.register(req).await;
    Json(Envelope::ok(RegisterResponse { session_token }))
}

async fn poll(
    State(state): State<AppState>,
    Path(session_token): Path<String>,
) -> Json<Envelope<Option<PluginCommand>>> {
    match state
        .registry
        .poll(&session_token, Duration::from_secs(10))
        .await
    {
        Ok(command) => Json(Envelope::ok(command)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn result(
    State(state): State<AppState>,
    Path(command_id): Path<String>,
    Json(req): Json<CommandResult>,
) -> Json<Envelope<serde_json::Value>> {
    match state.registry.submit_result(&command_id, req).await {
        Ok(()) => Json(Envelope::ok(serde_json::json!({}))),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn heartbeat(
    State(state): State<AppState>,
    Path(session_token): Path<String>,
) -> Json<Envelope<serde_json::Value>> {
    match state.registry.heartbeat(&session_token).await {
        Ok(()) => Json(Envelope::ok(serde_json::json!({}))),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn studios(
    State(state): State<AppState>,
) -> Json<Envelope<Vec<crate::protocol::messages::StudioInfo>>> {
    Json(Envelope::ok(state.registry.list().await))
}

async fn exec(
    State(state): State<AppState>,
    Json(req): Json<ExecRequest>,
) -> Json<Envelope<serde_json::Value>> {
    if !req.allow_dangerous_exec {
        return Json(Envelope::err(
            "exec runs arbitrary Luau and requires allowDangerousExec=true",
            "dangerous-exec-not-approved",
        ));
    }
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "exec",
        serde_json::json!({ "lua": req.lua, "allowDangerousExec": true }),
        30,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn read(
    State(state): State<AppState>,
    Json(req): Json<ReadRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "read",
        serde_json::json!({ "path": req.path, "depth": req.depth }),
        30,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn export(
    State(state): State<AppState>,
    Json(req): Json<ExportRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "export",
        serde_json::json!({ "path": req.path, "depth": req.depth }),
        120,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn import_asset(
    State(state): State<AppState>,
    Json(req): Json<ImportAssetRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "importAsset",
        serde_json::json!({
            "parentPath": req.parent_path,
            "name": req.name,
            "meshes": req.meshes,
            "anchored": req.anchored,
            "weld": req.weld,
            "sourceId": req.source_id
        }),
        180,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn import_image(
    State(state): State<AppState>,
    Json(req): Json<ImportImageRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "importImage",
        serde_json::json!({
            "parentPath": req.parent_path,
            "name": req.name,
            "kind": req.kind,
            "width": req.width,
            "height": req.height,
            "uiWidth": req.ui_width,
            "uiHeight": req.ui_height,
            "positionX": req.position_x,
            "positionY": req.position_y,
            "pixelsBase64": req.pixels_base64,
            "sourceId": req.source_id
        }),
        120,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn import_uploaded(
    State(state): State<AppState>,
    Json(req): Json<ImportUploadedRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "importUploaded",
        serde_json::json!({
            "parentPath": req.parent_path,
            "kind": req.kind,
            "name": req.name,
            "assetId": req.asset_id,
            "uiKind": req.ui_kind,
            "uiWidth": req.ui_width,
            "uiHeight": req.ui_height,
            "positionX": req.position_x,
            "positionY": req.position_y,
            "volume": req.volume,
            "playbackSpeed": req.playback_speed,
            "looped": req.looped,
            "sourceId": req.source_id
        }),
        60,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn import_ui_pack(
    State(state): State<AppState>,
    Json(req): Json<ImportUiPackRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "importUiPack",
        serde_json::json!({
            "parentPath": req.parent_path,
            "name": req.name,
            "elements": req.elements,
            "sourceId": req.source_id
        }),
        180,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn import_audio(
    State(state): State<AppState>,
    Json(req): Json<ImportAudioRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "importAudio",
        serde_json::json!({
            "parentPath": req.parent_path,
            "sounds": req.sounds,
            "sourceId": req.source_id
        }),
        60,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn validate(
    State(state): State<AppState>,
    Json(req): Json<ValidateRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "validate",
        serde_json::json!({ "path": req.path, "rules": req.rules }),
        60,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn repair_tool(
    State(state): State<AppState>,
    Json(req): Json<RepairToolRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "repairTool",
        serde_json::json!({
            "path": req.path,
            "handle": req.handle,
            "dryRun": req.dry_run,
            "replaceBroken": req.replace_broken,
            "physicsFix": req.physics_fix,
            "collision": req.collision,
            "massless": req.massless
        }),
        60,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn snapshot(
    State(state): State<AppState>,
    Json(req): Json<SnapshotRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "snapshot",
        serde_json::json!({ "path": req.path, "includePaths": req.include_paths }),
        60,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn create_instance(
    State(state): State<AppState>,
    Json(req): Json<CreateInstanceRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "create",
        serde_json::json!({
            "parentPath": req.parent_path,
            "className": req.class_name,
            "name": req.name,
            "properties": req.properties
        }),
        60,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn upsert_files(
    State(state): State<AppState>,
    Json(req): Json<UpsertFilesRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "upsertFiles",
        serde_json::json!({
            "parentPath": req.parent_path,
            "dryRun": req.dry_run,
            "delete": req.delete,
            "items": req.items,
            "force": req.force,
            "sourceId": req.source_id
        }),
        120,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn apply_plan(
    State(state): State<AppState>,
    Json(req): Json<ApplyPlanRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "applyPlan",
        serde_json::json!({
            "rootPath": req.root_path,
            "plan": req.plan,
            "dryRun": req.dry_run,
            "approved": req.approved,
            "force": req.force,
            "only": req.only,
            "exclude": req.exclude
        }),
        180,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn package_update(
    State(state): State<AppState>,
    Json(req): Json<PackageUpdateRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "packageUpdate",
        serde_json::json!({
            "parentPath": req.parent_path,
            "blob": req.blob,
            "packageId": req.package_id,
            "mode": req.mode,
            "dryRun": req.dry_run,
            "force": req.force
        }),
        240,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn history(
    State(state): State<AppState>,
    Json(req): Json<HistoryRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "history",
        serde_json::json!({
            "action": req.action,
            "commandId": req.command_id
        }),
        120,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn deps(
    State(state): State<AppState>,
    Json(req): Json<DepsRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "deps",
        serde_json::json!({ "path": req.path }),
        90,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn serialize(
    State(state): State<AppState>,
    Json(req): Json<SerializeRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "serialize",
        serde_json::json!({ "path": req.path }),
        120,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn deserialize(
    State(state): State<AppState>,
    Json(req): Json<DeserializeRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "deserialize",
        serde_json::json!({
            "parentPath": req.parent_path,
            "blob": req.blob,
            "conflictMode": req.conflict_mode,
            "dryRun": req.dry_run,
            "rollbackOnError": req.rollback_on_error,
            "packageId": req.package_id
        }),
        180,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn autopilot_review(
    State(state): State<AppState>,
    Json(req): Json<AutopilotReviewRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "autopilotReview",
        serde_json::json!({
            "action": req.action,
            "run": req.run
        }),
        30,
    )
    .await
    {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn transfer(
    State(state): State<AppState>,
    Json(req): Json<TransferRequest>,
) -> Json<Envelope<serde_json::Value>> {
    match run_transfer(&state.registry, req).await {
        Ok(value) => Json(Envelope::ok(value)),
        Err(err) => Json(Envelope::err(err.to_string(), err.bridge_code())),
    }
}

async fn shutdown(State(state): State<AppState>) -> Json<Envelope<serde_json::Value>> {
    if let Some(tx) = state.shutdown.lock().await.take() {
        let _ = tx.send(());
    }
    Json(Envelope::ok(serde_json::json!({ "stopping": true })))
}

async fn run_plugin_command(
    registry: &Registry,
    studio: Option<&str>,
    kind: &str,
    mut payload: serde_json::Value,
    timeout_secs: u64,
) -> AppResult<serde_json::Value> {
    let token = registry.resolve_token(studio).await?;
    registry
        .ensure_protocol(&token, PLUGIN_PROTOCOL_VERSION)
        .await?;
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "_rs".into(),
            serde_json::json!({
                "cliVersion": CLI_VERSION,
                "protocolVersion": PLUGIN_PROTOCOL_VERSION
            }),
        );
    }
    let rx = registry.enqueue(&token, kind, payload).await?;
    let result = tokio::time::timeout(Duration::from_secs(timeout_secs), rx)
        .await
        .map_err(|_| AppError::CommandTimeout {
            timeout_ms: timeout_secs * 1000,
        })?
        .map_err(|_| AppError::Other("plugin dropped the result channel".into()))?;

    if !result.ok {
        return Err(AppError::PluginError(
            result.error.unwrap_or_else(|| "unknown error".into()),
        ));
    }
    Ok(result.data.unwrap_or_else(|| serde_json::json!(null)))
}

#[cfg(test)]
mod tests {
    use super::bridge_auth_matches;
    use crate::bridge::auth::TOKEN_HEADER;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn bridge_auth_matches_expected_token_only() {
        let mut headers = HeaderMap::new();
        assert!(!bridge_auth_matches(&headers, "secret"));

        headers.insert(TOKEN_HEADER, HeaderValue::from_static("wrong"));
        assert!(!bridge_auth_matches(&headers, "secret"));

        headers.insert(TOKEN_HEADER, HeaderValue::from_static("secret"));
        assert!(bridge_auth_matches(&headers, "secret"));
    }
}
