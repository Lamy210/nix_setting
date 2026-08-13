use std::fmt;

/// OS (Platform) の種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux,
    Unsupported,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Platform::MacOS => "macos",
            Platform::Linux => "linux",
            Platform::Unsupported => "unsupported",
        })
    }
}

/// CPU アーキテクチャ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    Aarch64,
    X86_64,
    Unsupported,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Architecture::Aarch64 => "aarch64",
            Architecture::X86_64 => "x86_64",
            Architecture::Unsupported => "unsupported",
        })
    }
}

/// flake のどの configuration を使うか (ConfigurationTarget)
///
/// Platform / Architecture (実行環境の検出結果) とは独立した概念。
/// `name` は flake の `darwinConfigurations.<name>` / `homeConfigurations.<name>` に
/// 一致する構成名。検出時の既定名は data として持つため、将来 hostname / manifest から
/// 上書きできる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationTarget {
    name: String,
    platform: Platform,
    architecture: Architecture,
}

impl ConfigurationTarget {
    pub fn new(name: impl Into<String>, platform: Platform, architecture: Architecture) -> Self {
        Self {
            name: name.into(),
            platform,
            architecture,
        }
    }

    /// flake の configuration 名 (例: "macbook-air", "linux", "linux-arm")
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn architecture(&self) -> Architecture {
        self.architecture
    }

    /// 対応プラットフォームか (検出可能な OS / arch か)
    pub fn is_supported(&self) -> bool {
        self.platform != Platform::Unsupported && self.architecture != Architecture::Unsupported
    }

    /// homeDirectory を username から派生
    pub fn home_directory(&self, username: &str) -> String {
        match self.platform {
            Platform::MacOS => format!("/Users/{username}"),
            Platform::Linux => format!("/home/{username}"),
            Platform::Unsupported => String::new(),
        }
    }
}

impl fmt::Display for ConfigurationTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// 実行環境から ConfigurationTarget を検出
pub fn detect_target() -> ConfigurationTarget {
    detect_target_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// OS / arch 文字列から ConfigurationTarget を導出する純関数 (テスト可能)
pub fn detect_target_for(os: &str, arch: &str) -> ConfigurationTarget {
    let platform = detect_platform_for(os);
    let architecture = detect_arch_for(arch);
    ConfigurationTarget::new(
        default_target_name(platform, architecture),
        platform,
        architecture,
    )
}

fn default_target_name(platform: Platform, architecture: Architecture) -> &'static str {
    match (platform, architecture) {
        (Platform::MacOS, Architecture::Aarch64) => "macbook-air",
        (Platform::Linux, Architecture::X86_64) => "linux",
        (Platform::Linux, Architecture::Aarch64) => "linux-arm",
        _ => "unsupported",
    }
}

/// 実行環境から Platform を検出
pub fn detect_platform() -> Platform {
    detect_platform_for(std::env::consts::OS)
}

/// OS 文字列から Platform を導出する純関数
pub fn detect_platform_for(os: &str) -> Platform {
    match os {
        "macos" => Platform::MacOS,
        "linux" => Platform::Linux,
        _ => Platform::Unsupported,
    }
}

/// 実行環境から Architecture を検出
pub fn detect_arch() -> Architecture {
    detect_arch_for(std::env::consts::ARCH)
}

/// arch 文字列から Architecture を導出する純関数
pub fn detect_arch_for(arch: &str) -> Architecture {
    match arch {
        "aarch64" => Architecture::Aarch64,
        "x86_64" => Architecture::X86_64,
        _ => Architecture::Unsupported,
    }
}

/// PATH から実行可能ファイルを探す
pub fn which(cmd: &str) -> Option<String> {
    let path = std::env::var("PATH").ok()?;
    for dir in path.split(':') {
        let candidate = format!("{dir}/{cmd}");
        if std::path::Path::new(&candidate).is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Nix がインストールされているか
pub fn has_nix() -> bool {
    which("nix").is_some()
}

/// Homebrew がインストールされているか
pub fn has_homebrew() -> bool {
    which("brew").is_some()
}

/// Git がインストールされているか
pub fn has_git() -> bool {
    which("git").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_from_os() {
        assert_eq!(detect_platform_for("macos"), Platform::MacOS);
        assert_eq!(detect_platform_for("linux"), Platform::Linux);
        assert_eq!(detect_platform_for("windows"), Platform::Unsupported);
    }

    #[test]
    fn architecture_from_arch() {
        assert_eq!(detect_arch_for("aarch64"), Architecture::Aarch64);
        assert_eq!(detect_arch_for("x86_64"), Architecture::X86_64);
        assert_eq!(detect_arch_for("riscv64"), Architecture::Unsupported);
    }

    #[test]
    fn home_directory_by_platform() {
        let target = ConfigurationTarget::new("x", Platform::MacOS, Architecture::Aarch64);
        assert_eq!(target.home_directory("alice"), "/Users/alice");
        let target = ConfigurationTarget::new("x", Platform::Linux, Architecture::X86_64);
        assert_eq!(target.home_directory("alice"), "/home/alice");
        let target = ConfigurationTarget::new("x", Platform::Unsupported, Architecture::Aarch64);
        assert_eq!(target.home_directory("alice"), "");
    }

    #[test]
    fn target_name_matches_flake() {
        assert_eq!(detect_target_for("macos", "aarch64").name(), "macbook-air");
        assert_eq!(detect_target_for("linux", "x86_64").name(), "linux");
        assert_eq!(detect_target_for("linux", "aarch64").name(), "linux-arm");
        assert_eq!(detect_target_for("windows", "x86_64").name(), "unsupported");
    }

    #[test]
    fn target_separates_platform_from_name() {
        // 同一 platform/arch でも ConfigurationTarget の name は独立した data
        let mac_mini = ConfigurationTarget::new("mac-mini", Platform::MacOS, Architecture::Aarch64);
        let macbook_air =
            ConfigurationTarget::new("macbook-air", Platform::MacOS, Architecture::Aarch64);
        assert_eq!(mac_mini.platform(), macbook_air.platform());
        assert_eq!(mac_mini.architecture(), macbook_air.architecture());
        assert_ne!(mac_mini.name(), macbook_air.name());
    }

    #[test]
    fn target_is_supported() {
        assert!(detect_target_for("macos", "aarch64").is_supported());
        assert!(detect_target_for("linux", "x86_64").is_supported());
        assert!(!detect_target_for("windows", "x86_64").is_supported());
        assert!(!detect_target_for("linux", "riscv64").is_supported());
    }

    #[test]
    fn target_display_is_name() {
        assert_eq!(detect_target_for("linux", "x86_64").to_string(), "linux");
    }

    #[test]
    fn detect_target_for_all_platforms() {
        assert_eq!(detect_target_for("macos", "aarch64").name(), "macbook-air");
        assert_eq!(detect_target_for("macos", "x86_64").name(), "unsupported");
        assert_eq!(detect_target_for("linux", "x86_64").name(), "linux");
        assert_eq!(detect_target_for("linux", "aarch64").name(), "linux-arm");
        assert_eq!(detect_target_for("linux", "riscv64").name(), "unsupported");
        assert_eq!(detect_target_for("windows", "x86_64").name(), "unsupported");
        assert_eq!(detect_target_for("freebsd", "x86_64").name(), "unsupported");
    }

    #[test]
    fn which_finds_existing_command() {
        assert!(which("sh").is_some() || which("ls").is_some());
    }

    #[test]
    fn which_returns_none_for_nonexistent() {
        assert!(which("__definitely_not_a_real_command__").is_none());
    }
}
