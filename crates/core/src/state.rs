use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

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
}

/// State の原子的な読み書き (temp → fsync → rename)
#[derive(Debug, Clone)]
pub struct StateStore {
    path: PathBuf,
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new(State::default_path())
    }
}

impl StateStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Option<State> {
        let content = std::fs::read_to_string(&self.path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// 原子的に保存する (temp 書き込み → fsync → rename)。失敗時はエラーを返す
    pub fn save(&self, state: &State) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Io(format!("create_dir: {e}")))?;
        }
        let content = serde_json::to_string_pretty(state)
            .map_err(|e| Error::Io(format!("serialize: {e}")))?;
        let tmp = self.tmp_path();
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp)
                .map_err(|e| Error::Io(format!("create tmp {}: {e}", tmp.display())))?;
            f.write_all(content.as_bytes())
                .map_err(|e| Error::Io(format!("write tmp {}: {e}", tmp.display())))?;
            f.sync_all()
                .map_err(|e| Error::Io(format!("fsync tmp {}: {e}", tmp.display())))?;
        }
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            Error::Io(format!(
                "rename {} -> {}: {e}",
                tmp.display(),
                self.path.display()
            ))
        })?;
        Ok(())
    }

    /// プロセス ID を含む一時ファイルパス (プロセス間の tmp 衝突を避ける)
    fn tmp_path(&self) -> PathBuf {
        let file_name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "state.json".to_string());
        self.path
            .with_file_name(format!("{file_name}.{}.tmp", std::process::id()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> (StateStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("schneeforge-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let store = StateStore::new(dir.join("state.json"));
        (store, dir)
    }

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
        let (store, dir) = temp_store("roundtrip");
        let s = State {
            host: Some("linux".to_string()),
            applied_revision: Some("def456".to_string()),
            applied_at: None,
            product_version: Some("0.1.0".to_string()),
        };
        store.save(&s).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.host.as_deref(), Some("linux"));
        assert_eq!(loaded.applied_revision.as_deref(), Some("def456"));
        assert_eq!(loaded.applied_at, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_file_returns_none() {
        let store =
            StateStore::new(std::env::temp_dir().join("schneeforge-nonexistent-state.json"));
        assert!(store.load().is_none());
    }

    #[test]
    fn save_creates_parent_dirs() {
        let (_store, dir) = temp_store("nested");
        // parent が存在しないネストパス
        let nested = StateStore::new(dir.join("a/b/state.json"));
        let s = State::default();
        nested.save(&s).unwrap();
        assert!(nested.path().exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_leaves_no_tmp_file() {
        let (store, dir) = temp_store("tmp-cleanup");
        store.save(&State::default()).unwrap();
        assert!(!store.tmp_path().exists());
        let _ = std::fs::remove_dir_all(&dir);
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
