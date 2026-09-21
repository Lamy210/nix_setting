//! ConfigurationSource (ADR-0003) — repository checkout の実態から
//! source 種別を解決する。network access は行わない。

use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, path::Path};

use crate::error::Result;
use crate::process::run_capture;
use crate::tool::ResolvedTool;

/// source の種別 (v2 設計 §5-§7)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// release tag (stable) への pinned checkout
    ReleaseStable,
    /// prerelease tag への pinned checkout
    ReleasePreview,
    /// branch への checkout (fetch → pull --ff-only で追従)
    GitTracking,
    /// tag / commit への固定 (release 形式以外)
    GitPinned,
    /// git 管理外の directory (開発用 working tree 等)
    Local,
}

impl SourceKind {
    /// release channel 系か (tag checkout で表現される source)
    pub fn is_release(&self) -> bool {
        matches!(self, SourceKind::ReleaseStable | SourceKind::ReleasePreview)
    }
}

impl std::fmt::Display for SourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SourceKind::ReleaseStable => "release-stable",
            SourceKind::ReleasePreview => "release-preview",
            SourceKind::GitTracking => "git-tracking",
            SourceKind::GitPinned => "git-pinned",
            SourceKind::Local => "local",
        })
    }
}

/// State に記録する source の現在状態
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceState {
    pub kind: SourceKind,
    /// checkout が指している ref (tag 名 / branch 名 / commit hash)
    #[serde(rename = "ref")]
    pub ref_: String,
    /// Release source の channel ("stable" / "preview")。それ以外は None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// managed (working tree-less) 表現か (v2 §7)。true なら source の
    /// 実体は flake ref `github:<owner>/<repo>/<tag>` で、nix が直接
    /// 取得・cache する。旧 state.json は false (checkout 表現) として
    /// 読み込める
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub managed: bool,
    /// managed source の取得元 repository URL。fork で差し替えられた
    /// 場合の特定のため state に記録する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// ReleaseMetadata (§27) の source_revision。tag → commit SHA の
    /// 不変性の検証記録。metadata asset を持たない tag は None
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

impl SourceState {
    /// managed な Release source か (detect / dispatch の分岐条件)
    pub fn is_managed_release(&self) -> bool {
        self.managed && self.kind.is_release()
    }

    /// managed source の取得元 repository URL
    /// (未記録なら `SCHNEEFORGE_REPO_URL` / 既定 URL)
    pub fn remote_url(&self) -> String {
        self.remote.clone().unwrap_or_else(repo_url)
    }

    /// managed source の flake ref (`github:<owner>/<repo>/<tag>`)。
    /// managed でない Release や URL から owner/repo が解決できない
    /// 場合は None
    pub fn flake_ref(&self) -> Option<String> {
        if !self.is_managed_release() {
            return None;
        }
        let (owner, repo) = github_slug(&self.remote_url())?;
        Some(format!("github:{owner}/{repo}/{}", self.ref_))
    }
}

/// 既定の repository URL (`SCHNEEFORGE_REPO_URL` > `DEFAULT_REPO_URL`)。
/// install.sh の `REPO_URL="${SCHNEEFORGE_REPO_URL:-...}"` と同じ規約
pub fn repo_url() -> String {
    std::env::var("SCHNEEFORGE_REPO_URL")
        .unwrap_or_else(|_| crate::bootstrap::DEFAULT_REPO_URL.to_string())
}

/// repository URL から github の owner / repo を解決する。
/// `https://github.com/<owner>/<repo>[.git]` と
/// `git@github.com:<owner>/<repo>[.git]` 形式を受け付ける
pub fn github_slug(url: &str) -> Option<(String, String)> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .or_else(|| url.strip_prefix("git@github.com:"))?;
    let path = path.trim_end_matches(".git").trim_end_matches('/');
    let mut parts = path.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if !is_slug_component(owner) || !is_slug_component(repo) {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// owner / repo 名として安全な文字列か (URL 構築の前に検証する)
fn is_slug_component(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// nix 引数に渡す repository 参照。state が managed Release を示す場合は
/// flake ref (`github:<owner>/<repo>/<tag>`)、それ以外は path をそのまま返す
pub fn effective_ref(repo: &str, store: &crate::state::StateStore) -> String {
    store
        .load()
        .and_then(|s| s.source)
        .and_then(|src| src.flake_ref())
        .unwrap_or_else(|| repo.to_string())
}

/// checkout の実態から SourceKind を解決する
#[derive(Debug, Clone, Default)]
pub struct SourceResolver;

impl SourceResolver {
    pub fn new() -> Self {
        Self
    }

    /// repo path と解決済み git binary から source を検出する。
    /// `.git` が無ければ Local、branch があれば GitTracking、
    /// detached HEAD は tag の形式で Release/GitPinned を判別する。
    pub fn detect(&self, repo: &str, git: &ResolvedTool) -> Result<SourceState> {
        let repo_path = Path::new(repo);
        if !repo_path.join(".git").exists() {
            return Ok(SourceState {
                kind: SourceKind::Local,
                ref_: "-".to_string(),
                channel: None,
                managed: false,
                remote: None,
                revision: None,
            });
        }

        match current_branch(repo, git)? {
            Some(branch) => Ok(SourceState {
                kind: SourceKind::GitTracking,
                ref_: branch,
                channel: None,
                managed: false,
                remote: None,
                revision: None,
            }),
            None => {
                // detached HEAD: exact tag を持つか
                let tag = exact_tag(repo, git)?;
                match tag {
                    Some(tag) => match classify_release_tag(&tag) {
                        Some((kind, channel)) => Ok(SourceState {
                            kind,
                            ref_: tag,
                            channel: Some(channel.to_string()),
                            managed: false,
                            remote: None,
                            revision: None,
                        }),
                        // v prefix 以外の tag や semver 形式でない tag は固定扱い
                        None => Ok(SourceState {
                            kind: SourceKind::GitPinned,
                            ref_: tag,
                            channel: None,
                            managed: false,
                            remote: None,
                            revision: None,
                        }),
                    },
                    None => {
                        let rev = head_revision(repo, git)?;
                        Ok(SourceState {
                            kind: SourceKind::GitPinned,
                            ref_: rev,
                            channel: None,
                            managed: false,
                            remote: None,
                            revision: None,
                        })
                    }
                }
            }
        }
    }

    /// state に記録された managed source を優先する解決 (v2 §7)。
    /// state が managed な Release を示す場合は checkout の実態を見ずに
    /// それを返す。それ以外 (managed 無しを含む) は checkout から検出する。
    pub fn resolve(
        &self,
        repo: &str,
        git: &ResolvedTool,
        stored: Option<&SourceState>,
    ) -> Result<SourceState> {
        if let Some(state) = stored {
            if state.is_managed_release() {
                return Ok(state.clone());
            }
        }
        self.detect(repo, git)
    }
}

/// 現在 checkout されている branch 名 (detached HEAD は None)
fn current_branch(repo: &str, git: &ResolvedTool) -> Result<Option<String>> {
    let out = git_output(repo, git, &["symbolic-ref", "--short", "HEAD"]);
    match out {
        Ok(branch) => {
            let branch = branch.trim();
            if branch.is_empty() {
                Ok(None)
            } else {
                Ok(Some(branch.to_string()))
            }
        }
        // detached HEAD では symbolic-ref が非 exit で失敗する
        Err(_) => Ok(None),
    }
}

/// HEAD が指している exact tag (複数 tag は最初の 1 つ)
fn exact_tag(repo: &str, git: &ResolvedTool) -> Result<Option<String>> {
    let out = git_output(repo, git, &["describe", "--tags", "--exact-match"]);
    match out {
        Ok(tag) => {
            let tag = tag.trim();
            if tag.is_empty() {
                Ok(None)
            } else {
                Ok(Some(tag.to_string()))
            }
        }
        Err(_) => Ok(None),
    }
}

/// HEAD の commit hash
fn head_revision(repo: &str, git: &ResolvedTool) -> Result<String> {
    let out = git_output(repo, git, &["rev-parse", "HEAD"])?;
    Ok(out.trim().to_string())
}

fn git_output(repo: &str, git: &ResolvedTool, args: &[&str]) -> Result<String> {
    let mut cmd_args: Vec<String> = vec!["-C".to_string(), repo.to_string()];
    cmd_args.extend(args.iter().map(|s| s.to_string()));
    run_capture(&git.path, &cmd_args)
}

/// release tag 名を分類する。`v` prefix + SemVer なら
/// prerelease suffix の有無で Stable/Preview を返す。
/// managed source の設定時に tag から channel を導出するため public
pub fn classify_release_tag(tag: &str) -> Option<(SourceKind, &'static str)> {
    let version = tag.strip_prefix('v')?;
    let (_, prerelease) = parse_semver(version)?;
    if prerelease.is_some() {
        Some((SourceKind::ReleasePreview, "preview"))
    } else {
        Some((SourceKind::ReleaseStable, "stable"))
    }
}

/// SemVer 2.0.0 の release version を parse する。
///
/// - core は exactly 3 numeric identifiers
/// - core / numeric prerelease の leading zero を拒否
/// - prerelease / build identifier は ASCII alphanumeric + hyphen のみ
/// - build metadata は precedence には影響しない
fn parse_semver(version: &str) -> Option<([&str; 3], Option<&str>)> {
    let without_build = match version.split_once('+') {
        Some((base, build)) => {
            if build.contains('+') || !valid_dot_identifiers(build, false) {
                return None;
            }
            base
        }
        None => version,
    };

    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, prerelease)) => {
            if !valid_dot_identifiers(prerelease, true) {
                return None;
            }
            (core, Some(prerelease))
        }
        None => (without_build, None),
    };

    let mut parts = core.split('.');
    let major = validate_core_identifier(parts.next()?)?;
    let minor = validate_core_identifier(parts.next()?)?;
    let patch = validate_core_identifier(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }

    Some(([major, minor, patch], prerelease))
}

fn validate_core_identifier(value: &str) -> Option<&str> {
    if value.is_empty()
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    Some(value)
}

fn valid_dot_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !(reject_numeric_leading_zero
                    && identifier.len() > 1
                    && identifier.starts_with('0')
                    && identifier.bytes().all(|b| b.is_ascii_digit()))
        })
}

fn compare_numeric_identifier(a: &str, b: &str) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

fn compare_core(a: [&str; 3], b: [&str; 3]) -> Ordering {
    for (a_part, b_part) in a.into_iter().zip(b) {
        let ordering = compare_numeric_identifier(a_part, b_part);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

/// 候補 tag 列から channel に合う最新 tag を選ぶ純関数。
/// stable は prerelease を含まない。preview は prerelease のみ。
/// 同一 channel 内で SemVer precedence の最大値を最新とする。
pub fn latest_tag_for_channel<'a>(tags: &'a [String], channel: &str) -> Option<&'a String> {
    let is_preview = channel == "preview";
    tags.iter()
        .filter(|tag| {
            let Some(version) = tag.strip_prefix('v') else {
                return false;
            };
            parse_semver(version)
                .is_some_and(|(_, prerelease)| prerelease.is_some() == is_preview)
        })
        .max_by(|a, b| {
            let a = a.strip_prefix('v').expect("filtered release tag");
            let b = b.strip_prefix('v').expect("filtered release tag");
            compare_semver(a, b).expect("filtered valid semver")
        })
}

fn compare_semver(a: &str, b: &str) -> Option<Ordering> {
    let (a_core, a_pre) = parse_semver(a)?;
    let (b_core, b_pre) = parse_semver(b)?;

    Some(compare_core(a_core, b_core).then_with(|| match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a_pre), Some(b_pre)) => compare_prerelease(a_pre, b_pre),
    }))
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        let a_numeric = a_part.bytes().all(|b| b.is_ascii_digit());
        let b_numeric = b_part.bytes().all(|b| b.is_ascii_digit());
        let ordering = match (a_numeric, b_numeric) {
            (true, true) => compare_numeric_identifier(a_part, b_part),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => a_part.cmp(b_part),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    a_parts.len().cmp(&b_parts.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_path() -> std::path::PathBuf {
        which_git()
    }

    fn which_git() -> std::path::PathBuf {
        let out = std::process::Command::new("which")
            .arg("git")
            .output()
            .expect("which git");
        let p = String::from_utf8(out.stdout).expect("utf8");
        std::path::PathBuf::from(p.trim())
    }

    fn resolved_git() -> crate::tool::ResolvedTool {
        crate::tool::ResolvedTool {
            path: git_path(),
            source: crate::tool::ToolSource::Path,
            version: None,
        }
    }

    fn temp_repo(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sf-source-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn git_cmd(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new(&resolved_git().path)
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .expect("git cmd");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn git_stdout(dir: &std::path::Path, args: &[&str]) -> String {
        let out = std::process::Command::new(&resolved_git().path)
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git stdout");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .expect("utf8")
            .trim()
            .to_string()
    }

    fn commit_file(dir: &std::path::Path, name: &str) {
        std::fs::write(dir.join(name), "x").unwrap();
        git_cmd(dir, &["add", name]);
        git_cmd(dir, &["commit", "-m", name]);
    }

    #[test]
    fn classify_stable_and_preview_tags() {
        assert_eq!(
            classify_release_tag("v0.2.0"),
            Some((SourceKind::ReleaseStable, "stable"))
        );
        assert_eq!(
            classify_release_tag("v0.5.0-rc.2"),
            Some((SourceKind::ReleasePreview, "preview"))
        );
        assert_eq!(
            classify_release_tag("v1.2.3+build.5"),
            Some((SourceKind::ReleaseStable, "stable"))
        );
        assert_eq!(classify_release_tag("experiment"), None);
        assert_eq!(classify_release_tag("1.2.3"), None); // v prefix 無し

        for invalid in [
            "v01.2.3",
            "v1.02.3",
            "v1.2.03",
            "v1.2.3-01",
            "v1.2.3-alpha..1",
            "v1٢.2.3",
            "v1.2.3-",
            "v1.2",
        ] {
            assert_eq!(
                classify_release_tag(invalid),
                None,
                "{invalid} must not be accepted as a release tag"
            );
        }
    }

    #[test]
    fn detect_branch_is_tracking() {
        let dir = temp_repo("branch");
        git_cmd(&dir, &["init", "-q", "-b", "main"]);
        commit_file(&dir, "a.txt");
        let state = SourceResolver::new()
            .detect(dir.to_str().unwrap(), &resolved_git())
            .unwrap();
        assert_eq!(state.kind, SourceKind::GitTracking);
        assert_eq!(state.ref_, "main");
        assert_eq!(state.channel, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_release_tag_checkout() {
        let dir = temp_repo("release");
        git_cmd(&dir, &["init", "-q", "-b", "main"]);
        commit_file(&dir, "a.txt");
        git_cmd(&dir, &["tag", "v0.2.0"]);
        git_cmd(&dir, &["checkout", "-q", "v0.2.0"]);
        let state = SourceResolver::new()
            .detect(dir.to_str().unwrap(), &resolved_git())
            .unwrap();
        assert_eq!(state.kind, SourceKind::ReleaseStable);
        assert_eq!(state.ref_, "v0.2.0");
        assert_eq!(state.channel.as_deref(), Some("stable"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_prerelease_tag_checkout() {
        let dir = temp_repo("preview");
        git_cmd(&dir, &["init", "-q", "-b", "main"]);
        commit_file(&dir, "a.txt");
        git_cmd(&dir, &["tag", "v0.5.0-rc.2"]);
        git_cmd(&dir, &["checkout", "-q", "v0.5.0-rc.2"]);
        let state = SourceResolver::new()
            .detect(dir.to_str().unwrap(), &resolved_git())
            .unwrap();
        assert_eq!(state.kind, SourceKind::ReleasePreview);
        assert_eq!(state.channel.as_deref(), Some("preview"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_detached_commit_is_pinned() {
        let dir = temp_repo("pinned");
        git_cmd(&dir, &["init", "-q", "-b", "main"]);
        commit_file(&dir, "a.txt");
        let rev = git_stdout(&dir, &["rev-parse", "HEAD"]);
        git_cmd(&dir, &["checkout", "-q", &rev]);
        let state = SourceResolver::new()
            .detect(dir.to_str().unwrap(), &resolved_git())
            .unwrap();
        assert_eq!(state.kind, SourceKind::GitPinned);
        assert_eq!(state.ref_, rev);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_non_git_dir_is_local() {
        let dir = temp_repo("local");
        let state = SourceResolver::new()
            .detect(dir.to_str().unwrap(), &resolved_git())
            .unwrap();
        assert_eq!(state.kind, SourceKind::Local);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn latest_tag_filters_channel_and_sorts_by_semver_precedence() {
        let tags = vec![
            "v0.2.0".to_string(),
            "v0.10.0".to_string(),
            "v0.9.0".to_string(),
            "v0.11.0-rc.9".to_string(),
            "v0.11.0-rc.10".to_string(),
            "v0.11.0-10".to_string(),
            "v0.11.0-alpha".to_string(),
            "v00.12.0".to_string(),
            "v0.12.0-01".to_string(),
            "experiment".to_string(),
        ];
        assert_eq!(latest_tag_for_channel(&tags, "stable"), Some(&tags[1]));
        assert_eq!(
            latest_tag_for_channel(&tags, "preview"),
            Some(&"v0.11.0-rc.10".to_string())
        );
    }

    #[test]
    fn semver_prerelease_numeric_identifiers_sort_before_text_identifiers() {
        assert_eq!(
            compare_semver("1.0.0-1", "1.0.0-alpha"),
            Some(Ordering::Less)
        );
        assert_eq!(
            compare_semver("1.0.0-alpha.9", "1.0.0-alpha.10"),
            Some(Ordering::Less)
        );
        assert_eq!(
            compare_semver("1.0.0-rc.10+build.1", "1.0.0-rc.10+build.2"),
            Some(Ordering::Equal),
            "build metadata must not affect precedence"
        );
        assert_eq!(
            compare_semver(
                "184467440737095516160.0.0",
                "184467440737095516159.999.999",
            ),
            Some(Ordering::Greater),
            "SemVer numeric identifiers are not bounded to machine integers"
        );
    }

    #[test]
    fn source_state_serializes_with_kebab_kind() {
        let s = SourceState {
            kind: SourceKind::ReleaseStable,
            ref_: "v0.2.0".to_string(),
            channel: Some("stable".to_string()),
            managed: false,
            remote: None,
            revision: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"release-stable\""));
        assert!(json.contains("\"ref\""));
        let back: SourceState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    // -------------------------------------------------------------------------
    // managed source (v2 §7)
    // -------------------------------------------------------------------------

    fn managed_state(kind: SourceKind, tag: &str, channel: &str) -> SourceState {
        SourceState {
            kind,
            ref_: tag.to_string(),
            channel: Some(channel.to_string()),
            managed: true,
            remote: Some("https://github.com/Lamy210/nix_setting.git".to_string()),
            revision: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
        }
    }

    #[test]
    fn legacy_state_without_managed_loads_as_checkout() {
        // managed field を持たない旧 state.json は managed=false として読める
        let json = r#"{"kind":"release-stable","ref":"v0.2.0","channel":"stable"}"#;
        let s: SourceState = serde_json::from_str(json).unwrap();
        assert_eq!(s.kind, SourceKind::ReleaseStable);
        assert!(!s.managed);
        assert!(s.remote.is_none());
        assert!(s.revision.is_none());
    }

    #[test]
    fn managed_state_roundtrips_new_fields() {
        let s = managed_state(SourceKind::ReleasePreview, "v0.3.0-rc.1", "preview");
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"managed\":true"));
        assert!(json.contains("\"remote\""));
        assert!(json.contains("\"revision\""));
        let back: SourceState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn managed_fields_are_omitted_when_default() {
        // checkout 表現の state.json は旧形式と同じ key 構成のまま
        let s = SourceState {
            kind: SourceKind::ReleaseStable,
            ref_: "v0.2.0".to_string(),
            channel: Some("stable".to_string()),
            managed: false,
            remote: None,
            revision: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("\"managed\""));
        assert!(!json.contains("\"remote\""));
        assert!(!json.contains("\"revision\""));
    }

    #[test]
    fn flake_ref_formats_github_tag_ref() {
        let s = managed_state(SourceKind::ReleaseStable, "v0.2.0", "stable");
        assert_eq!(
            s.flake_ref().as_deref(),
            Some("github:Lamy210/nix_setting/v0.2.0")
        );
        // ssh 形式の fork URL でも owner/repo は解決できる
        let fork = SourceState {
            remote: Some("git@github.com:alice/nix_setting.git".to_string()),
            ..managed_state(SourceKind::ReleaseStable, "v0.2.0", "stable")
        };
        assert_eq!(
            fork.flake_ref().as_deref(),
            Some("github:alice/nix_setting/v0.2.0")
        );
    }

    #[test]
    fn flake_ref_is_none_for_non_managed_or_non_release() {
        let mut s = managed_state(SourceKind::ReleaseStable, "v0.2.0", "stable");
        s.managed = false;
        assert_eq!(s.flake_ref(), None);
        let pinned = SourceState {
            kind: SourceKind::GitPinned,
            ref_: "v0.2.0".to_string(),
            channel: None,
            managed: true,
            remote: None,
            revision: None,
        };
        assert_eq!(pinned.flake_ref(), None);
    }

    #[test]
    fn github_slug_parses_supported_url_forms() {
        assert_eq!(
            github_slug("https://github.com/Lamy210/nix_setting.git"),
            Some(("Lamy210".to_string(), "nix_setting".to_string()))
        );
        assert_eq!(
            github_slug("https://github.com/Lamy210/nix_setting"),
            Some(("Lamy210".to_string(), "nix_setting".to_string()))
        );
        assert_eq!(
            github_slug("git@github.com:alice/nix_setting.git"),
            Some(("alice".to_string(), "nix_setting".to_string()))
        );
        assert_eq!(
            github_slug("ssh://git@github.com/alice/nix_setting.git"),
            Some(("alice".to_string(), "nix_setting".to_string()))
        );
        assert_eq!(github_slug("https://gitlab.com/a/b.git"), None);
        assert_eq!(github_slug("https://github.com/a/b/c"), None);
        assert_eq!(github_slug("https://github.com/../nix_setting"), None);
    }

    #[test]
    fn effective_ref_uses_flake_ref_only_for_managed_release() {
        let dir = std::env::temp_dir().join(format!("sf-effective-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::state::StateStore::new(dir.join("state.json"));

        // managed でない場合は path をそのまま返す
        assert_eq!(effective_ref("/tmp/repo", &store), "/tmp/repo");

        let mut state = crate::state::State {
            source: Some(managed_state(SourceKind::ReleaseStable, "v0.2.0", "stable")),
            ..crate::state::State::default()
        };
        store.save(&state).unwrap();
        assert_eq!(
            effective_ref("/tmp/repo", &store),
            "github:Lamy210/nix_setting/v0.2.0"
        );

        // checkout 表現の source が記録されていても path のまま
        state.source = Some(SourceState {
            kind: SourceKind::ReleaseStable,
            ref_: "v0.2.0".to_string(),
            channel: Some("stable".to_string()),
            managed: false,
            remote: None,
            revision: None,
        });
        store.save(&state).unwrap();
        assert_eq!(effective_ref("/tmp/repo", &store), "/tmp/repo");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_prefers_managed_state_over_checkout() {
        // managed state は checkout 実態 (Local な空 dir) によらず返される
        let dir = temp_repo("managed-resolve");
        let stored = managed_state(SourceKind::ReleaseStable, "v0.2.0", "stable");
        let resolved = SourceResolver::new()
            .resolve(dir.to_str().unwrap(), &resolved_git(), Some(&stored))
            .unwrap();
        assert_eq!(resolved, stored);

        // managed でない state (旧 state.json 相当) は checkout 検出に fallthrough
        let checkout_state = SourceState {
            kind: SourceKind::ReleaseStable,
            ref_: "v0.2.0".to_string(),
            channel: Some("stable".to_string()),
            managed: false,
            remote: None,
            revision: None,
        };
        let resolved = SourceResolver::new()
            .resolve(
                dir.to_str().unwrap(),
                &resolved_git(),
                Some(&checkout_state),
            )
            .unwrap();
        assert_eq!(resolved.kind, SourceKind::Local);
        assert!(!resolved.managed);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
