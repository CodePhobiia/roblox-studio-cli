use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::Envelope;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

pub(crate) fn post<TReq, TResp>(
    port: u16,
    operation: &'static str,
    path: &str,
    request: &TReq,
    timeout_secs: u64,
) -> AppResult<TResp>
where
    TReq: Serialize,
    TResp: DeserializeOwned,
{
    ensure_bridge_running(port)?;
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;
    let resp = crate::bridge::auth::attach_blocking(client.post(&url))?
        .json(request)
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    let env: Envelope<TResp> = resp.json()?;
    if !env.ok {
        return Err(crate::cli::envelope_error(operation, env.error, env.code));
    }
    env.data
        .ok_or_else(|| AppError::Other(format!("{operation} returned no data")))
}
