use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 適用状態を記録する State
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub host: Option<String>,
    pub applied_revision: Option<String>,
    pub applied_at: Option<String>,
    pub product_version: Option<String>,
}

impl State {
    /// 既定の state ファイルパス (~/.local/state/schneeforge/state.json)
    pub fn default_path() -> PathBuf {
        let base = std::env::var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local/state")))
            .unwrap_or_else(|_| PathBuf::from("."));
        base.join("schneeforge").join("state.json")
    }

    pub fn load(path: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_state() {
        let s = State {
            host: Some("macbook-air".to_string()),
            applied_revision: Some("abc123".to_string()),
            applied_at: Some("2026-08-13".to_string()),
            product_version: Some("0.1.0".to_string()),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: State = serde_json::from_str(&json).unwrap();
        assert_eq!(back.host.as_deref(), Some("macbook-air"));
        assert_eq!(back.applied_revision.as_deref(), Some("abc123"));
    }
}
