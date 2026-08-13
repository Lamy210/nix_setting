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

    #[test]
    fn save_and_load_to_file() {
        let dir = std::env::temp_dir().join("schneeforge-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");

        let s = State {
            host: Some("linux".to_string()),
            applied_revision: Some("def456".to_string()),
            applied_at: None,
            product_version: Some("0.1.0".to_string()),
        };
        s.save(&path).unwrap();

        let loaded = State::load(&path).unwrap();
        assert_eq!(loaded.host.as_deref(), Some("linux"));
        assert_eq!(loaded.applied_revision.as_deref(), Some("def456"));
        assert_eq!(loaded.applied_at, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_file_returns_none() {
        let path = std::env::temp_dir().join("schneeforge-nonexistent.json");
        assert!(State::load(&path).is_none());
    }

    #[test]
    fn default_state_is_empty() {
        let s = State::default();
        assert!(s.host.is_none());
        assert!(s.applied_revision.is_none());
        assert!(s.applied_at.is_none());
        assert!(s.product_version.is_none());
    }
}
