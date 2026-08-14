use std::path::PathBuf;

/// Managed Nix 操作の構造化エラー (design.md D10)
///
/// `PartialEq` を導出できるよう、`io::Error` / `ExitStatus` は文字列表現へ変換して保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedNixError {
    UnsupportedArch {
        arch: String,
    },
    ChecksumMismatch {
        expected: String,
        actual: String,
    },
    NetworkRequired,
    ReceiptNotFound {
        path: PathBuf,
    },
    OwnershipNotFound {
        path: PathBuf,
    },
    OwnershipInvalid {
        reason: String,
    },
    Download {
        source: String,
    },
    Subprocess {
        exit_status: Option<i32>,
        stderr_tail: String,
    },
    ManifestParse {
        source: String,
    },
    ReceiptParse {
        source: String,
    },
    PlanFileNotFound {
        path: PathBuf,
    },
    PlannerConflict,
    ExistingNixDetected {
        path: PathBuf,
    },
    Io {
        context: String,
        source: String,
    },
}

impl std::fmt::Display for ManagedNixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagedNixError::UnsupportedArch { arch } => {
                write!(f, "unsupported arch for Managed Nix: {arch}")
            }
            ManagedNixError::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch (expected {expected}, got {actual})")
            }
            ManagedNixError::NetworkRequired => {
                write!(
                    f,
                    "network access required but unavailable, and no cached binary"
                )
            }
            ManagedNixError::ReceiptNotFound { path } => {
                write!(f, "receipt not found: {}", path.display())
            }
            ManagedNixError::OwnershipNotFound { path } => write!(
                f,
                "SchneeForge ownership record not found: {} (not installed by SchneeForge)",
                path.display()
            ),
            ManagedNixError::OwnershipInvalid { reason } => {
                write!(f, "SchneeForge ownership record is invalid: {reason}")
            }
            ManagedNixError::Download { source } => write!(f, "download failed: {source}"),
            ManagedNixError::Subprocess {
                exit_status,
                stderr_tail,
            } => match exit_status {
                Some(code) => write!(
                    f,
                    "nix-installer subprocess exited with {code}: {stderr_tail}"
                ),
                None => write!(f, "nix-installer subprocess failed: {stderr_tail}"),
            },
            ManagedNixError::ManifestParse { source } => {
                write!(f, "bootstrap-manifest parse error: {source}")
            }
            ManagedNixError::ReceiptParse { source } => {
                write!(f, "receipt json parse error: {source}")
            }
            ManagedNixError::PlanFileNotFound { path } => {
                write!(f, "plan file not found: {}", path.display())
            }
            ManagedNixError::PlannerConflict => {
                write!(f, "--plan and planner-subcommand are mutually exclusive")
            }
            ManagedNixError::ExistingNixDetected { path } => write!(
                f,
                "existing Nix detected at {}; refusing Managed Nix install",
                path.display()
            ),
            ManagedNixError::Io { context, source } => {
                write!(f, "io error during {context}: {source}")
            }
        }
    }
}

impl std::error::Error for ManagedNixError {}

impl From<std::io::Error> for ManagedNixError {
    fn from(e: std::io::Error) -> Self {
        ManagedNixError::Io {
            context: "managed nix".to_string(),
            source: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_unsupported_arch() {
        let e = ManagedNixError::UnsupportedArch {
            arch: "x86_64-darwin".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "unsupported arch for Managed Nix: x86_64-darwin"
        );
    }

    #[test]
    fn display_network_required() {
        let e = ManagedNixError::NetworkRequired;
        assert!(e.to_string().contains("network access required"));
    }

    #[test]
    fn display_checksum_mismatch() {
        let e = ManagedNixError::ChecksumMismatch {
            expected: "abc".to_string(),
            actual: "def".to_string(),
        };
        assert!(e.to_string().contains("abc"));
        assert!(e.to_string().contains("def"));
    }
}
