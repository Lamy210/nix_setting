//! profile 選択の解決と flake への注入 (v2 §17)。
//!
//! 選択状態は state (`profile: Option<String>`) が持ち、未選択時は
//! distribution manifest の `profiles.default` を使う。repo は書き換えず、
//! 選択名を `{ profile = "<name>"; }` として state dir へ生成し
//! `--override-input profile <path>` で差し替える (machine input と同じ
//! pattern)。実際の `profiles/<name>.nix` 解決は flake 側
//! (modules/profile-input.nix) が行う。

use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::state::{State, StateStore};

/// profile input (`profile.nix`) の既定 path
pub fn default_profile_nix_path() -> std::path::PathBuf {
    crate::machine::state_dir().join("profile.nix")
}

/// 選択 profile を解決する。優先順位:
/// 1. state の `profile` (manifest の available に含まれること)
/// 2. manifest の `profiles.default`
///
/// 戻り値は (profile 名, state 由来か)
pub fn resolve(repo: &str) -> Result<(String, bool)> {
    resolve_with(repo, &StateStore::default())
}

/// [`resolve`] の state store 注入版 (test 用)
pub fn resolve_with(repo: &str, store: &StateStore) -> Result<(String, bool)> {
    let manifest = Manifest::load(repo)?;
    let default = manifest
        .profiles
        .default
        .clone()
        .ok_or_else(|| Error::Manifest("profiles.default is not set".to_string()))?;
    let selected = store.load().and_then(|s| s.profile);
    match selected {
        Some(name) => {
            if manifest.profiles.available.contains(&name) {
                Ok((name, true))
            } else {
                Err(Error::Manifest(format!(
                    "selected profile '{name}' is not in manifest profiles.available"
                )))
            }
        }
        None => Ok((default, false)),
    }
}

/// state の profile 選択を保存する (manifest 検証済みであること)
pub fn save_selection(name: &str) -> Result<()> {
    let store = StateStore::default();
    save_selection_with(&store, name)
}

/// [`save_selection`] の state store 注入版 (test 用)
pub fn save_selection_with(store: &StateStore, name: &str) -> Result<()> {
    let mut state: State = store.load().unwrap_or_default();
    state.profile = Some(name.to_string());
    store.save(&state)
}

/// state の profile 選択を解除する (manifest default へ戻す)
pub fn clear_selection() -> Result<()> {
    let store = StateStore::default();
    let mut state: State = store.load().unwrap_or_default();
    state.profile = None;
    store.save(&state)
}

/// profile input (`profile.nix`) を生成する。常に上書き
pub fn write_profile_input(name: &str) -> Result<std::path::PathBuf> {
    let path = default_profile_nix_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Io(format!("create profile input dir: {e}")))?;
    }
    std::fs::write(&path, format!("{{ profile = \"{name}\"; }}\n"))
        .map_err(|e| Error::Io(format!("write profile input: {e}")))?;
    Ok(path)
}

/// machine input と profile input の `--override-input` 引数を返す。
/// apply / plan の評価前に必ず呼ばれる。
///
/// file を指す path input の override は `path:<abs>` URL 形式が必須
/// (nix 2.35 は bare の絶対 path を flake ref として解釈し
/// "not a flake (because it's not a directory)" で拒否する)
pub fn override_args(repo: &str) -> Result<Vec<String>> {
    let facts = crate::machine::MachineFacts::detect()?;
    let machine_path = crate::machine::write_machine_input(&facts)?;
    let (profile, _) = resolve(repo)?;
    let profile_path = write_profile_input(&profile)?;
    Ok(vec![
        "--override-input".to_string(),
        "machine".to_string(),
        format!("path:{}", machine_path.to_string_lossy()),
        "--override-input".to_string(),
        "profile".to_string(),
        format!("path:{}", profile_path.to_string_lossy()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_repo(name: &str, manifest: &str) -> String {
        let dir = std::env::temp_dir().join(format!("schneeforge-profile-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("schneeforge.toml"), manifest).unwrap();
        dir.to_string_lossy().to_string()
    }

    /// test 毎に独立した state store を作る (env var 経由だと並列 test で競合する)
    fn setup_store(name: &str, profile: Option<&str>) -> StateStore {
        let dir = std::env::temp_dir().join(format!("schneeforge-profile-state-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = StateStore::new(dir.join("state.json"));
        if let Some(p) = profile {
            save_selection_with(&store, p).unwrap();
        }
        store
    }

    const MANIFEST: &str = r#"
schema = 1
[profiles]
default = "developer"
available = ["minimal", "developer"]
[systems]
x86_64-linux = true
"#;

    #[test]
    fn resolve_uses_manifest_default_when_unselected() {
        let store = setup_store("default", None);
        let repo = setup_repo("default", MANIFEST);
        let (name, from_state) = resolve_with(&repo, &store).unwrap();
        assert_eq!(name, "developer");
        assert!(!from_state);
    }

    #[test]
    fn resolve_uses_state_selection() {
        let store = setup_store("selected", Some("minimal"));
        let repo = setup_repo("selected", MANIFEST);
        let (name, from_state) = resolve_with(&repo, &store).unwrap();
        assert_eq!(name, "minimal");
        assert!(from_state);
    }

    #[test]
    fn resolve_rejects_profile_not_in_available() {
        let store = setup_store("invalid", Some("unknown-profile"));
        let repo = setup_repo("invalid", MANIFEST);
        let err = resolve_with(&repo, &store).unwrap_err();
        assert!(err.to_string().contains("not in manifest"));
    }

    #[test]
    fn resolve_fails_without_manifest() {
        let store = setup_store("nomanifest", None);
        let dir = std::env::temp_dir().join("schneeforge-profile-test-empty");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(resolve_with(dir.to_string_lossy().as_ref(), &store).is_err());
    }

    #[test]
    fn save_and_clear_selection_roundtrip() {
        let store = setup_store("roundtrip", None);
        save_selection_with(&store, "minimal").unwrap();
        assert_eq!(store.load().unwrap().profile.as_deref(), Some("minimal"));
        let mut state = store.load().unwrap();
        state.profile = None;
        store.save(&state).unwrap();
        assert_eq!(store.load().unwrap().profile, None);
    }

    #[test]
    fn write_profile_input_contains_name() {
        let path = write_profile_input("minimal").unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "{ profile = \"minimal\"; }\n");
    }
}
