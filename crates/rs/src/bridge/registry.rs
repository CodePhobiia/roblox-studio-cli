use crate::error::{AppError, AppResult};
use crate::protocol::messages::{CommandResult, PluginCommand, RegisterRequest, StudioInfo};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, Notify, RwLock};
use uuid::Uuid;

const SESSION_TTL: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct StudioSession {
    pub id: String,
    pub name: String,
    pub place_file_path: Option<String>,
    pub protocol_version: Option<u32>,
    pub plugin_version: Option<String>,
    pub capabilities: Vec<String>,
    pub last_heartbeat: Instant,
    pub pending_commands: VecDeque<PluginCommand>,
    pub pending_results: HashMap<String, oneshot::Sender<CommandResult>>,
}

pub struct QueuedCommand {
    pub command_id: String,
    pub result: oneshot::Receiver<CommandResult>,
}

#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<RwLock<HashMap<String, StudioSession>>>,
    notify: Arc<Notify>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn register(&self, req: RegisterRequest) -> String {
        let token = Uuid::new_v4().to_string();
        let mut map = self.inner.write().await;
        map.retain(|_, existing| existing.id != req.id);
        map.insert(
            token.clone(),
            StudioSession {
                id: req.id,
                name: req.name,
                place_file_path: req.place_file_path,
                protocol_version: req.protocol_version,
                plugin_version: req.plugin_version,
                capabilities: req.capabilities,
                last_heartbeat: Instant::now(),
                pending_commands: VecDeque::new(),
                pending_results: HashMap::new(),
            },
        );
        self.notify.notify_waiters();
        token
    }

    pub async fn heartbeat(&self, session_token: &str) -> AppResult<()> {
        self.expire_stale().await;
        let mut map = self.inner.write().await;
        match map.get_mut(session_token) {
            Some(sess) => {
                sess.last_heartbeat = Instant::now();
                Ok(())
            }
            None => Err(AppError::Other(format!(
                "unknown session token: {session_token}"
            ))),
        }
    }

    pub async fn list(&self) -> Vec<StudioInfo> {
        self.expire_stale().await;
        let map = self.inner.read().await;
        let now = Instant::now();
        let mut studios: Vec<_> = map
            .values()
            .map(|s| StudioInfo {
                id: s.id.clone(),
                name: s.name.clone(),
                place_file_path: s.place_file_path.clone(),
                protocol_version: s.protocol_version,
                plugin_version: s.plugin_version.clone(),
                capabilities: s.capabilities.clone(),
                last_heartbeat_ms_ago: now.duration_since(s.last_heartbeat).as_millis() as u64,
            })
            .collect();
        studios.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        studios
    }

    pub async fn resolve_token(&self, requested: Option<&str>) -> AppResult<String> {
        self.expire_stale().await;
        let map = self.inner.read().await;

        if let Some(name) = requested.filter(|name| !name.trim().is_empty()) {
            let id_matches: Vec<_> = map
                .iter()
                .filter(|(_, s)| s.id == name)
                .map(|(token, s)| (token.clone(), s.clone_info()))
                .collect();
            match id_matches.as_slice() {
                [(token, _)] => return Ok(token.clone()),
                [] => {}
                many => {
                    return Err(AppError::StudioAmbiguous {
                        name: name.to_string(),
                        candidates: many
                            .iter()
                            .map(|(_, info)| format!("{} ({})", info.name, info.id))
                            .collect(),
                    })
                }
            }

            let exact_name_matches: Vec<_> = map
                .iter()
                .filter(|(_, s)| s.name == name)
                .map(|(token, s)| (token.clone(), s.clone_info()))
                .collect();
            match exact_name_matches.as_slice() {
                [(token, _)] => return Ok(token.clone()),
                [] => {}
                many => {
                    return Err(AppError::StudioAmbiguous {
                        name: name.to_string(),
                        candidates: many
                            .iter()
                            .map(|(_, info)| format!("{} ({})", info.name, info.id))
                            .collect(),
                    })
                }
            }

            let needle = name.to_lowercase();
            let matches: Vec<_> = map
                .iter()
                .filter(|(_, s)| s.name.to_lowercase().contains(&needle))
                .map(|(token, s)| (token.clone(), s.clone_info()))
                .collect();

            return match matches.as_slice() {
                [(token, _)] => Ok(token.clone()),
                [] => Err(AppError::StudioNotConnected {
                    name: name.to_string(),
                }),
                many => Err(AppError::StudioAmbiguous {
                    name: name.to_string(),
                    candidates: many
                        .iter()
                        .map(|(_, info)| format!("{} ({})", info.name, info.id))
                        .collect(),
                }),
            };
        }

        match map.len() {
            1 => Ok(map.keys().next().expect("len checked").clone()),
            0 => Err(AppError::StudioNotConnected {
                name: "<default>".to_string(),
            }),
            _ => Err(AppError::StudioAmbiguous {
                name: "<default>".to_string(),
                candidates: map
                    .values()
                    .map(|s| format!("{} ({})", s.name, s.id))
                    .collect(),
            }),
        }
    }

    #[cfg(test)]
    pub async fn enqueue(
        &self,
        session_token: &str,
        kind: impl Into<String>,
        payload: serde_json::Value,
    ) -> AppResult<oneshot::Receiver<CommandResult>> {
        Ok(self
            .enqueue_command(session_token, kind, payload)
            .await?
            .result)
    }

    pub async fn enqueue_command(
        &self,
        session_token: &str,
        kind: impl Into<String>,
        payload: serde_json::Value,
    ) -> AppResult<QueuedCommand> {
        self.expire_stale().await;
        let command_id = Uuid::new_v4().to_string();
        let command = PluginCommand {
            command_id: command_id.clone(),
            kind: kind.into(),
            payload,
        };
        let (tx, rx) = oneshot::channel();

        let mut map = self.inner.write().await;
        let session = map
            .get_mut(session_token)
            .ok_or_else(|| AppError::StudioNotConnected {
                name: session_token.to_string(),
            })?;
        session.pending_commands.push_back(command);
        session.pending_results.insert(command_id.clone(), tx);
        drop(map);
        self.notify.notify_waiters();
        Ok(QueuedCommand {
            command_id,
            result: rx,
        })
    }

    pub async fn ensure_protocol(&self, session_token: &str, expected: u32) -> AppResult<()> {
        self.expire_stale().await;
        let map = self.inner.read().await;
        let session = map
            .get(session_token)
            .ok_or_else(|| AppError::StudioNotConnected {
                name: session_token.to_string(),
            })?;
        if session.protocol_version == Some(expected) {
            return Ok(());
        }
        Err(AppError::PluginProtocolMismatch {
            studio: session.name.clone(),
            expected,
            actual: session
                .protocol_version
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".into()),
        })
    }

    pub async fn ensure_capability(&self, session_token: &str, capability: &str) -> AppResult<()> {
        self.expire_stale().await;
        let map = self.inner.read().await;
        let session = map
            .get(session_token)
            .ok_or_else(|| AppError::StudioNotConnected {
                name: session_token.to_string(),
            })?;
        if session
            .capabilities
            .iter()
            .any(|advertised| advertised == capability)
        {
            return Ok(());
        }
        Err(AppError::PluginCapabilityMissing {
            studio: session.name.clone(),
            command: capability.to_string(),
            capabilities: session.capabilities.clone(),
        })
    }

    pub async fn poll(
        &self,
        session_token: &str,
        timeout: Duration,
    ) -> AppResult<Option<PluginCommand>> {
        let deadline = Instant::now() + timeout;

        loop {
            self.expire_stale().await;
            {
                let mut map = self.inner.write().await;
                let session = map
                    .get_mut(session_token)
                    .ok_or_else(|| AppError::Other("unknown session token".into()))?;
                session.last_heartbeat = Instant::now();
                if let Some(command) = session.pending_commands.pop_front() {
                    return Ok(Some(command));
                }
            }

            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let remaining = deadline.saturating_duration_since(now);
            let _ = tokio::time::timeout(
                remaining.min(Duration::from_secs(1)),
                self.notify.notified(),
            )
            .await;
        }
    }

    pub async fn submit_result(&self, command_id: &str, result: CommandResult) -> AppResult<()> {
        let mut map = self.inner.write().await;
        for session in map.values_mut() {
            if let Some(tx) = session.pending_results.remove(command_id) {
                let _ = tx.send(result);
                return Ok(());
            }
        }
        Err(AppError::Other(format!("unknown command id: {command_id}")))
    }

    pub async fn cancel_pending_command(&self, command_id: &str) -> bool {
        let mut map = self.inner.write().await;
        let mut removed = false;
        for session in map.values_mut() {
            let before = session.pending_commands.len();
            session
                .pending_commands
                .retain(|command| command.command_id != command_id);
            removed |= session.pending_commands.len() != before;
            removed |= session.pending_results.remove(command_id).is_some();
        }
        removed
    }

    async fn expire_stale(&self) {
        let mut map = self.inner.write().await;
        let now = Instant::now();
        map.retain(|_, session| now.duration_since(session.last_heartbeat) <= SESSION_TTL);
    }
}

impl StudioSession {
    fn clone_info(&self) -> StudioInfo {
        StudioInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            place_file_path: self.place_file_path.clone(),
            protocol_version: self.protocol_version,
            plugin_version: self.plugin_version.clone(),
            capabilities: self.capabilities.clone(),
            last_heartbeat_ms_ago: Instant::now()
                .duration_since(self.last_heartbeat)
                .as_millis() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::messages::RegisterRequest;

    #[tokio::test]
    async fn register_then_list_returns_one_studio() {
        let reg = Registry::new();
        let token = reg
            .register(RegisterRequest {
                id: "stud-A".into(),
                name: "Snipe a Slime!".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;

        let list = reg.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Snipe a Slime!");
        assert_eq!(list[0].id, "stud-A");
        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn resolve_by_exact_name_returns_one() {
        let reg = Registry::new();
        let expected = reg
            .register(RegisterRequest {
                id: "stud-A".into(),
                name: "Snipe a Slime!".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;
        let resolved = reg.resolve_token(Some("Snipe a Slime!")).await.unwrap();
        assert_eq!(resolved, expected);
    }

    #[tokio::test]
    async fn resolve_by_substring_returns_one_if_unique() {
        let reg = Registry::new();
        let expected = reg
            .register(RegisterRequest {
                id: "stud-A".into(),
                name: "Snipe a Slime!".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;
        let resolved = reg.resolve_token(Some("slime")).await.unwrap();
        assert_eq!(resolved, expected);
    }

    #[tokio::test]
    async fn resolve_ambiguous_returns_err() {
        let reg = Registry::new();
        reg.register(RegisterRequest {
            id: "A".into(),
            name: "Project Alpha".into(),
            place_file_path: None,
            protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
            plugin_version: Some("0.1.0".into()),
            capabilities: vec![],
        })
        .await;
        reg.register(RegisterRequest {
            id: "B".into(),
            name: "Project Beta".into(),
            place_file_path: None,
            protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
            plugin_version: Some("0.1.0".into()),
            capabilities: vec![],
        })
        .await;
        let result = reg.resolve_token(Some("Project")).await;
        assert!(matches!(result, Err(AppError::StudioAmbiguous { .. })));
    }

    #[tokio::test]
    async fn resolve_duplicate_exact_name_returns_err() {
        let reg = Registry::new();
        reg.register(RegisterRequest {
            id: "A".into(),
            name: "Project".into(),
            place_file_path: None,
            protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
            plugin_version: Some("0.1.0".into()),
            capabilities: vec![],
        })
        .await;
        reg.register(RegisterRequest {
            id: "B".into(),
            name: "Project".into(),
            place_file_path: None,
            protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
            plugin_version: Some("0.1.0".into()),
            capabilities: vec![],
        })
        .await;

        let result = reg.resolve_token(Some("Project")).await;
        assert!(matches!(result, Err(AppError::StudioAmbiguous { .. })));
    }

    #[tokio::test]
    async fn enqueue_poll_result_roundtrips() {
        let reg = Registry::new();
        let token = reg
            .register(RegisterRequest {
                id: "A".into(),
                name: "Project".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;
        let rx = reg
            .enqueue(&token, "exec", serde_json::json!({"lua": "return 1"}))
            .await
            .unwrap();
        let command = reg
            .poll(&token, Duration::from_millis(1))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(command.kind, "exec");
        reg.submit_result(
            &command.command_id,
            CommandResult {
                ok: true,
                data: Some(serde_json::json!(1)),
                error: None,
            },
        )
        .await
        .unwrap();
        let result = rx.await.unwrap();
        assert!(result.ok);
        assert_eq!(result.data.unwrap(), serde_json::json!(1));
    }

    #[tokio::test]
    async fn ensure_protocol_rejects_mismatch() {
        let reg = Registry::new();
        let token = reg
            .register(RegisterRequest {
                id: "A".into(),
                name: "Project".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION - 1),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec!["deserialize".into()],
            })
            .await;

        let err = reg
            .ensure_protocol(&token, crate::protocol::messages::PLUGIN_PROTOCOL_VERSION)
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            AppError::PluginProtocolMismatch {
                studio,
                expected: crate::protocol::messages::PLUGIN_PROTOCOL_VERSION,
                ..
            } if studio == "Project"
        ));
    }

    #[tokio::test]
    async fn ensure_capability_rejects_missing_command_capability() {
        let reg = Registry::new();
        let token = reg
            .register(RegisterRequest {
                id: "A".into(),
                name: "Project".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec!["serialize".into()],
            })
            .await;

        reg.ensure_capability(&token, "serialize").await.unwrap();
        let err = reg
            .ensure_capability(&token, "deserialize")
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            AppError::PluginCapabilityMissing {
                studio,
                command,
                ..
            } if studio == "Project" && command == "deserialize"
        ));
    }

    #[tokio::test]
    async fn queue_polls_commands_fifo() {
        let reg = Registry::new();
        let token = reg
            .register(RegisterRequest {
                id: "A".into(),
                name: "Project".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;

        let _first = reg
            .enqueue_command(&token, "read", serde_json::json!({"path": "Workspace"}))
            .await
            .unwrap();
        let _second = reg
            .enqueue_command(&token, "validate", serde_json::json!({"path": "Workspace"}))
            .await
            .unwrap();

        let first = reg
            .poll(&token, Duration::from_millis(1))
            .await
            .unwrap()
            .unwrap();
        let second = reg
            .poll(&token, Duration::from_millis(1))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(first.kind, "read");
        assert_eq!(second.kind, "validate");
    }

    #[tokio::test]
    async fn queue_is_independent_per_session() {
        let reg = Registry::new();
        let token_a = reg
            .register(RegisterRequest {
                id: "A".into(),
                name: "Project A".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;
        let token_b = reg
            .register(RegisterRequest {
                id: "B".into(),
                name: "Project B".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;

        reg.enqueue_command(&token_a, "read", serde_json::json!({"path": "Workspace"}))
            .await
            .unwrap();

        assert!(reg
            .poll(&token_b, Duration::from_millis(1))
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            reg.poll(&token_a, Duration::from_millis(1))
                .await
                .unwrap()
                .unwrap()
                .kind,
            "read"
        );
    }

    #[tokio::test]
    async fn cancel_pending_command_removes_queue_and_result_sender() {
        let reg = Registry::new();
        let token = reg
            .register(RegisterRequest {
                id: "A".into(),
                name: "Project".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;
        let queued = reg
            .enqueue_command(&token, "read", serde_json::json!({"path": "Workspace"}))
            .await
            .unwrap();

        assert!(reg.cancel_pending_command(&queued.command_id).await);
        assert!(reg
            .poll(&token, Duration::from_millis(1))
            .await
            .unwrap()
            .is_none());
        assert!(queued.result.await.is_err());
    }

    #[tokio::test]
    async fn submit_result_rejects_unknown_command_id() {
        let reg = Registry::new();
        let err = reg
            .submit_result(
                "missing",
                CommandResult {
                    ok: true,
                    data: Some(serde_json::json!(null)),
                    error: None,
                },
            )
            .await
            .unwrap_err();

        assert!(err.to_string().contains("unknown command id: missing"));
    }

    #[tokio::test]
    async fn stale_sessions_expire_before_listing() {
        let reg = Registry::new();
        let token = reg
            .register(RegisterRequest {
                id: "A".into(),
                name: "Project".into(),
                place_file_path: None,
                protocol_version: Some(crate::protocol::messages::PLUGIN_PROTOCOL_VERSION),
                plugin_version: Some("0.1.0".into()),
                capabilities: vec![],
            })
            .await;
        {
            let mut map = reg.inner.write().await;
            map.get_mut(&token).unwrap().last_heartbeat =
                Instant::now() - SESSION_TTL - Duration::from_millis(1);
        }

        assert!(reg.list().await.is_empty());
    }
}
