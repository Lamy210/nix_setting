use crate::discovery::{Architecture, Platform};
use crate::managed_nix::error::ManagedNixError;

/// nix-installer binary の取得元 (design.md D2)
pub struct Provider {
    base_url: &'static str,
}

impl Provider {
    pub fn new() -> Self {
        Self {
            // v2.34.5 以降は SHA256SUMS と SLSA attestation が添付されている。
            // tag は "v" 無し。
            base_url: "https://github.com/NixOS/nix-installer/releases/download",
        }
    }

    /// 指定 version / platform / arch の binary asset URL と arch 名を返す
    pub fn asset(
        &self,
        version: &str,
        platform: Platform,
        arch: Architecture,
    ) -> Result<Asset, ManagedNixError> {
        let arch_name = arch_asset_name(platform, arch)?;
        Ok(Asset {
            url: format!("{}/{}/nix-installer-{}", self.base_url, version, arch_name),
            arch_name: arch_name.to_string(),
        })
    }

    /// SHA256SUMS の URL
    pub fn sha256_sums_url(&self, version: &str) -> String {
        format!("{}/{}/SHA256SUMS", self.base_url, version)
    }
}

impl Default for Provider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Asset {
    pub url: String,
    pub arch_name: String,
}

/// NixOS/nix-installer の asset 命名規則に変換
fn arch_asset_name(
    platform: Platform,
    arch: Architecture,
) -> Result<&'static str, ManagedNixError> {
    match (platform, arch) {
        (Platform::Linux, Architecture::X86_64) => Ok("x86_64-linux"),
        (Platform::Linux, Architecture::Aarch64) => Ok("aarch64-linux"),
        (Platform::MacOS, Architecture::Aarch64) => Ok("aarch64-darwin"),
        _ => Err(ManagedNixError::UnsupportedArch {
            arch: format!("{platform}-{arch}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_x86_64_asset() {
        let p = Provider::new();
        let a = p
            .asset("2.35.1", Platform::Linux, Architecture::X86_64)
            .unwrap();
        assert_eq!(a.arch_name, "x86_64-linux");
        assert!(a.url.ends_with("/2.35.1/nix-installer-x86_64-linux"));
    }

    #[test]
    fn macos_aarch64_asset() {
        let p = Provider::new();
        let a = p
            .asset("2.35.1", Platform::MacOS, Architecture::Aarch64)
            .unwrap();
        assert_eq!(a.arch_name, "aarch64-darwin");
        assert!(a.url.ends_with("/2.35.1/nix-installer-aarch64-darwin"));
    }

    #[test]
    fn x86_64_darwin_is_unsupported() {
        let p = Provider::new();
        let e = p
            .asset("2.35.1", Platform::MacOS, Architecture::X86_64)
            .unwrap_err();
        match e {
            ManagedNixError::UnsupportedArch { arch } => {
                assert!(arch.contains("macos"));
            }
            _ => panic!("expected UnsupportedArch"),
        }
    }

    #[test]
    fn sha256_sums_url() {
        let p = Provider::new();
        assert!(p.sha256_sums_url("2.35.1").ends_with("/2.35.1/SHA256SUMS"));
    }
}
