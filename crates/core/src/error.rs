use std::fmt;

use crate::managed_nix::ManagedNixError;
use crate::tool::ToolRequirementError;

/// SchneeForge core の構造化エラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// 非対応 OS / arch
    UnsupportedPlatform { os: String, arch: String },
    /// manifest (config.toml) の読み込み・parse・検証エラー
    Manifest(String),
    /// コマンド実行エラー
    Command { command: String, detail: String },
    /// ファイル入出力エラー (state 保存等)
    Io(String),
    /// 別の操作が進行中のため開始できない
    Busy(String),
    /// 前提条件を満たしていない (Nix 未インストール等)
    Precondition(String),
    /// Managed Nix (nix-installer 統合) のエラー
    ManagedNix(ManagedNixError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnsupportedPlatform { os, arch } => {
                write!(f, "unsupported platform: {os} {arch}")
            }
            Error::Manifest(msg) => write!(f, "manifest error: {msg}"),
            Error::Command { command, detail } => write!(f, "{command}: {detail}"),
            Error::Io(msg) => write!(f, "io error: {msg}"),
            Error::Busy(msg) => write!(f, "busy: {msg}"),
            Error::Precondition(msg) => write!(f, "precondition not met: {msg}"),
            Error::ManagedNix(e) => write!(f, "managed nix error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// `ToolInventory::require_*` の失敗を `Error::Precondition` に統一
impl From<ToolRequirementError> for Error {
    fn from(e: ToolRequirementError) -> Self {
        Error::Precondition(e.to_string())
    }
}

impl From<ManagedNixError> for Error {
    fn from(e: ManagedNixError) -> Self {
        Error::ManagedNix(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn display_unsupported_platform() {
        let e = Error::UnsupportedPlatform {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
        };
        assert_eq!(e.to_string(), "unsupported platform: windows x86_64");
    }

    #[test]
    fn display_command_error() {
        let e = Error::Command {
            command: "nix".to_string(),
            detail: "exited with 1".to_string(),
        };
        assert_eq!(e.to_string(), "nix: exited with 1");
    }

    #[test]
    fn from_managed_nix_error_preserves_message() {
        let inner = ManagedNixError::ReceiptNotFound {
            path: PathBuf::from("/nix/receipt.json"),
        };
        let e: Error = inner.into();
        assert!(e.to_string().contains("receipt not found"));
        assert!(e.to_string().contains("/nix/receipt.json"));
    }

    #[test]
    fn from_managed_nix_network_required() {
        let inner = ManagedNixError::NetworkRequired;
        let e: Error = inner.into();
        assert!(e.to_string().contains("network"));
    }
}
