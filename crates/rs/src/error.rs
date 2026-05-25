use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("bridge unreachable at {url}: {source}")]
    BridgeUnreachable {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("couldn't start rs-bridge on port {port} within 8s. Check {log_path} for details.")]
    BridgeStartFailed { port: u16, log_path: String },

    #[error("studio '{name}' is not registered with the bridge. Open Studio and verify the plugin is installed.")]
    StudioNotConnected { name: String },

    #[error("ambiguous studio name '{name}'; candidates: {candidates:?}")]
    StudioAmbiguous {
        name: String,
        candidates: Vec<String>,
    },

    #[error("plugin error: {0}")]
    PluginError(String),

    #[error("plugin protocol mismatch for studio '{studio}': plugin has {actual}, CLI expects {expected}. Restart Studio after installing the rebuilt rs plugin.")]
    PluginProtocolMismatch {
        studio: String,
        expected: u32,
        actual: String,
    },

    #[error("command timed out after {timeout_ms}ms")]
    CommandTimeout { timeout_ms: u64 },

    #[error("{operation} failed: {error} (code: {code})")]
    BridgeResponse {
        operation: String,
        error: String,
        code: String,
    },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),

    #[error("{message}")]
    Silent { exit_code: i32, message: String },
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Silent { exit_code, .. } => *exit_code,
            AppError::BridgeUnreachable { .. } | AppError::BridgeStartFailed { .. } => 2,
            AppError::StudioNotConnected { .. } | AppError::StudioAmbiguous { .. } => 3,
            AppError::PluginError(_) | AppError::PluginProtocolMismatch { .. } => 4,
            AppError::CommandTimeout { .. } => 5,
            AppError::BridgeResponse { code, .. } => match code.as_str() {
                "bridge-unreachable" => 2,
                "studio-not-found" | "studio-ambiguous" => 3,
                "plugin-error" | "plugin-protocol-mismatch" => 4,
                "bridge-unresponsive" | "command-timeout" => 5,
                _ => 1,
            },
            _ => 1,
        }
    }

    pub fn bridge_code(&self) -> &'static str {
        match self {
            AppError::BridgeUnreachable { .. } | AppError::BridgeStartFailed { .. } => {
                "bridge-unreachable"
            }
            AppError::StudioNotConnected { .. } => "studio-not-found",
            AppError::StudioAmbiguous { .. } => "studio-ambiguous",
            AppError::PluginError(_) => "plugin-error",
            AppError::PluginProtocolMismatch { .. } => "plugin-protocol-mismatch",
            AppError::CommandTimeout { .. } => "command-timeout",
            AppError::BridgeResponse { code, .. } => match code.as_str() {
                "bridge-unreachable" => "bridge-unreachable",
                "studio-not-found" => "studio-not-found",
                "studio-ambiguous" => "studio-ambiguous",
                "plugin-error" => "plugin-error",
                "bridge-unresponsive" => "bridge-unresponsive",
                "command-timeout" => "command-timeout",
                _ => "internal",
            },
            AppError::Serde(_) => "bad-json",
            AppError::Io(_) => "io",
            AppError::Http(_) => "http",
            AppError::Silent { .. } => "internal",
            AppError::Other(_) => "internal",
        }
    }

    pub fn is_silent(&self) -> bool {
        matches!(self, AppError::Silent { .. })
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn bridge_response_exit_code_matches_protocol_code() {
        let err = AppError::BridgeResponse {
            operation: "exec".into(),
            error: "missing".into(),
            code: "studio-not-found".into(),
        };
        assert_eq!(err.exit_code(), 3);

        let err = AppError::BridgeResponse {
            operation: "exec".into(),
            error: "plugin failed".into(),
            code: "plugin-error".into(),
        };
        assert_eq!(err.exit_code(), 4);

        let err = AppError::BridgeResponse {
            operation: "exec".into(),
            error: "slow".into(),
            code: "command-timeout".into(),
        };
        assert_eq!(err.exit_code(), 5);
    }
}
