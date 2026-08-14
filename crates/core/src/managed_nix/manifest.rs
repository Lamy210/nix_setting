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

    /// TOML 文字列から parse
    pub fn parse(toml_str: &str) -> Result<Self, ManagedNixError> {
        toml::from_str(toml_str).map_err(|e| ManagedNixError::ManifestParse {
            source: e.to_string(),
        })
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
x86_64-linux = "3b49a0b9deadbeef"
aarch64-linux = "cafebabe"
aarch64-darwin = "feedface"
"#;

    #[test]
    fn parse_manifest() {
        let m = BootstrapManifest::parse(SAMPLE).unwrap();
        assert_eq!(m.managed_nix.version, "2.35.1");
        assert_eq!(
            m.expected_sha256("x86_64-linux").unwrap(),
            "3b49a0b9deadbeef"
        );
        assert_eq!(m.expected_sha256("aarch64-darwin").unwrap(), "feedface");
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
        assert_eq!(m2.expected_sha256("aarch64-linux").unwrap(), "cafebabe");
    }

    #[test]
    fn unknown_arch_returns_none() {
        let m = BootstrapManifest::parse(SAMPLE).unwrap();
        assert!(m.expected_sha256("x86_64-darwin").is_none());
    }
}
