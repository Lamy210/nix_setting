use std::fmt;

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
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

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
}
