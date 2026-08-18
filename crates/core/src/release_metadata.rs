//! Release Metadata (v2 §27) — release asset `schneeforge-release.json` の
//! parse・検証・取得。release の version / channel / source revision /
//! 最低限必要な schneeforge 版数 / 対応 systems を machine-readable に表現し、
//! GUI Dashboard (§28) の Installed / Available 表示の基盤になる。

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::managed_nix::download_text;

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

/// prerelease suffix (`-rc.N` / `-beta.N` 等) の有無から channel を導出する。
/// 生成 script (release_metadata.py) の `re.search(r"-\w+")` と同じ規則。
pub fn channel_for_version(version: &str) -> &'static str {
    if has_prerelease_suffix(version) {
        "preview"
    } else {
        "stable"
    }
}

/// `-` の後に word character が続くか (生成 script の `-\w+` 相当)
fn has_prerelease_suffix(version: &str) -> bool {
    version.split_once('-').is_some_and(|(_, suffix)| {
        suffix
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
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
        if self.version != expected_version {
            return Err(Error::ReleaseMetadata(format!(
                "version {} does not match tag {tag}",
                self.version
            )));
        }
        let expected_channel = channel_for_version(&self.version);
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

    /// release tag の metadata asset URL
    pub fn asset_url(tag: &str) -> String {
        format!("https://github.com/Lamy210/nix_setting/releases/download/{tag}/{METADATA_ASSET}")
    }

    /// 指定 tag の metadata を GitHub release asset から取得して parse・検証する。
    /// asset が存在しない release (metadata 導入前の release や存在しない tag) や
    /// network error は fail-closed に error。
    pub fn fetch(tag: &str) -> Result<Self> {
        if !tag.starts_with('v') {
            return Err(Error::ReleaseMetadata(format!(
                "tag must start with 'v': {tag}"
            )));
        }
        let text = download_text(&Self::asset_url(tag)).map_err(Error::ManagedNix)?;
        let metadata = Self::parse(&text)?;
        metadata.validate(tag)?;
        Ok(metadata)
    }
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
        assert_eq!(channel_for_version("0.2.0"), "stable");
        assert_eq!(channel_for_version("1.0.0"), "stable");
    }

    #[test]
    fn validate_accepts_consistent_metadata() {
        sample().validate("v0.2.0-rc.5").unwrap();
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
    fn asset_url_shape() {
        assert_eq!(
            ReleaseMetadata::asset_url("v0.2.0-rc.5"),
            "https://github.com/Lamy210/nix_setting/releases/download/v0.2.0-rc.5/schneeforge-release.json"
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
