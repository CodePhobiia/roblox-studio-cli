use crate::error::{AppError, AppResult};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) fn slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "run".into()
    } else {
        trimmed.chars().take(48).collect()
    }
}

pub(super) fn safe_join(root: &Path, relative: &str) -> AppResult<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err(AppError::Other(format!(
            "autopilot artifact path must be relative: {relative}"
        )));
    }
    let mut out = root.to_path_buf();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::CurDir => {}
            _ => {
                return Err(AppError::Other(format!(
                    "autopilot artifact path escapes run directory: {relative}"
                )))
            }
        }
    }
    Ok(out)
}

pub(super) fn redact_json(value: &Value) -> Value {
    match value {
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            if lower.contains("api_key")
                || lower.contains("apikey")
                || lower.contains("token")
                || lower.contains("secret")
                || lower.contains("roblox_api_key")
            {
                Value::String("<redacted>".into())
            } else {
                Value::String(text.clone())
            }
        }
        Value::Array(values) => Value::Array(values.iter().map(redact_json).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let lower = key.to_ascii_lowercase();
                    if lower.contains("api_key")
                        || lower.contains("apikey")
                        || lower.contains("token")
                        || lower.contains("secret")
                    {
                        (key.clone(), Value::String("<redacted>".into()))
                    } else {
                        (key.clone(), redact_json(value))
                    }
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

pub(super) fn redact_text(text: &str) -> String {
    match redact_json(&Value::String(text.to_string())) {
        Value::String(value) => value,
        _ => "<redacted>".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn slug_normalizes_ascii_and_falls_back() {
        assert_eq!(slug("Starter Shop!"), "starter-shop");
        assert_eq!(slug("..."), "run");
        assert_eq!(
            slug("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuv"
        );
    }

    #[test]
    fn safe_join_keeps_artifacts_under_root() {
        let root = PathBuf::from("run");
        assert_eq!(
            safe_join(&root, "generated/Foo.server.lua").unwrap(),
            root.join("generated").join("Foo.server.lua")
        );
        assert!(safe_join(&root, "../outside.lua").is_err());
        assert!(safe_join(&root, "generated/../outside.lua").is_err());
    }

    #[test]
    fn redact_json_covers_secretish_keys_and_values() {
        let redacted = redact_json(&json!({
            "apiKey": "abc",
            "nested": ["normal", "token=abc"],
            "prompt": "starter shop"
        }));
        assert_eq!(redacted["apiKey"], "<redacted>");
        assert_eq!(redacted["nested"][0], "normal");
        assert_eq!(redacted["nested"][1], "<redacted>");
        assert_eq!(redacted["prompt"], "starter shop");
        assert_eq!(redact_text("plain note"), "plain note");
        assert_eq!(redact_text("roblox_api_key=abc"), "<redacted>");
    }
}
