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
    pub last_heartbeat: Instant,
    pub pending_commands: VecDeque<PluginCommand>,
    pub pending_results: HashMap<String, oneshot::Sender<CommandResult>>,
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
            if let Some((token, _)) = map.iter().find(|(_, s)| s.id == name) {
                return Ok(token.clone());
            }
            if let Some((token, _)) = map.iter().find(|(_, s)| s.name == name) {
                return Ok(token.clone());
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

    pub async fn enqueue(
        &self,
        session_token: &str,
        kind: impl Into<String>,
        payload: serde_json::Value,
    ) -> AppResult<oneshot::Receiver<CommandResult>> {
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
        session.pending_results.insert(command_id, tx);
        drop(map);
        self.notify.notify_waiters();
        Ok(rx)
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
        })
        .await;
        reg.register(RegisterRequest {
            id: "B".into(),
            name: "Project Beta".into(),
            place_file_path: None,
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
}
