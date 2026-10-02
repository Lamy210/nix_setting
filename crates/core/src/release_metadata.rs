//! Release Metadata (v2 §27) — release asset `schneeforge-release.json` の
//! parse・検証・取得。release の version / channel / source revision /
//! 最低限必要な schneeforge 版数 / 対応 systems を machine-readable に表現し、
//! GUI Dashboard (§28) の Installed / Available 表示の基盤になる。

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::managed_nix::download::{download_text_detailed, TextDownloadError};
use crate::source::{classify_release_tag, github_slug};

pub const RELEASE_METADATA_SCHEMA: u32 = 1;
const METADATA_ASSET: &str = "schneeforge-release.json";

/// release asset `schneeforge-release.json` の内容 (v2 §27)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseMetadata {
    pub schema: u32,
    pub version: String,
    /// "stable" / "preview" (version の prerelease 有無から導出)
    pub channel: String,
    /// 40-hex commit SHA
    pub source_revision: String,
    /// この metadata を読むために必要な schneeforge の最低版数
    pub minimum_schneeforge_version: String,
    /// metadata 生成時点の schneeforge.toml schema
    pub configuration_schema: u32,
    /// 生成時点で有効化されていた systems
    pub systems: Vec<String>,
}

/// prerelease suffix の有無から channel を導出する。
/// build metadata 内の `-` は prerelease separator として扱わない。
pub fn channel_for_version(version: &str) -> &'static str {
    let precedence_version = version.split_once('+').map_or(version, |(base, _)| base);
    if precedence_version.contains('-') {
        "preview"
    } else {
        "stable"
    }
}

impl ReleaseMetadata {
    /// JSON text を metadata に parse する。未対応 schema は fail-closed。
    pub fn parse(json: &str) -> Result<Self> {
        let m: ReleaseMetadata = serde_json::from_str(json)
            .map_err(|e| Error::ReleaseMetadata(format!("parse failed: {e}")))?;
        if m.schema != RELEASE_METADATA_SCHEMA {
            return Err(Error::ReleaseMetadata(format!(
                "schema {} is not supported (expected {}); this schneeforge is too old for the release",
                m.schema, RELEASE_METADATA_SCHEMA
            )));
        }
        Ok(m)
    }

    /// release tag との整合を検証する。version / channel / systems の
    /// 不一致は error (1 release = 1 source tree = 1 checksum set の前提確認)。
    pub fn validate(&self, tag: &str) -> Result<()> {
        let expected_version = tag
            .strip_prefix('v')
            .ok_or_else(|| Error::ReleaseMetadata(format!("tag must start with 'v': {tag}")))?;
        let (_, expected_channel) = classify_release_tag(tag)
            .ok_or_else(|| Error::ReleaseMetadata(format!("invalid SemVer release tag: {tag}")))?;
        if self.version != expected_version {
            return Err(Error::ReleaseMetadata(format!(
                "version {} does not match tag {tag}",
                self.version
            )));
        }
        if self.channel != expected_channel {
            return Err(Error::ReleaseMetadata(format!(
                "channel {} does not match version {} (expected {expected_channel})",
                self.channel, self.version
            )));
        }
        if self.systems.is_empty() {
            return Err(Error::ReleaseMetadata(
                "systems must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// upstream repository の release metadata asset URL。
    ///
    /// 後方互換 API。managed source / fork のように repository identity が
    /// 明示されている経路は `asset_url_for_repo` / `fetch_from` を使う。
    pub fn asset_url(tag: &str) -> String {
        format!("https://github.com/Lamy210/nix_setting/releases/download/{tag}/{METADATA_ASSET}")
    }

    /// 指定 repository の release metadata asset URL。
    ///
    /// managed source と同じ GitHub repository identity を使い、fork の tag を
    /// upstream metadata と混同しない。
    pub fn asset_url_for_repo(repo_url: &str, tag: &str) -> Result<String> {
        let (owner, repo) = github_slug(repo_url).ok_or_else(|| {
            Error::ReleaseMetadata(format!(
                "unsupported GitHub repository URL for release metadata: {repo_url}"
            ))
        })?;
        Ok(format!(
            "https://github.com/{owner}/{repo}/releases/download/{tag}/{METADATA_ASSET}"
        ))
    }

    /// 現在の repository (`SCHNEEFORGE_REPO_URL` / default) から指定 tag の
    /// metadata を取得して parse・検証する。
    pub fn fetch(tag: &str) -> Result<Self> {
        let repo_url = crate::source::repo_url();
        Self::fetch_from(&repo_url, tag)
    }

    /// 指定 repository / tag の metadata を GitHub release asset から取得して
    /// parse・検証する。asset が存在しない release や network error は
    /// fail-closed に error。
    pub fn fetch_from(repo_url: &str, tag: &str) -> Result<Self> {
        if !tag.starts_with('v') {
            return Err(Error::ReleaseMetadata(format!(
                "tag must start with 'v': {tag}"
            )));
        }
        if classify_release_tag(tag).is_none() {
            return Err(Error::ReleaseMetadata(format!(
                "invalid SemVer release tag: {tag}"
            )));
        }
        let url = Self::asset_url_for_repo(repo_url, tag)?;
        let text = metadata_text_from_download(tag, download_text_detailed(&url))?;
        let metadata = Self::parse(&text)?;
        metadata.validate(tag)?;
        Ok(metadata)
    }
}

fn metadata_text_from_download(
    tag: &str,
    downloaded: std::result::Result<String, TextDownloadError>,
) -> Result<String> {
    match downloaded {
        Ok(text) => Ok(text),
        Err(TextDownloadError::HttpStatus { status: 404, .. }) => {
            Err(Error::ReleaseMetadataAssetMissing {
                tag: tag.to_string(),
            })
        }
        Err(error) => Err(Error::ManagedNix(error.into_managed_nix())),
    }
}

/// release page の URL。GUI の「GitHub Releases を開く」誘導で使う。
/// `repo_url` は `DEFAULT_REPO_URL` (`SCHNEEFORGE_REPO_URL` で上書き可) を
/// 想定する。HTTPS / SSH の supported GitHub repository URL を canonical
/// HTTPS web URL へ正規化する。`version` は `ReleaseMetadata.version`
/// (tag から先頭の v を除いたもの) で、tag は常に `v<version>`。
pub fn release_page_url(repo_url: &str, version: &str) -> Result<String> {
    let (owner, repo) = github_slug(repo_url).ok_or_else(|| {
        Error::ReleaseMetadata(format!(
            "unsupported GitHub repository URL for release page: {repo_url}"
        ))
    })?;
    let tag = format!("v{version}");
    if classify_release_tag(&tag).is_none() {
        return Err(Error::ReleaseMetadata(format!(
            "invalid SemVer release version for release page: {version}"
        )));
    }
    Ok(format!(
        "https://github.com/{owner}/{repo}/releases/tag/{tag}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn sample_json() -> String {
        format!(
            r#"{{
  "schema": 1,
  "version": "0.2.0-rc.5",
  "channel": "preview",
  "source_revision": "{SHA}",
  "minimum_schneeforge_version": "0.2.0-rc.5",
  "configuration_schema": 1,
  "systems": ["darwin-aarch64", "linux-generic"]
}}"#
        )
    }

    fn sample() -> ReleaseMetadata {
        ReleaseMetadata::parse(&sample_json()).unwrap()
    }

    #[test]
    fn parse_extracts_all_fields() {
        let m = sample();
        assert_eq!(m.schema, 1);
        assert_eq!(m.version, "0.2.0-rc.5");
        assert_eq!(m.channel, "preview");
        assert_eq!(m.source_revision, SHA);
        assert_eq!(m.minimum_schneeforge_version, "0.2.0-rc.5");
        assert_eq!(m.configuration_schema, 1);
        assert_eq!(m.systems, vec!["darwin-aarch64", "linux-generic"]);
    }

    #[test]
    fn parse_rejects_unsupported_schema() {
        let json = sample_json().replace("\"schema\": 1", "\"schema\": 2");
        let err = ReleaseMetadata::parse(&json).unwrap_err();
        assert!(err.to_string().contains("schema 2"), "{err}");
    }

    #[test]
    fn parse_rejects_invalid_json() {
        let err = ReleaseMetadata::parse("not json").unwrap_err();
        assert!(err.to_string().contains("parse failed"), "{err}");
    }

    #[test]
    fn parse_rejects_missing_field() {
        // GitHub 404 が JSON body を返す case 相当: schema 等の必須 field が無い
        let err = ReleaseMetadata::parse(r#"{"message":"Not Found"}"#).unwrap_err();
        assert!(err.to_string().contains("parse failed"), "{err}");
    }

    #[test]
    fn channel_for_version_preview_and_stable() {
        assert_eq!(channel_for_version("0.2.0-rc.5"), "preview");
        assert_eq!(channel_for_version("0.2.0-beta.1"), "preview");
        assert_eq!(channel_for_version("1.0.0--foo"), "preview");
        assert_eq!(channel_for_version("0.2.0"), "stable");
        assert_eq!(channel_for_version("1.0.0"), "stable");
        assert_eq!(channel_for_version("1.0.0+build-5"), "stable");
    }

    #[test]
    fn validate_accepts_consistent_metadata() {
        sample().validate("v0.2.0-rc.5").unwrap();
    }

    #[test]
    fn validate_rejects_invalid_semver_tag() {
        let err = sample().validate("v00.2.0-rc.5").unwrap_err();
        assert!(
            err.to_string().contains("invalid SemVer release tag"),
            "{err}"
        );
    }

    #[test]
    fn validate_rejects_version_mismatch() {
        let err = sample().validate("v0.3.0-rc.1").unwrap_err();
        assert!(err.to_string().contains("does not match tag"), "{err}");
    }

    #[test]
    fn validate_rejects_tag_without_v_prefix() {
        let err = sample().validate("0.2.0-rc.5").unwrap_err();
        assert!(err.to_string().contains("must start with 'v'"), "{err}");
    }

    #[test]
    fn validate_rejects_channel_mismatch() {
        let m = ReleaseMetadata {
            channel: "stable".to_string(),
            ..sample()
        };
        let err = m.validate("v0.2.0-rc.5").unwrap_err();
        assert!(err.to_string().contains("channel"), "{err}");
    }

    #[test]
    fn validate_rejects_empty_systems() {
        let m = ReleaseMetadata {
            systems: vec![],
            ..sample()
        };
        let err = m.validate("v0.2.0-rc.5").unwrap_err();
        assert!(err.to_string().contains("systems"), "{err}");
    }

    #[test]
    fn missing_metadata_asset_is_structured_separately_from_other_failures() {
        let missing = metadata_text_from_download(
            "v0.1.0",
            Err(TextDownloadError::HttpStatus {
                url: "https://example.invalid/v0.1.0/schneeforge-release.json".to_string(),
                status: 404,
            }),
        )
        .unwrap_err();
        assert!(matches!(
            missing,
            Error::ReleaseMetadataAssetMissing { ref tag } if tag == "v0.1.0"
        ));

        let unavailable = metadata_text_from_download(
            "v0.2.0",
            Err(TextDownloadError::Other(
                crate::managed_nix::ManagedNixError::Download {
                    source: "network unavailable".to_string(),
                },
            )),
        )
        .unwrap_err();
        assert!(matches!(unavailable, Error::ManagedNix(_)), "{unavailable}");

        let server_error = metadata_text_from_download(
            "v0.2.0",
            Err(TextDownloadError::HttpStatus {
                url: "https://example.invalid/v0.2.0/schneeforge-release.json".to_string(),
                status: 500,
            }),
        )
        .unwrap_err();
        assert!(
            matches!(server_error, Error::ManagedNix(_)),
            "{server_error}"
        );
    }

    #[test]
    fn asset_url_shape() {
        assert_eq!(
            ReleaseMetadata::asset_url("v0.2.0-rc.5"),
            "https://github.com/Lamy210/nix_setting/releases/download/v0.2.0-rc.5/schneeforge-release.json"
        );
    }

    #[test]
    fn asset_url_for_repo_preserves_fork_identity() {
        assert_eq!(
            ReleaseMetadata::asset_url_for_repo(
                "https://github.com/example/schneeforge-fork.git",
                "v1.2.3"
            )
            .unwrap(),
            "https://github.com/example/schneeforge-fork/releases/download/v1.2.3/schneeforge-release.json"
        );
        assert_eq!(
            ReleaseMetadata::asset_url_for_repo(
                "git@github.com:example/schneeforge-fork.git",
                "v1.2.3-rc.1"
            )
            .unwrap(),
            "https://github.com/example/schneeforge-fork/releases/download/v1.2.3-rc.1/schneeforge-release.json"
        );
    }

    #[test]
    fn asset_url_for_repo_rejects_unsupported_repository() {
        let err =
            ReleaseMetadata::asset_url_for_repo("https://gitlab.com/example/schneeforge", "v1.2.3")
                .unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported GitHub repository URL"),
            "{err}"
        );
    }

    #[test]
    fn release_page_url_normalizes_supported_repository_urls() {
        assert_eq!(
            release_page_url(crate::DEFAULT_REPO_URL, "0.2.0-rc.7").unwrap(),
            "https://github.com/Lamy210/nix_setting/releases/tag/v0.2.0-rc.7"
        );
        assert_eq!(
            release_page_url("https://github.com/example/fork", "1.0.0").unwrap(),
            "https://github.com/example/fork/releases/tag/v1.0.0"
        );
        assert_eq!(
            release_page_url("git@github.com:example/fork.git", "1.0.0").unwrap(),
            "https://github.com/example/fork/releases/tag/v1.0.0"
        );
        assert_eq!(
            release_page_url("ssh://git@github.com/example/fork.git", "1.0.0").unwrap(),
            "https://github.com/example/fork/releases/tag/v1.0.0"
        );
    }

    #[test]
    fn release_page_url_rejects_invalid_versions() {
        for version in [
            "",
            "v1.2.3",
            "1.2",
            "1.2.3/../../issues",
            "1.2.3?tab=assets",
        ] {
            let err = release_page_url("https://github.com/example/fork.git", version).unwrap_err();
            assert!(
                err.to_string()
                    .contains("invalid SemVer release version for release page"),
                "{version}: {err}"
            );
        }
    }

    #[test]
    fn release_page_url_rejects_unsupported_repository() {
        let err = release_page_url("https://gitlab.com/example/fork.git", "1.0.0").unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported GitHub repository URL for release page"),
            "{err}"
        );
    }

    #[test]
    fn roundtrip_serialization() {
        let m = sample();
        let json = serde_json::to_string(&m).unwrap();
        let back: ReleaseMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }
}
