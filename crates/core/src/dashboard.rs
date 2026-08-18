//! GUI Dashboard (v2 §28) — Installed / Available 表示のための snapshot 構築。
//!
//! available release の解決 (`git ls-remote --tags` + `ReleaseMetadata::fetch`)
//! は network を伴うため snapshot 構築から分離し、呼び出し元が結果を差し込む。
//! これにより offline でも installed 側は表示でき、test は hermetic になる。

use serde::Serialize;
use std::cmp::Ordering;

use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::process::run_capture;
use crate::release_metadata::ReleaseMetadata;
use crate::source::latest_tag_for_channel;
use crate::state::State;
use crate::tool::ResolvedTool;

/// Dashboard の installed 側情報
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledInfo {
    /// 実行 binary の version (呼び出し元の `CARGO_PKG_VERSION`)
    pub version: String,
    /// 実効 profile (state 選択 > manifest default)。manifest が読めなければ None
    pub profile: Option<String>,
    /// channel (state の source channel、無ければ "stable")
    pub channel: String,
    pub applied_revision: Option<String>,
    pub applied_at: Option<String>,
}

/// Dashboard 表示の snapshot (v2 §28)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DashboardSnapshot {
    pub installed: InstalledInfo,
    /// channel の最新 release の metadata。解決失敗時は None
    pub available: Option<ReleaseMetadata>,
    /// available 解決の失敗理由 (available が Some なら None)
    pub available_error: Option<String>,
    /// available version が実行 version より新しい場合に限り true
    pub update_available: bool,
}

/// state から表示する channel を決定する。
/// source が release kind で channel を持つならそれ、無ければ stable。
pub fn channel_of(state: Option<&State>) -> String {
    state
        .and_then(|s| s.source.as_ref())
        .and_then(|s| s.channel.clone())
        .unwrap_or_else(|| "stable".to_string())
}

/// installed 側情報を組み立てる。profile は state 選択 > manifest default。
pub fn installed_info(
    current_version: &str,
    state: Option<&State>,
    effective_profile: Option<&str>,
) -> InstalledInfo {
    InstalledInfo {
        version: current_version.to_string(),
        profile: effective_profile.map(|p| p.to_string()),
        channel: channel_of(state),
        applied_revision: state.and_then(|s| s.applied_revision.clone()),
        applied_at: state.and_then(|s| s.applied_at.clone()),
    }
}

/// snapshot を構築する。`available` には release 解決の結果 (Ok) か失敗理由
/// (Err) を渡す。失敗しても snapshot 全体は失敗させない (installed は保持)。
pub fn snapshot(
    current_version: &str,
    state: Option<&State>,
    manifest: Option<&Manifest>,
    available: std::result::Result<ReleaseMetadata, String>,
) -> DashboardSnapshot {
    let effective_profile = state
        .and_then(|s| s.profile.clone())
        .or_else(|| manifest.and_then(|m| m.profiles.default.clone()));
    let installed = installed_info(current_version, state, effective_profile.as_deref());
    let (available, available_error) = match available {
        Ok(m) => (Some(m), None),
        Err(e) => (None, Some(e)),
    };
    let update_available = available
        .as_ref()
        .is_some_and(|m| version_is_newer(&m.version, current_version));
    DashboardSnapshot {
        installed,
        available,
        available_error,
        update_available,
    }
}

/// `git ls-remote --tags` の出力から channel に合う最新 tag を解決する。
/// 出力は各行 `<sha>\trefs/tags/<tag>` (`^{}` peel 行を含む)。
pub fn latest_tag_from_ls_remote(output: &str, channel: &str) -> Result<String> {
    let tags: Vec<String> = output
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .filter_map(|r| r.strip_prefix("refs/tags/"))
        .filter(|t| !t.ends_with("^{}"))
        .map(|t| t.to_string())
        .collect();
    latest_tag_for_channel(&tags, channel)
        .cloned()
        .ok_or_else(|| {
            Error::ReleaseMetadata(format!("no release tag found for channel {channel}"))
        })
}

/// remote の tag 列を `git ls-remote --tags` で取得する (network)。
pub fn remote_tags(repo_url: &str, git: &ResolvedTool) -> Result<Vec<String>> {
    let args = vec![
        "ls-remote".to_string(),
        "--tags".to_string(),
        repo_url.to_string(),
    ];
    let out = run_capture(&git.path, &args)?;
    Ok(out
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .filter_map(|r| r.strip_prefix("refs/tags/"))
        .filter(|t| !t.ends_with("^{}"))
        .map(|t| t.to_string())
        .collect())
}

/// channel の最新 release を解決して metadata を取得する (network)。
/// tag 選択は `latest_tag_for_channel` と同じ規則、metadata は §27 の
/// fetch (parse + tag 整合検証) に従う。
pub fn fetch_available(
    repo_url: &str,
    channel: &str,
    git: &ResolvedTool,
) -> Result<ReleaseMetadata> {
    let tags = remote_tags(repo_url, git)?;
    let tag = latest_tag_for_channel(&tags, channel)
        .cloned()
        .ok_or_else(|| {
            Error::ReleaseMetadata(format!("no release tag found for channel {channel}"))
        })?;
    ReleaseMetadata::fetch(&tag)
}

/// `available` version が `current` より新しいか (semver 風比較)。
/// 同一 core version では正式版 > prerelease (semver 準拠)。
/// prerelease suffix の比較は数値 segment を数値として扱う (`rc.10` > `rc.9`)。
pub fn version_is_newer(available: &str, current: &str) -> bool {
    compare_versions(available, current) == Ordering::Greater
}

/// 2 つの version 文字列を比較する。core 3 segment (X.Y.Z) を数値比較し、
/// 同一なら prerelease 有無 (無し > 有り)、双方 prerelease なら suffix の
/// dot segment を数値/文字列比較する。
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (a_core, a_pre) = split_version(a);
    let (b_core, b_pre) = split_version(b);
    a_core.cmp(&b_core).then_with(|| match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        // 正式版 (suffix 無し) の方が新しい (semver: prerelease < release)
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => compare_prerelease(&x, &y),
    })
}

/// version を core 3 segment と prerelease suffix に分割する
fn split_version(v: &str) -> (Vec<u64>, Option<String>) {
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p.to_string())),
        None => (v, None),
    };
    let nums = core
        .split('.')
        .map(|p| p.parse::<u64>().unwrap_or(0))
        .collect();
    (nums, pre)
}

/// prerelease suffix を semver 風に比較する。segment 每に、双方数値なら
/// 数値比較、それ以外は文字列比較。短い方が前 (semver 準拠)。
fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    for (x, y) in a_parts.iter().zip(b_parts.iter()) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(nx), Ok(ny)) => nx.cmp(&ny),
            _ => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a_parts.len().cmp(&b_parts.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use crate::source::{SourceKind, SourceState};
    use crate::state::State;

    fn metadata(version: &str, channel: &str) -> ReleaseMetadata {
        ReleaseMetadata {
            schema: 1,
            version: version.to_string(),
            channel: channel.to_string(),
            source_revision: "0123456789abcdef0123456789abcdef01234567".to_string(),
            minimum_schneeforge_version: version.to_string(),
            configuration_schema: 1,
            systems: vec!["darwin-aarch64".to_string()],
        }
    }

    fn manifest_with_default(default: &str) -> Manifest {
        Manifest::parse(&format!(
            "schema = 1\n[distribution]\nname = \"test\"\n[profiles]\ndefault = \"{default}\"\navailable = [\"minimal\", \"developer\"]\n"
        ))
        .unwrap()
    }

    fn state_with(channel: Option<&str>, profile: Option<&str>) -> State {
        State {
            host: Some("darwin-aarch64".to_string()),
            applied_revision: Some("abc123".to_string()),
            applied_at: Some("2026-08-18T00:00:00Z".to_string()),
            product_version: Some("0.2.0".to_string()),
            source: channel.map(|c| SourceState {
                kind: if c == "preview" {
                    SourceKind::ReleasePreview
                } else {
                    SourceKind::ReleaseStable
                },
                ref_: "v0.2.0".to_string(),
                channel: Some(c.to_string()),
            }),
            profile: profile.map(|p| p.to_string()),
        }
    }

    #[test]
    fn channel_of_defaults_to_stable() {
        assert_eq!(channel_of(None), "stable");
        assert_eq!(channel_of(Some(&State::default())), "stable");
        assert_eq!(
            channel_of(Some(&state_with(Some("preview"), None))),
            "preview"
        );
        assert_eq!(
            channel_of(Some(&state_with(Some("stable"), None))),
            "stable"
        );
    }

    #[test]
    fn installed_info_uses_state_over_manifest_default() {
        let state = state_with(Some("stable"), Some("minimal"));
        let info = installed_info("0.2.0", Some(&state), Some("minimal"));
        assert_eq!(info.version, "0.2.0");
        assert_eq!(info.profile.as_deref(), Some("minimal"));
        assert_eq!(info.channel, "stable");
        assert_eq!(info.applied_revision.as_deref(), Some("abc123"));
        assert_eq!(info.applied_at.as_deref(), Some("2026-08-18T00:00:00Z"));
    }

    #[test]
    fn snapshot_prefers_state_profile_then_manifest_default() {
        let manifest = manifest_with_default("developer");
        // state 選択 > manifest default
        let snap = snapshot(
            "0.2.0",
            Some(&state_with(Some("stable"), Some("minimal"))),
            Some(&manifest),
            Ok(metadata("0.3.0", "stable")),
        );
        assert_eq!(snap.installed.profile.as_deref(), Some("minimal"));
        // state 未選択なら manifest default
        let snap = snapshot(
            "0.2.0",
            Some(&state_with(Some("stable"), None)),
            Some(&manifest),
            Ok(metadata("0.3.0", "stable")),
        );
        assert_eq!(snap.installed.profile.as_deref(), Some("developer"));
        // manifest が無ければ state の記録 (無し)
        let snap = snapshot("0.2.0", None, None, Ok(metadata("0.3.0", "stable")));
        assert_eq!(snap.installed.profile, None);
        assert_eq!(snap.installed.channel, "stable");
    }

    #[test]
    fn snapshot_with_newer_available_sets_update_flag() {
        let snap = snapshot("0.2.0", None, None, Ok(metadata("0.3.0", "stable")));
        assert!(snap.update_available);
        assert!(snap.available.is_some());
        assert_eq!(snap.available_error, None);
        assert_eq!(snap.available.as_ref().unwrap().version, "0.3.0");
    }

    #[test]
    fn snapshot_with_same_or_older_available_clears_update_flag() {
        let same = snapshot("0.2.0", None, None, Ok(metadata("0.2.0", "stable")));
        assert!(!same.update_available);
        let older = snapshot("0.2.0", None, None, Ok(metadata("0.1.0", "stable")));
        assert!(!older.update_available);
    }

    #[test]
    fn snapshot_with_available_error_keeps_installed() {
        let snap = snapshot(
            "0.2.0",
            Some(&state_with(Some("preview"), None)),
            None,
            Err("network unreachable".to_string()),
        );
        assert_eq!(snap.available, None);
        assert_eq!(snap.available_error.as_deref(), Some("network unreachable"));
        assert!(!snap.update_available);
        assert_eq!(snap.installed.channel, "preview");
        assert_eq!(snap.installed.version, "0.2.0");
    }

    #[test]
    fn ls_remote_parsing_selects_latest_tag_per_channel() {
        let out = "\
abc100\trefs/tags/v0.1.0
abc200\trefs/tags/v0.2.0-rc.4
abc300\trefs/tags/v0.2.0-rc.5
abc300\trefs/tags/v0.2.0-rc.5^{}
abc400\trefs/tags/v0.3.0
abc500\trefs/tags/not-a-release
";
        assert_eq!(latest_tag_from_ls_remote(out, "stable").unwrap(), "v0.3.0");
        assert_eq!(
            latest_tag_from_ls_remote(out, "preview").unwrap(),
            "v0.2.0-rc.5"
        );
    }

    #[test]
    fn ls_remote_parsing_fails_closed_without_matching_tag() {
        let err = latest_tag_from_ls_remote("abc\trefs/tags/v0.1.0\n", "preview").unwrap_err();
        assert!(err.to_string().contains("no release tag"), "{err}");
        // tag が 1 つも無い場合も error
        let err = latest_tag_from_ls_remote("", "stable").unwrap_err();
        assert!(err.to_string().contains("no release tag"), "{err}");
    }

    #[test]
    fn version_comparison_semver_rules() {
        // core segment の数値比較
        assert!(version_is_newer("0.3.0", "0.2.0"));
        assert!(version_is_newer("0.10.0", "0.9.0"));
        assert!(!version_is_newer("0.2.0", "0.3.0"));
        // 同一 core では正式版 > prerelease
        assert!(version_is_newer("0.2.0", "0.2.0-rc.5"));
        assert!(!version_is_newer("0.2.0-rc.5", "0.2.0"));
        // prerelease 同士は数値比較 (rc.10 > rc.9)
        assert!(version_is_newer("0.2.0-rc.10", "0.2.0-rc.9"));
        assert!(version_is_newer("0.2.0-rc.5", "0.2.0-rc.4"));
        assert!(!version_is_newer("0.2.0-rc.5", "0.2.0-rc.5"));
        // core が新しければ prerelease も newer
        assert!(version_is_newer("0.3.0-rc.1", "0.2.0"));
    }
}
