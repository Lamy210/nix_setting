//! Machine 固有情報 (MachineFacts) の検出と Nix への注入。
//!
//! configuration repo に machine 情報を持たせない設計 (v2) の中核。
//! username / home / OS / arch / hostname を実行環境から検出し、
//! `machine.nix` として state dir へ生成する。評価は pure を維持する
//! ため `builtins.getEnv` は使わない (Rust 側で明示管理)。

use std::fmt;
use std::path::PathBuf;

use crate::discovery::{detect_arch_for, detect_platform_for, Architecture, Platform};
use crate::error::{Error, Result};

/// 実行環境から検出した machine 固有情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineFacts {
    pub username: String,
    pub home_directory: PathBuf,
    pub os: OperatingSystem,
    pub architecture: Architecture,
    pub hostname: String,
}

/// OS 種別 (Platform の machine facts 向け表現)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    MacOS,
    Linux,
}

impl fmt::Display for OperatingSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OperatingSystem::MacOS => "macos",
            OperatingSystem::Linux => "linux",
        })
    }
}

impl MachineFacts {
    /// 実行環境から検出する。username / home が取れない場合は error
    pub fn detect() -> Result<Self> {
        let username = crate::discovery::current_user()
            .ok_or_else(|| Error::Precondition("username could not be detected".to_string()))?;
        if username.is_empty() {
            return Err(Error::Precondition("username is empty".to_string()));
        }

        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| Error::Precondition("HOME is not set".to_string()))?;
        if home.as_os_str().is_empty() {
            return Err(Error::Precondition("HOME is empty".to_string()));
        }

        let platform = detect_platform_for(std::env::consts::OS);
        let architecture = detect_arch_for(std::env::consts::ARCH);
        let os = match platform {
            Platform::MacOS => OperatingSystem::MacOS,
            Platform::Linux => OperatingSystem::Linux,
            Platform::Unsupported => {
                return Err(Error::Precondition(format!(
                    "unsupported platform: {}",
                    std::env::consts::OS
                )))
            }
        };
        if architecture == Architecture::Unsupported {
            return Err(Error::Precondition(format!(
                "unsupported architecture: {}",
                std::env::consts::ARCH
            )));
        }

        Ok(Self {
            username,
            home_directory: home,
            os,
            architecture,
            hostname: hostname(),
        })
    }

    /// machine input (`machine.nix`) の中身を生成する
    pub fn to_machine_nix(&self) -> String {
        format!(
            "{{\n  username = \"{username}\";\n  homeDirectory = \"{home}\";\n  system = \"{system}\";\n  hostname = \"{hostname}\";\n}}\n",
            username = escape_nix_string(&self.username),
            home = escape_nix_string(&self.home_directory.to_string_lossy()),
            system = self.nix_system_string(),
            hostname = escape_nix_string(&self.hostname),
        )
    }

    /// Nix system string (`aarch64-darwin` 等)
    pub fn nix_system_string(&self) -> String {
        let arch = match self.architecture {
            Architecture::Aarch64 => "aarch64",
            Architecture::X86_64 => "x86_64",
            Architecture::Unsupported => "unsupported",
        };
        let os = match self.os {
            OperatingSystem::MacOS => "darwin",
            OperatingSystem::Linux => "linux",
        };
        format!("{arch}-{os}")
    }
}

/// state dir における machine input の既定 path
pub fn default_machine_nix_path() -> PathBuf {
    state_dir().join("machine.nix")
}

/// state dir (`XDG_STATE_HOME/schneeforge` or `~/.local/state/schneeforge`)
pub fn state_dir() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("schneeforge")
}

/// facts を machine.nix として state dir へ生成する。常に上書き
pub fn write_machine_input(facts: &MachineFacts) -> Result<PathBuf> {
    let path = default_machine_nix_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Io(format!("create machine input dir: {e}")))?;
    }
    std::fs::write(&path, facts.to_machine_nix())
        .map_err(|e| Error::Io(format!("write machine input: {e}")))?;
    Ok(path)
}

fn hostname() -> String {
    if let Ok(h) = std::env::var("HOSTNAME") {
        if !h.is_empty() {
            return h;
        }
    }
    hostname_via_command().unwrap_or_else(|| "unknown".to_string())
}

#[cfg(unix)]
fn hostname_via_command() -> Option<String> {
    let out = std::process::Command::new("hostname").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(not(unix))]
fn hostname_via_command() -> Option<String> {
    None
}

/// Nix string literal 内の escape (`"` と `\` のみ。改行等は入らない想定)
fn escape_nix_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_nix_contains_facts() {
        let facts = MachineFacts {
            username: "alice".to_string(),
            home_directory: PathBuf::from("/Users/alice"),
            os: OperatingSystem::MacOS,
            architecture: Architecture::Aarch64,
            hostname: "alice-macbook".to_string(),
        };
        let nix = facts.to_machine_nix();
        assert!(nix.contains("username = \"alice\";"));
        assert!(nix.contains("homeDirectory = \"/Users/alice\";"));
        assert!(nix.contains("system = \"aarch64-darwin\";"));
        assert!(nix.contains("hostname = \"alice-macbook\";"));
    }

    #[test]
    fn machine_nix_escapes_special_chars() {
        let facts = MachineFacts {
            username: "us\"er\\x".to_string(),
            home_directory: PathBuf::from("/Users/us\"er"),
            os: OperatingSystem::Linux,
            architecture: Architecture::X86_64,
            hostname: "host".to_string(),
        };
        let nix = facts.to_machine_nix();
        assert!(nix.contains("username = \"us\\\"er\\\\x\";"));
    }

    #[test]
    fn write_machine_input_creates_file_in_state_dir() {
        let dir = std::env::temp_dir().join(format!("sf-machine-test-{}", std::process::id()));
        std::env::set_var("XDG_STATE_HOME", &dir);
        let facts = MachineFacts {
            username: "alice".to_string(),
            home_directory: PathBuf::from("/Users/alice"),
            os: OperatingSystem::MacOS,
            architecture: Architecture::Aarch64,
            hostname: "alice-macbook".to_string(),
        };
        let path = write_machine_input(&facts).unwrap();
        assert!(path.starts_with(&dir));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("username = \"alice\";"));
        std::fs::remove_dir_all(&dir).ok();
        std::env::remove_var("XDG_STATE_HOME");
    }

    #[test]
    fn detect_fails_without_home() {
        let home = std::env::var_os("HOME");
        std::env::remove_var("HOME");
        let r = MachineFacts::detect();
        std::env::set_var("HOME", home.unwrap_or_default());
        assert!(r.is_err());
    }

    #[test]
    fn system_string_variants() {
        let mk = |os: OperatingSystem, arch: Architecture| MachineFacts {
            username: "u".to_string(),
            home_directory: PathBuf::from("/home/u"),
            os,
            architecture: arch,
            hostname: "h".to_string(),
        };
        assert_eq!(
            mk(OperatingSystem::MacOS, Architecture::Aarch64).nix_system_string(),
            "aarch64-darwin"
        );
        assert_eq!(
            mk(OperatingSystem::Linux, Architecture::X86_64).nix_system_string(),
            "x86_64-linux"
        );
    }
}
