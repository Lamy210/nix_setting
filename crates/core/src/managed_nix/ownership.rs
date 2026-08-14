//! SchneeForge ownership record (design.md review: uninstall 対称性)
//!
//! `/nix/receipt.json` は upstream への revert 手段の source of truth だが、
//! 「SchneeForge が install したものか」の source of truth にはならない。
//! SchneeForge 経由で install した場合のみ `/nix/schneeforge-managed.json`
//! へ ownership record を残し、uninstall 時に確認する。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::managed_nix::error::ManagedNixError;

/// SchneeForge が install を所有していることを示す record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnershipRecord {
    pub schema: u32,
    pub provider: String,
    pub installer_version: String,
    /// 検証済み installer binary の SHA256 (64 hex)。uninstall 時の
    /// cached binary 再検証に使うため必須 (fail-closed)。
    pub installer_sha256: String,
    pub upstream_receipt: PathBuf,
    pub installed_by: String,
    /// install 完了時刻 (ISO 8601)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<String>,
}

pub const OWNERSHIP_SCHEMA: u32 = 1;

pub fn default_ownership_path() -> PathBuf {
    PathBuf::from("/nix/schneeforge-managed.json")
}

impl OwnershipRecord {
    pub fn new(installer_version: &str, installer_sha256: String) -> Self {
        Self {
            schema: OWNERSHIP_SCHEMA,
            provider: "nixos-nix-installer".to_string(),
            installer_version: installer_version.to_string(),
            installer_sha256,
            upstream_receipt: crate::managed_nix::receipt::default_receipt_path(),
            installed_by: "schneeforge".to_string(),
            installed_at: Some(crate::time::now_iso8601()),
        }
    }

    pub fn load(path: &Path) -> Result<Self, ManagedNixError> {
        if !path.exists() {
            return Err(ManagedNixError::OwnershipNotFound {
                path: path.to_path_buf(),
            });
        }
        let body = fs::read_to_string(path).map_err(|e| ManagedNixError::Io {
            context: format!("read ownership record {}", path.display()),
            source: e.to_string(),
        })?;
        let rec: OwnershipRecord =
            serde_json::from_str(&body).map_err(|e| ManagedNixError::ReceiptParse {
                source: format!("ownership record json: {e}"),
            })?;
        rec.validate()?;
        Ok(rec)
    }

    /// ownership の根拠として成立しているか検証する。
    /// JSON parse だけでは「誰かが置いた file」を ownership と認めてしまう。
    pub fn validate(&self) -> Result<(), ManagedNixError> {
        if self.schema != OWNERSHIP_SCHEMA {
            return Err(ManagedNixError::OwnershipInvalid {
                reason: format!(
                    "unsupported schema {} (expected {OWNERSHIP_SCHEMA})",
                    self.schema
                ),
            });
        }
        if self.installed_by != "schneeforge" {
            return Err(ManagedNixError::OwnershipInvalid {
                reason: format!("installed_by is {:?}, not schneeforge", self.installed_by),
            });
        }
        if self.provider != "nixos-nix-installer" {
            return Err(ManagedNixError::OwnershipInvalid {
                reason: format!("unexpected provider {:?}", self.provider),
            });
        }
        if self.installer_version.is_empty() {
            return Err(ManagedNixError::OwnershipInvalid {
                reason: "installer_version is empty".to_string(),
            });
        }
        // SHA は uninstall 時の cached binary 再検証に必須 (fail-closed)。
        // 64 文字 hex であることも検証する。
        if self.installer_sha256.len() != 64
            || !self.installer_sha256.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(ManagedNixError::OwnershipInvalid {
                reason: format!(
                    "installer_sha256 must be 64 hex chars, got len {}",
                    self.installer_sha256.len()
                ),
            });
        }
        if self.upstream_receipt != crate::managed_nix::receipt::default_receipt_path() {
            return Err(ManagedNixError::OwnershipInvalid {
                reason: format!(
                    "unexpected upstream_receipt {}",
                    self.upstream_receipt.display()
                ),
            });
        }
        Ok(())
    }

    /// install 成功後に呼ぶ。root 所有の 0644 で書く。
    pub fn write(&self, path: &Path) -> Result<(), ManagedNixError> {
        let body =
            serde_json::to_string_pretty(self).map_err(|e| ManagedNixError::ReceiptParse {
                source: format!("serialize ownership record: {e}"),
            })?;
        // atomic にするため tmp → rename
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, body + "\n").map_err(|e| ManagedNixError::Io {
            context: format!("write {}", tmp.display()),
            source: e.to_string(),
        })?;
        set_owner_only_readable(&tmp);
        fs::rename(&tmp, path).map_err(|e| ManagedNixError::Io {
            context: format!("rename {} -> {}", tmp.display(), path.display()),
            source: e.to_string(),
        })?;
        Ok(())
    }

    /// uninstall 時に呼ぶ。record を削除する。
    pub fn remove(path: &Path) -> Result<(), ManagedNixError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ManagedNixError::Io {
                context: format!("remove {}", path.display()),
                source: e.to_string(),
            }),
        }
    }
}

fn set_owner_only_readable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o644));
    }
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn tmp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "schneeforge_ownership_{name}{}.json",
            std::process::id()
        ))
    }

    #[test]
    fn roundtrip_write_and_load() {
        let p = tmp_path("roundtrip");
        let rec = OwnershipRecord::new("2.35.1", VALID_SHA.to_string());
        rec.write(&p).unwrap();
        let loaded = OwnershipRecord::load(&p).unwrap();
        assert_eq!(loaded, rec);
        assert_eq!(loaded.schema, 1);
        assert_eq!(loaded.provider, "nixos-nix-installer");
        assert_eq!(loaded.installed_by, "schneeforge");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn load_missing_is_ownership_not_found() {
        let res = OwnershipRecord::load(Path::new("/__no_such_ownership__.json"));
        assert!(matches!(
            res,
            Err(ManagedNixError::OwnershipNotFound { .. })
        ));
    }

    #[test]
    fn load_invalid_json_is_parse_error() {
        let p = tmp_path("invalid");
        fs::write(&p, "not json").unwrap();
        let res = OwnershipRecord::load(&p);
        assert!(matches!(res, Err(ManagedNixError::ReceiptParse { .. })));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn remove_missing_is_ok() {
        assert!(OwnershipRecord::remove(Path::new("/__no_such_ownership__.json")).is_ok());
    }

    #[test]
    fn validate_rejects_wrong_installed_by() {
        let mut rec = OwnershipRecord::new("2.35.1", VALID_SHA.to_string());
        rec.installed_by = "someone-else".to_string();
        assert!(matches!(
            rec.validate(),
            Err(ManagedNixError::OwnershipInvalid { .. })
        ));
    }

    #[test]
    fn validate_rejects_wrong_schema() {
        let mut rec = OwnershipRecord::new("2.35.1", VALID_SHA.to_string());
        rec.schema = 99;
        assert!(matches!(
            rec.validate(),
            Err(ManagedNixError::OwnershipInvalid { .. })
        ));
    }

    #[test]
    fn validate_rejects_wrong_provider() {
        let mut rec = OwnershipRecord::new("2.35.1", VALID_SHA.to_string());
        rec.provider = "other-installer".to_string();
        assert!(matches!(
            rec.validate(),
            Err(ManagedNixError::OwnershipInvalid { .. })
        ));
    }

    #[test]
    fn validate_rejects_empty_version() {
        let mut rec = OwnershipRecord::new("", VALID_SHA.to_string());
        rec.installer_version = String::new();
        assert!(matches!(
            rec.validate(),
            Err(ManagedNixError::OwnershipInvalid { .. })
        ));
    }

    #[test]
    fn load_rejects_spoofed_record() {
        let p = tmp_path("spoof");
        // JSON としては有効だが installed_by が偽の record
        fs::write(
            &p,
            r#"{"schema":1,"provider":"nixos-nix-installer","installer_version":"2.35.1","installer_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","upstream_receipt":"/nix/receipt.json","installed_by":"attacker"}"#,
        )
        .unwrap();
        let res = OwnershipRecord::load(&p);
        assert!(matches!(res, Err(ManagedNixError::OwnershipInvalid { .. })));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn validate_rejects_missing_sha() {
        // installer_sha256 が無い (旧 fail-open 形式) record は拒否
        let mut rec = OwnershipRecord::new("2.35.1", VALID_SHA.to_string());
        rec.installer_sha256 = String::new();
        assert!(matches!(
            rec.validate(),
            Err(ManagedNixError::OwnershipInvalid { .. })
        ));
    }

    #[test]
    fn validate_rejects_short_sha() {
        let mut rec = OwnershipRecord::new("2.35.1", VALID_SHA.to_string());
        rec.installer_sha256 = "abc123".to_string();
        assert!(matches!(
            rec.validate(),
            Err(ManagedNixError::OwnershipInvalid { .. })
        ));
    }

    #[test]
    fn load_rejects_record_without_sha_field() {
        let p = tmp_path("no_sha");
        // installer_sha256 field 自体が無い record (schema 1 の旧形式)
        fs::write(
            &p,
            r#"{"schema":1,"provider":"nixos-nix-installer","installer_version":"2.35.1","upstream_receipt":"/nix/receipt.json","installed_by":"schneeforge"}"#,
        )
        .unwrap();
        let res = OwnershipRecord::load(&p);
        assert!(res.is_err());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn remove_existing() {
        let p = tmp_path("remove");
        let rec = OwnershipRecord::new("2.35.1", VALID_SHA.to_string());
        rec.write(&p).unwrap();
        assert!(p.exists());
        OwnershipRecord::remove(&p).unwrap();
        assert!(!p.exists());
    }
}
