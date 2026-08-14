use serde::{Deserialize, Serialize};

use crate::managed_nix::error::ManagedNixError;

/// bootstrap-manifest.toml の SchneeForge 側 schema (design.md D3, tasks 2.2)
///
/// ```toml
/// [managed_nix]
/// version = "2.35.1"
///
/// [managed_nix.sha256_by_arch]
/// x86_64-linux = "3b49..."
/// aarch64-linux = "..."
/// aarch64-darwin = "..."
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapManifest {
    pub managed_nix: ManagedNixSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedNixSection {
    pub version: String,
    pub sha256_by_arch: Sha256ByArch,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sha256ByArch {
    #[serde(rename = "x86_64-linux")]
    pub x86_64_linux: Option<String>,
    #[serde(rename = "aarch64-linux")]
    pub aarch64_linux: Option<String>,
    #[serde(rename = "aarch64-darwin")]
    pub aarch64_darwin: Option<String>,
}

impl Sha256ByArch {
    pub fn get(&self, arch_name: &str) -> Option<&str> {
        match arch_name {
            "x86_64-linux" => self.x86_64_linux.as_deref(),
            "aarch64-linux" => self.aarch64_linux.as_deref(),
            "aarch64-darwin" => self.aarch64_darwin.as_deref(),
            _ => None,
        }
    }

    pub fn set(&mut self, arch_name: &str, value: String) {
        match arch_name {
            "x86_64-linux" => self.x86_64_linux = Some(value),
            "aarch64-linux" => self.aarch64_linux = Some(value),
            "aarch64-darwin" => self.aarch64_darwin = Some(value),
            _ => {}
        }
    }
}

impl BootstrapManifest {
    /// 指定 arch_name の expected sha256 を取り出す
    pub fn expected_sha256(&self, arch_name: &str) -> Option<&str> {
        self.managed_nix.sha256_by_arch.get(arch_name)
    }

    /// manifest 値の妥当性を検証する。
    /// user-controlled な malformed manifest がそのまま privileged downloader へ
    /// 入力される境界を作らないため (design review)。
    ///   - version: `X.Y.Z` 形式 (semver の major.minor.patch 部分と同等)
    ///   - sha256: ちょうど 64 文字の hex
    pub fn validate(&self) -> Result<(), ManagedNixError> {
        let version = &self.managed_nix.version;
        let parts: Vec<&str> = version.split('.').collect();
        let valid_version = parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            && parts.iter().all(|p| p.parse::<u32>().is_ok());
        if !valid_version {
            return Err(ManagedNixError::ManifestParse {
                source: format!(
                    "version {:?} is not X.Y.Z numeric form (got {parts:?})",
                    self.managed_nix.version
                ),
            });
        }
        for (arch, sha) in [
            (
                "x86_64-linux",
                &self.managed_nix.sha256_by_arch.x86_64_linux,
            ),
            (
                "aarch64-linux",
                &self.managed_nix.sha256_by_arch.aarch64_linux,
            ),
            (
                "aarch64-darwin",
                &self.managed_nix.sha256_by_arch.aarch64_darwin,
            ),
        ] {
            if let Some(sha) = sha {
                let valid_len = sha.len() == 64;
                let valid_hex = sha.chars().all(|c| c.is_ascii_hexdigit());
                if !valid_len || !valid_hex {
                    return Err(ManagedNixError::ManifestParse {
                        source: format!(
                            "sha256 for {arch} must be 64 hex chars, got {:?} (len {})",
                            sha,
                            sha.len()
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    /// TOML 文字列から parse して validate
    pub fn parse(toml_str: &str) -> Result<Self, ManagedNixError> {
        let m: BootstrapManifest =
            toml::from_str(toml_str).map_err(|e| ManagedNixError::ManifestParse {
                source: e.to_string(),
            })?;
        m.validate()?;
        Ok(m)
    }

    /// TOML 文字列へ serialize (CI の bump workflow 用)
    pub fn to_toml(&self) -> Result<String, ManagedNixError> {
        toml::to_string_pretty(self).map_err(|e| ManagedNixError::ManifestParse {
            source: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[managed_nix]
version = "2.35.1"

[managed_nix.sha256_by_arch]
x86_64-linux = "1111111111111111111111111111111111111111111111111111111111111111"
aarch64-linux = "2222222222222222222222222222222222222222222222222222222222222222"
aarch64-darwin = "3333333333333333333333333333333333333333333333333333333333333333"
"#;

    #[test]
    fn parse_manifest() {
        let m = BootstrapManifest::parse(SAMPLE).unwrap();
        assert_eq!(m.managed_nix.version, "2.35.1");
        assert_eq!(
            m.expected_sha256("x86_64-linux").unwrap(),
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(
            m.expected_sha256("aarch64-darwin").unwrap(),
            "3333333333333333333333333333333333333333333333333333333333333333"
        );
    }

    #[test]
    fn parse_invalid_manifest() {
        let res = BootstrapManifest::parse("not toml {{{");
        assert!(matches!(res, Err(ManagedNixError::ManifestParse { .. })));
    }

    #[test]
    fn roundtrip_serialize() {
        let m = BootstrapManifest::parse(SAMPLE).unwrap();
        let toml_str = m.to_toml().unwrap();
        let m2 = BootstrapManifest::parse(&toml_str).unwrap();
        assert_eq!(m2.managed_nix.version, "2.35.1");
        assert_eq!(
            m2.expected_sha256("aarch64-linux").unwrap(),
            "2222222222222222222222222222222222222222222222222222222222222222"
        );
    }

    #[test]
    fn unknown_arch_returns_none() {
        let m = BootstrapManifest::parse(SAMPLE).unwrap();
        assert!(m.expected_sha256("x86_64-darwin").is_none());
    }

    #[test]
    fn validate_rejects_short_sha256() {
        let bad = r#"
[managed_nix]
version = "2.35.1"

[managed_nix.sha256_by_arch]
x86_64-linux = "deadbeef"
aarch64-linux = "2222222222222222222222222222222222222222222222222222222222222222"
aarch64-darwin = "3333333333333333333333333333333333333333333333333333333333333333"
"#;
        let res = BootstrapManifest::parse(bad);
        assert!(matches!(res, Err(ManagedNixError::ManifestParse { .. })));
    }

    #[test]
    fn validate_rejects_non_semver_version() {
        let bad = r#"
[managed_nix]
version = "v2.35"

[managed_nix.sha256_by_arch]
x86_64-linux = "1111111111111111111111111111111111111111111111111111111111111111"
aarch64-linux = "2222222222222222222222222222222222222222222222222222222222222222"
aarch64-darwin = "3333333333333333333333333333333333333333333333333333333333333333"
"#;
        let res = BootstrapManifest::parse(bad);
        assert!(matches!(res, Err(ManagedNixError::ManifestParse { .. })));
    }

    #[test]
    fn validate_accepts_partial_arch_entries() {
        // 全 arch 分の entry が無くても、存在するものが正しければ OK
        // (unsupported arch は expected_sha256 が None → UnsupportedArch で弾かれる)
        let partial = r#"
[managed_nix]
version = "2.35.1"

[managed_nix.sha256_by_arch]
x86_64-linux = "1111111111111111111111111111111111111111111111111111111111111111"
"#;
        assert!(BootstrapManifest::parse(partial).is_ok());
    }
}
