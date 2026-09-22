//! managed source の repo file 読み取り (v2 §7)。
//!
//! managed source は local に source tree を持たないため、repo file
//! (`schneeforge.toml` 等) は `raw.githubusercontent.com` から tag pinned で
//! 取得し state dir (`sources/<tag>/`) へ原子保存する。cache entry は
//! repository identity + content SHA-256 の sidecar に bind し、fork 間で
//! 同一 tag の cache を誤再利用しない。検証済み cache の 2 回目以降の読み取り
//! (offline 含む) は network を行わない。cache が無い状態での取得失敗
//! (offline 初回 / 404) は fail-closed に error を返す。
//! path source (checkout / Local) の file 読み取りは従来どおり local
//! filesystem を使う。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::source::{classify_release_tag, github_slug, SourceState};
use crate::state::StateStore;

const CACHE_PROVENANCE_SCHEMA: u32 = 1;
const CACHE_PROVENANCE_SUFFIX: &str = ".schneeforge-cache.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CacheProvenance {
    schema: u32,
    repository: String,
    sha256: String,
}

/// tag pinned の raw file URL
/// (`raw.githubusercontent.com/<owner>/<repo>/<tag>/<file>`)
pub fn raw_url(remote: &str, tag: &str, file: &str) -> Result<String> {
    if classify_release_tag(tag).is_none() {
        return Err(Error::Precondition(format!(
            "invalid managed release tag: {tag}"
        )));
    }
    if !is_safe_cache_component(tag) || !is_safe_cache_file(file) {
        return Err(Error::Precondition(format!(
            "invalid tag or file name: {tag}/{file}"
        )));
    }
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
fn cache_path(cache_base: &Path, tag: &str, file: &str) -> PathBuf {
    cache_base.join("sources").join(tag).join(file)
}

fn cache_provenance_path(cache_base: &Path, tag: &str, file: &str) -> PathBuf {
    cache_base
        .join("sources")
        .join(tag)
        .join(format!("{file}{CACHE_PROVENANCE_SUFFIX}"))
}

fn repository_identity(remote: &str) -> Result<String> {
    let (owner, repo) = github_slug(remote).ok_or_else(|| {
        Error::Precondition(format!(
            "cannot resolve owner/repo from repository URL: {remote}"
        ))
    })?;
    Ok(format!(
        "{}/{}",
        owner.to_ascii_lowercase(),
        repo.to_ascii_lowercase()
    ))
}

fn sha256_text(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_safe_cache_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains("..")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-'))
}

fn is_safe_cache_file(file: &str) -> bool {
    is_safe_cache_component(file) && !file.ends_with(CACHE_PROVENANCE_SUFFIX)
}

fn read_verified_cache(
    repository: &str,
    tag: &str,
    file: &str,
    cache_base: &Path,
) -> Option<String> {
    if !is_safe_cache_component(tag) || !is_safe_cache_file(file) {
        return None;
    }
    let path = cache_path(cache_base, tag, file);
    let provenance_path = cache_provenance_path(cache_base, tag, file);
    let content = std::fs::read_to_string(path).ok()?;
    let provenance_text = std::fs::read_to_string(provenance_path).ok()?;
    let provenance: CacheProvenance = serde_json::from_str(&provenance_text).ok()?;

    if provenance.schema != CACHE_PROVENANCE_SCHEMA
        || provenance.repository != repository
        || provenance.sha256 != sha256_text(&content)
    {
        return None;
    }
    Some(content)
}

/// managed source の file cache があるか (offline で読み取れるかの目安)。
/// repository identity + digest を検証できる entry だけを cache とみなす。
pub fn has_cached_files(source: &SourceState, cache_base: &Path) -> bool {
    if source.validate_managed_release().is_err()
        || classify_release_tag(&source.ref_).is_none()
        || !is_safe_cache_component(&source.ref_)
    {
        return false;
    }
    let remote = source.remote_url();
    let Ok(repository) = repository_identity(&remote) else {
        return false;
    };
    let dir = cache_base.join("sources").join(&source.ref_);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    entries.filter_map(std::result::Result::ok).any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(file) = name.strip_suffix(CACHE_PROVENANCE_SUFFIX) else {
            return false;
        };
        is_safe_cache_file(file)
            && read_verified_cache(&repository, &source.ref_, file, cache_base).is_some()
    })
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
    source.validate_managed_release()?;
    let tag = &source.ref_;
    let remote = source.remote_url();
    // Validate the immutable release-tag boundary before consulting cache.
    let url = raw_url(&remote, tag, file)?;
    let repository = repository_identity(&remote)?;
    if let Some(content) = read_verified_cache(&repository, tag, file, cache_base) {
        return Ok(content);
    }
    let content = fetch(&url)
        .map_err(|e| Error::Precondition(format!("failed to fetch repo file {url}: {e}")))?;
    let path = cache_path(cache_base, tag, file);
    let provenance_path = cache_provenance_path(cache_base, tag, file);

    // Existing provenance must not survive a content replacement. If the process
    // stops before the new sidecar is written, the next read treats the entry as
    // an untrusted cache miss instead of returning mismatched content.
    match std::fs::remove_file(&provenance_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(Error::Io(format!(
                "invalidate cache provenance {}: {e}",
                provenance_path.display()
            )));
        }
    }

    crate::machine::atomic_write(&path, &content)
        .map_err(|e| Error::Io(format!("write cache {}: {e}", path.display())))?;

    let provenance = CacheProvenance {
        schema: CACHE_PROVENANCE_SCHEMA,
        repository,
        sha256: sha256_text(&content),
    };
    let provenance_json = serde_json::to_string(&provenance)
        .map_err(|e| Error::Io(format!("serialize cache provenance: {e}")))?;
    crate::machine::atomic_write(&provenance_path, &provenance_json).map_err(|e| {
        Error::Io(format!(
            "write cache provenance {}: {e}",
            provenance_path.display()
        ))
    })?;

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
        .load()?
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
        managed_source_from("https://github.com/Lamy210/nix_setting.git", tag)
    }

    fn managed_source_from(remote: &str, tag: &str) -> SourceState {
        SourceState {
            kind: SourceKind::ReleaseStable,
            ref_: tag.to_string(),
            channel: Some("stable".to_string()),
            managed: true,
            remote: Some(remote.to_string()),
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
        assert!(raw_url(
            "https://github.com/Lamy210/nix_setting.git",
            "../main",
            "schneeforge.toml"
        )
        .is_err());
        assert!(
            raw_url(
                "https://github.com/Lamy210/nix_setting.git",
                "main",
                "schneeforge.toml"
            )
            .is_err(),
            "managed repo-file reads require an immutable release tag"
        );
        assert!(raw_url(
            "https://github.com/Lamy210/nix_setting.git",
            "v0.2.0",
            "nested\\schneeforge.toml"
        )
        .is_err());
        assert!(raw_url(
            "https://github.com/Lamy210/nix_setting.git",
            "v0.2.0",
            "C:state.json"
        )
        .is_err());
        assert!(raw_url(
            "https://github.com/Lamy210/nix_setting.git",
            "v0.2.0",
            "%2e%2e%2fstate.json"
        )
        .is_err());
        assert!(
            raw_url(
                "https://github.com/Lamy210/nix_setting.git",
                "v0.2.0",
                "schneeforge.toml.schneeforge-cache.json"
            )
            .is_err(),
            "provenance sidecar namespace is reserved"
        );
    }

    #[test]
    fn cache_path_is_under_sources_tag() {
        let p = cache_path(Path::new("/base"), "v0.2.0", "schneeforge.toml");
        assert_eq!(p, Path::new("/base/sources/v0.2.0/schneeforge.toml"));
    }

    #[test]
    fn managed_file_read_rejects_kind_tag_mismatch_before_cache_or_network() {
        let dir = temp_dir("kind-tag-mismatch");
        let mut source = managed_source("v0.2.0-rc.1");
        source.kind = crate::source::SourceKind::ReleaseStable;
        source.channel = Some("stable".to_string());

        let fetch = |_url: &str| -> std::result::Result<String, String> {
            panic!("invalid managed source must fail before network")
        };
        let err = read_managed_file_with(&source, "schneeforge.toml", &dir, &fetch).unwrap_err();
        assert!(matches!(err, Error::State(_)), "{err}");
        assert!(err.to_string().contains("does not match"), "{err}");
        assert!(!has_cached_files(&source, &dir));
        let _ = std::fs::remove_dir_all(&dir);
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
        // content + provenance が保存されている
        assert!(cache_path(&dir, "v0.2.0", "schneeforge.toml").is_file());
        assert!(cache_provenance_path(&dir, "v0.2.0", "schneeforge.toml").is_file());
        assert!(has_cached_files(&source, &dir));
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
        assert!(!has_cached_files(&source, &dir));

        source = managed_source("v0.2.0\\..\\etc");
        assert!(read_managed_file_with(&source, "schneeforge.toml", &dir, &fetch).is_err());
        assert!(!has_cached_files(&source, &dir));

        source = managed_source("main");
        assert!(read_managed_file_with(&source, "schneeforge.toml", &dir, &fetch).is_err());
        assert!(!has_cached_files(&source, &dir));

        source = managed_source("v0.2.0");
        assert!(read_managed_file_with(&source, "../state.json", &dir, &fetch).is_err());
        assert!(read_managed_file_with(&source, "nested\\state.json", &dir, &fetch).is_err());
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
    fn load_manifest_rejects_corrupt_existing_state() {
        let dir = temp_dir("manifest-corrupt-state");
        let store = StateStore::new(dir.join("state.json"));
        std::fs::write(store.path(), "{not-json").unwrap();
        let fetch = |_url: &str| -> std::result::Result<String, String> {
            panic!("corrupt state must fail before source fallback/fetch");
        };
        let err = load_manifest_for_with("/tmp/fallback-repo", &store, &dir, &fetch).unwrap_err();
        assert!(matches!(err, Error::State(_)), "{err}");
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
    fn has_cached_files_requires_verified_provenance() {
        let dir = temp_dir("has-cache");
        let source = managed_source("v0.2.0");
        assert!(!has_cached_files(&source, &dir));

        let path = cache_path(&dir, "v0.2.0", "schneeforge.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, MANIFEST_TOML).unwrap();
        assert!(
            !has_cached_files(&source, &dir),
            "legacy content without provenance must not be reported as trusted cache"
        );

        let malicious_sidecar = dir
            .join("sources")
            .join("v0.2.0")
            .join(format!("....{CACHE_PROVENANCE_SUFFIX}"));
        std::fs::write(
            malicious_sidecar,
            r#"{"schema":1,"repository":"lamy210/nix_setting","sha256":"ignored"}"#,
        )
        .unwrap();
        assert!(
            !has_cached_files(&source, &dir),
            "sidecar-derived file names must be validated before cache lookup"
        );

        let ok =
            |_url: &str| -> std::result::Result<String, String> { Ok(MANIFEST_TOML.to_string()) };
        read_managed_file_with(&source, "schneeforge.toml", &dir, &ok).unwrap();
        assert!(has_cached_files(&source, &dir));

        std::fs::write(&path, "tampered").unwrap();
        assert!(
            !has_cached_files(&source, &dir),
            "digest mismatch must invalidate cache availability"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_tag_from_different_repository_never_reuses_cache() {
        let dir = temp_dir("fork-collision");
        let source_a =
            managed_source_from("https://github.com/example-a/nix_setting.git", "v0.2.0");
        let source_b =
            managed_source_from("https://github.com/example-b/nix_setting.git", "v0.2.0");

        let fetch_a = |url: &str| -> std::result::Result<String, String> {
            assert!(url.contains("/example-a/nix_setting/"), "{url}");
            Ok("from-a".to_string())
        };
        assert_eq!(
            read_managed_file_with(&source_a, "schneeforge.toml", &dir, &fetch_a).unwrap(),
            "from-a"
        );

        let offline =
            |url: &str| -> std::result::Result<String, String> { Err(format!("offline: {url}")) };
        let err =
            read_managed_file_with(&source_b, "schneeforge.toml", &dir, &offline).unwrap_err();
        assert!(
            err.to_string().contains("failed to fetch"),
            "fork B must not receive fork A cache: {err}"
        );
        assert!(!has_cached_files(&source_b, &dir));

        let fetch_b = |url: &str| -> std::result::Result<String, String> {
            assert!(url.contains("/example-b/nix_setting/"), "{url}");
            Ok("from-b".to_string())
        };
        assert_eq!(
            read_managed_file_with(&source_b, "schneeforge.toml", &dir, &fetch_b).unwrap(),
            "from-b"
        );
        assert_eq!(
            read_managed_file_with(&source_b, "schneeforge.toml", &dir, &offline).unwrap(),
            "from-b"
        );

        let err =
            read_managed_file_with(&source_a, "schneeforge.toml", &dir, &offline).unwrap_err();
        assert!(
            err.to_string().contains("failed to fetch"),
            "fork A must not receive fork B cache: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_cached_content_is_not_returned_offline() {
        let dir = temp_dir("tampered-cache");
        let source = managed_source("v0.2.0");
        let ok =
            |_url: &str| -> std::result::Result<String, String> { Ok(MANIFEST_TOML.to_string()) };
        read_managed_file_with(&source, "schneeforge.toml", &dir, &ok).unwrap();

        std::fs::write(cache_path(&dir, "v0.2.0", "schneeforge.toml"), "tampered").unwrap();
        let offline =
            |url: &str| -> std::result::Result<String, String> { Err(format!("offline: {url}")) };
        let err = read_managed_file_with(&source, "schneeforge.toml", &dir, &offline).unwrap_err();
        assert!(err.to_string().contains("failed to fetch"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
