use std::fmt;

/// 対応プラットフォーム
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux,
    Unsupported,
}

/// ホスト構成名
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host {
    MacbookAir,
    Linux,
    LinuxArm,
    Unsupported,
}

impl Host {
    /// flake の homeConfigurations / darwinConfigurations に一致する名前
    pub fn name(&self) -> &'static str {
        match self {
            Host::MacbookAir => "macbook-air",
            Host::Linux => "linux",
            Host::LinuxArm => "linux-arm",
            Host::Unsupported => "unsupported",
        }
    }

    /// homeDirectory を username から派生
    pub fn home_directory(&self, username: &str) -> String {
        match self {
            Host::MacbookAir => format!("/Users/{username}"),
            Host::Linux | Host::LinuxArm => format!("/home/{username}"),
            Host::Unsupported => String::new(),
        }
    }
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// 実行環境から OS / arch を検出して Host を返す
pub fn detect_host() -> Host {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match os {
        "macos" => match arch {
            "aarch64" => Host::MacbookAir,
            _ => Host::Unsupported,
        },
        "linux" => match arch {
            "aarch64" => Host::LinuxArm,
            "x86_64" => Host::Linux,
            _ => Host::Unsupported,
        },
        _ => Host::Unsupported,
    }
}

/// 実行環境から Platform を検出
pub fn detect_platform() -> Platform {
    match std::env::consts::OS {
        "macos" => Platform::MacOS,
        "linux" => Platform::Linux,
        _ => Platform::Unsupported,
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
    fn home_directory_macos() {
        assert_eq!(Host::MacbookAir.home_directory("alice"), "/Users/alice");
    }

    #[test]
    fn home_directory_linux() {
        assert_eq!(Host::Linux.home_directory("alice"), "/home/alice");
    }

    #[test]
    fn home_directory_linux_arm() {
        assert_eq!(Host::LinuxArm.home_directory("alice"), "/home/alice");
    }

    #[test]
    fn host_name_matches_flake() {
        assert_eq!(Host::MacbookAir.name(), "macbook-air");
        assert_eq!(Host::Linux.name(), "linux");
        assert_eq!(Host::LinuxArm.name(), "linux-arm");
    }
}
