//! Machine 固有情報 (MachineFacts) の検出と Nix への注入。
//!
//! configuration repo に machine 情報を持たせない設計 (v2) の中核。
//! username / home / OS / arch / hostname を実行環境から検出し、
//! `machine.nix` として state dir へ生成する。評価は pure を維持する
//! ため `builtins.getEnv` は使わない (Rust 側で明示管理)。

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::discovery::{detect_arch_for, detect_platform_for, Architecture, Platform};
use crate::error::{Error, Result};

/// atomic input write の一時ファイル名をユニーク化するプロセス内シーケンス。
/// PID と組み合わせることでスレッド間・プロセス間の衝突を避ける。
static ATOMIC_WRITE_TMP_SEQ: AtomicU64 = AtomicU64::new(0);

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
        Self::detect_with_home_from(|k| std::env::var_os(k))
    }

    /// HOME の取得方法を差し込み可能にした detect。
    /// process 全体の env を test から書き換えると並列 test と race する
    /// (「HOME is not set」の失敗を再現する test が他 test を巻き込む) ため、
    /// 該当 test はこの経路で env 無しを simulate する
    fn detect_with_home_from(
        home_from_env: impl Fn(&str) -> Option<std::ffi::OsString>,
    ) -> Result<Self> {
        let username = crate::discovery::current_user()
            .ok_or_else(|| Error::Precondition("username could not be detected".to_string()))?;
        if username.is_empty() {
            return Err(Error::Precondition("username is empty".to_string()));
        }

        let home = home_from_env("HOME")
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
        .filter(|value| !value.as_os_str().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| !value.as_os_str().is_empty())
                .map(|h| PathBuf::from(h).join(".local/state"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("schneeforge")
}

/// facts を machine.nix として state dir へ生成する。常に上書き。
/// temp file + rename で atomic に置き換える (truncate 中の読み取りで
/// 空の file が観測されるのを防ぐ)
pub fn write_machine_input(facts: &MachineFacts) -> Result<PathBuf> {
    write_machine_input_at(&default_machine_nix_path(), facts)
}

/// [`write_machine_input`] の書き込み先指定版 (test 用)
pub fn write_machine_input_at(path: &Path, facts: &MachineFacts) -> Result<PathBuf> {
    atomic_write(path, &facts.to_machine_nix())
        .map_err(|e| Error::Io(format!("write machine input ({e})")))?;
    Ok(path.to_path_buf())
}

/// temp file (PID + process-local sequence) + fsync + rename による atomic 置換。
/// 時刻由来の suffix は同一 file への並列書き込みで衝突し得るため、
/// StateStore と同様に PID と単調 sequence で一意化する。write / fsync /
/// rename のどこで失敗しても destination は置換せず、temp は掃除する。
pub(crate) fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    atomic_write_with_sync(path, content, std::fs::File::sync_all)
}

fn atomic_write_with_sync<F>(path: &Path, content: &str, sync: F) -> std::io::Result<()>
where
    F: FnOnce(&std::fs::File) -> std::io::Result<()>,
{
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let seq = ATOMIC_WRITE_TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("nix.{}.{seq}.tmp", std::process::id()));

    let result = (|| {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(content.as_bytes())?;
        sync(&f)?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
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

/// Nix string literal 内の escape (`"`, `\`, `${`)。改行等は入らない想定。
fn escape_nix_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("${", "\\${")
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
        // XDG_STATE_HOME を設定すると並列 test の書き込み先まで変わって
        // 競合するため、env は操作せず test 固有 path へ書く
        // (既定 path の検証は default_machine_nix_path_is_in_state_dir)
        let dir = std::env::temp_dir().join(format!("sf-machine-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let facts = MachineFacts {
            username: "alice".to_string(),
            home_directory: PathBuf::from("/Users/alice"),
            os: OperatingSystem::MacOS,
            architecture: Architecture::Aarch64,
            hostname: "alice-macbook".to_string(),
        };
        let path = write_machine_input_at(&dir.join("schneeforge/machine.nix"), &facts).unwrap();
        assert!(path.starts_with(&dir));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("username = \"alice\";"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn atomic_write_propagates_sync_failure_without_replacing_destination() {
        let dir =
            std::env::temp_dir().join(format!("sf-machine-fsync-failure-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("machine.nix");
        std::fs::write(&path, "old-machine-input").unwrap();

        let err = atomic_write_with_sync(&path, "new-machine-input", |_| {
            Err(std::io::Error::other("forced fsync failure"))
        })
        .expect_err("fsync failure must abort atomic replacement");

        assert_eq!(err.kind(), std::io::ErrorKind::Other);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old-machine-input");
        let temp_files = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temp_files, 0, "failed writes must not leave temp files");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn atomic_write_parallel_writers_do_not_collide() {
        const WRITERS: usize = 16;
        const WRITES_PER_WRITER: usize = 16;

        let dir =
            std::env::temp_dir().join(format!("sf-machine-parallel-write-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = std::sync::Arc::new(dir.join("machine.nix"));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(WRITERS));

        let handles: Vec<_> = (0..WRITERS)
            .map(|writer| {
                let path = std::sync::Arc::clone(&path);
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || -> std::io::Result<()> {
                    barrier.wait();
                    for write in 0..WRITES_PER_WRITER {
                        atomic_write(&path, &format!("writer-{writer}-write-{write}"))?;
                    }
                    Ok(())
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        assert!(path.is_file());
        let temp_files = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temp_files, 0, "parallel writes must not leak temp files");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_machine_nix_path_is_in_state_dir() {
        let path = default_machine_nix_path();
        assert!(path.starts_with(crate::machine::state_dir()));
        assert!(path.ends_with("machine.nix"));
    }

    #[test]
    fn detect_fails_without_home() {
        // HOME 無しは env 参照の差し込みで再現する (process 全体の HOME を
        // remove すると並列 test と race して他 test を巻き込む)
        let r = MachineFacts::detect_with_home_from(|_| None);
        let err = r.expect_err("detect should fail without HOME");
        assert!(err.to_string().contains("HOME is not set"));
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
