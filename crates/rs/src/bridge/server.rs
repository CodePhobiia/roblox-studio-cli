use crate::bridge::orchestrator::run_transfer;
use crate::bridge::registry::Registry;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{
    CommandResult, Envelope, ExecRequest, ExportRequest, PluginCommand, ReadRequest,
    RegisterRequest, RegisterResponse, TransferRequest,
};
use axum::extract::{Path, State};
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
}

pub async fn serve(port: u16) -> AppResult<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let registry = Registry::new();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let state = AppState {
        registry,
        shutdown: Arc::new(Mutex::new(Some(shutdown_tx))),
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
    Router::new()
        .route("/healthz", get(healthz))
        .route("/register", post(register))
        .route("/poll/:session_token", get(poll))
        .route("/result/:command_id", post(result))
        .route("/heartbeat/:session_token", post(heartbeat))
        .route("/studios", get(studios))
        .route("/exec", post(exec))
        .route("/read", post(read))
        .route("/export", post(export))
        .route("/transfer", post(transfer))
        .route("/shutdown", post(shutdown))
        .with_state(state)
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
    match run_plugin_command(
        &state.registry,
        req.studio.as_deref(),
        "exec",
        serde_json::json!({ "lua": req.lua }),
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
    payload: serde_json::Value,
    timeout_secs: u64,
) -> AppResult<serde_json::Value> {
    let token = registry.resolve_token(studio).await?;
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
