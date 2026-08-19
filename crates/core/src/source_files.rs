//! managed source の repo file 読み取り (v2 §7)。
//!
//! managed source は local に source tree を持たないため、repo file
//! (`schneeforge.toml` 等) は `raw.githubusercontent.com` から tag pinned で
//! 取得し state dir (`sources/<tag>/`) へ原子保存する。tag は不変のため
//! 一度保存した cache は無期限に正しく、2 回目以降の読み取り
//! (offline 含む) は network を行わない。cache が無い状態での取得失敗
//! (offline 初回 / 404) は fail-closed に error を返す。
//! path source (checkout / Local) の file 読み取りは従来どおり local
//! filesystem を使う。

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::source::{github_slug, SourceState};
use crate::state::StateStore;

/// tag pinned の raw file URL
/// (`raw.githubusercontent.com/<owner>/<repo>/<tag>/<file>`)
pub fn raw_url(remote: &str, tag: &str, file: &str) -> Result<String> {
    let (owner, repo) = github_slug(remote).ok_or_else(|| {
        Error::Precondition(format!(
            "cannot resolve owner/repo from repository URL: {remote}"
        ))
    })?;
    Ok(format!(
        "https://raw.githubusercontent.com/{owner}/{repo}/{tag}/{file}"
    ))
}

/// repo file cache の保存先 (`<base>/sources/<tag>/<file>`)
pub fn cache_path(cache_base: &Path, tag: &str, file: &str) -> PathBuf {
    cache_base.join("sources").join(tag).join(file)
}

/// managed source の file cache があるか (offline で読み取れるかの目安)
pub fn has_cached_files(source: &SourceState, cache_base: &Path) -> bool {
    cache_base.join("sources").join(&source.ref_).is_dir()
}

/// managed source の repo file を読み取る。cache があればそれを返し
/// (network 不要)、無ければ `fetch` で取得して cache へ原子保存する。
/// fetch 関数は差し込み可能 (hermetic test。dashboard.rs と同じ分離
/// pattern)。tag / file は path 走査に使えない形式を拒否する。
pub fn read_managed_file_with(
    source: &SourceState,
    file: &str,
    cache_base: &Path,
    fetch: &dyn Fn(&str) -> std::result::Result<String, String>,
) -> Result<String> {
    let tag = &source.ref_;
    if tag.contains('/') || tag.contains("..") || file.contains('/') || file.contains("..") {
        return Err(Error::Precondition(format!(
            "invalid tag or file name: {tag}/{file}"
        )));
    }
    let path = cache_path(cache_base, tag, file);
    if path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Ok(content);
        }
    }
    let url = raw_url(&source.remote_url(), tag, file)?;
    let content = fetch(&url)
        .map_err(|e| Error::Precondition(format!("failed to fetch repo file {url}: {e}")))?;
    crate::machine::atomic_write(&path, &content)
        .map_err(|e| Error::Io(format!("write cache {}: {e}", path.display())))?;
    Ok(content)
}

/// [`read_managed_file_with`] の production 版
/// (state dir + `download_text`)
pub fn read_managed_file(source: &SourceState, file: &str) -> Result<String> {
    read_managed_file_with(source, file, &crate::machine::state_dir(), &default_fetch)
}

fn default_fetch(url: &str) -> std::result::Result<String, String> {
    crate::managed_nix::download_text(url).map_err(|e| e.to_string())
}

/// source 解決経由で manifest (`schneeforge.toml`) を読む。state が
/// managed Release を示す場合は tag-pinned 取得、それ以外は従来の
/// local filesystem 読み取り
pub fn load_manifest_for(repo: &str, store: &StateStore) -> Result<Manifest> {
    load_manifest_for_with(repo, store, &crate::machine::state_dir(), &default_fetch)
}

/// [`load_manifest_for`] の cache dir / fetch 差し込み版 (test 用)
pub fn load_manifest_for_with(
    repo: &str,
    store: &StateStore,
    cache_base: &Path,
    fetch: &dyn Fn(&str) -> std::result::Result<String, String>,
) -> Result<Manifest> {
    let managed = store
        .load()
        .and_then(|s| s.source)
        .filter(|s| s.is_managed_release());
    match managed {
        Some(source) => {
            let content = read_managed_file_with(&source, "schneeforge.toml", cache_base, fetch)?;
            Manifest::parse(&content)
                .map_err(|e| Error::Manifest(format!("failed to parse schneeforge.toml: {e}")))
        }
        None => Manifest::load(repo),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceKind;

    const MANIFEST_TOML: &str = "schema = 1\n[profiles]\ndefault = \"developer\"\navailable = [\"minimal\", \"developer\"]\n";

    fn managed_source(tag: &str) -> SourceState {
        SourceState {
            kind: SourceKind::ReleaseStable,
            ref_: tag.to_string(),
            channel: Some("stable".to_string()),
            managed: true,
            remote: Some("https://github.com/Lamy210/nix_setting.git".to_string()),
            revision: None,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sf-source-files-{name}-{}-{}",
            std::process::id(),
            TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn temp_store(dir: &Path, source: Option<SourceState>) -> StateStore {
        let store = StateStore::new(dir.join("state.json"));
        let state = crate::state::State {
            source,
            ..crate::state::State::default()
        };
        store.save(&state).unwrap();
        store
    }

    #[test]
    fn raw_url_is_tag_pinned() {
        assert_eq!(
            raw_url(
                "https://github.com/Lamy210/nix_setting.git",
                "v0.2.0",
                "schneeforge.toml"
            )
            .unwrap(),
            "https://raw.githubusercontent.com/Lamy210/nix_setting/v0.2.0/schneeforge.toml"
        );
        assert!(raw_url("https://gitlab.com/a/b.git", "v0.2.0", "f").is_err());
    }

    #[test]
    fn cache_path_is_under_sources_tag() {
        let p = cache_path(Path::new("/base"), "v0.2.0", "schneeforge.toml");
        assert_eq!(p, Path::new("/base/sources/v0.2.0/schneeforge.toml"));
    }

    #[test]
    fn first_read_fetches_and_second_read_uses_cache() {
        let dir = temp_dir("fetch-once");
        let source = managed_source("v0.2.0");
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let fetch = |url: &str| -> std::result::Result<String, String> {
            calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            assert!(url.ends_with("/v0.2.0/schneeforge.toml"), "url: {url}");
            Ok(MANIFEST_TOML.to_string())
        };
        let first = read_managed_file_with(&source, "schneeforge.toml", &dir, &fetch).unwrap();
        assert_eq!(first, MANIFEST_TOML);
        // cache が保存されている
        assert!(cache_path(&dir, "v0.2.0", "schneeforge.toml").is_file());
        // 2 回目は fetch が呼ばれない (network 不要)
        let second = read_managed_file_with(&source, "schneeforge.toml", &dir, &fetch).unwrap();
        assert_eq!(second, MANIFEST_TOML);
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn offline_read_serves_cache() {
        let dir = temp_dir("offline");
        let source = managed_source("v0.2.0");
        let ok =
            |_url: &str| -> std::result::Result<String, String> { Ok(MANIFEST_TOML.to_string()) };
        read_managed_file_with(&source, "schneeforge.toml", &dir, &ok).unwrap();
        // offline (fetch が常に失敗) でも cache から返る
        let fail =
            |url: &str| -> std::result::Result<String, String> { Err(format!("offline: {url}")) };
        let content = read_managed_file_with(&source, "schneeforge.toml", &dir, &fail).unwrap();
        assert_eq!(content, MANIFEST_TOML);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fetch_failure_without_cache_is_fail_closed() {
        let dir = temp_dir("fail-closed");
        let source = managed_source("v0.9.9");
        let fail =
            |url: &str| -> std::result::Result<String, String> { Err(format!("HTTP 404: {url}")) };
        let err = read_managed_file_with(&source, "schneeforge.toml", &dir, &fail).unwrap_err();
        assert!(err.to_string().contains("failed to fetch"), "got: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_path_traversal_in_tag_or_file() {
        let dir = temp_dir("traversal");
        let fetch = |_url: &str| -> std::result::Result<String, String> {
            panic!("fetch must not be called for invalid names");
        };
        let mut source = managed_source("v0.2.0/../../etc");
        assert!(read_managed_file_with(&source, "schneeforge.toml", &dir, &fetch).is_err());
        source = managed_source("v0.2.0");
        assert!(read_managed_file_with(&source, "../state.json", &dir, &fetch).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_manifest_managed_reads_via_fetch_and_caches() {
        let dir = temp_dir("manifest-managed");
        let source = managed_source("v0.2.0");
        let store = temp_store(&dir, Some(source));
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let fetch = |_url: &str| -> std::result::Result<String, String> {
            calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(MANIFEST_TOML.to_string())
        };
        let m = load_manifest_for_with("/nonexistent/repo", &store, &dir, &fetch).unwrap();
        assert_eq!(m.profiles.default.as_deref(), Some("developer"));
        // 2 回目は cache から (fetch 呼び出し回数は 1 のまま)
        load_manifest_for_with("/nonexistent/repo", &store, &dir, &fetch).unwrap();
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_manifest_path_source_reads_filesystem() {
        let dir = temp_dir("manifest-path");
        let repo = dir.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("schneeforge.toml"), MANIFEST_TOML).unwrap();
        // managed でない state (旧 state.json 相当) は fs 読み取り
        let store = temp_store(
            &dir,
            Some(SourceState {
                kind: SourceKind::ReleaseStable,
                ref_: "v0.2.0".to_string(),
                channel: Some("stable".to_string()),
                managed: false,
                remote: None,
                revision: None,
            }),
        );
        let fetch = |_url: &str| -> std::result::Result<String, String> {
            panic!("path source must not fetch");
        };
        let m = load_manifest_for_with(repo.to_str().unwrap(), &store, &dir, &fetch).unwrap();
        assert_eq!(m.profiles.default.as_deref(), Some("developer"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn has_cached_files_checks_tag_dir() {
        let dir = temp_dir("has-cache");
        let source = managed_source("v0.2.0");
        assert!(!has_cached_files(&source, &dir));
        std::fs::create_dir_all(cache_path(&dir, "v0.2.0", "schneeforge.toml")).unwrap();
        assert!(has_cached_files(&source, &dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
