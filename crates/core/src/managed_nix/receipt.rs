use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::managed_nix::error::ManagedNixError;

/// `/nix/receipt.json` の読み取り専用 view (design.md D5)
///
/// 本体 schema は nix-installer `InstallPlan` 構造体に由来するが、SchneeForge 側では
/// version / actions の有無 / planner の presence だけを扱い、内部構造に深く依存しない。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub version: Option<String>,
    /// actions の数 (element は polymorphic なので未型付け)
    #[serde(default)]
    pub actions: Vec<serde_json::Value>,
    /// planner の内容 (Linux / macOS 等の区別)。一部 field だけ抜くため Value。
    #[serde(default)]
    pub planner: Option<serde_json::Value>,
}

impl Receipt {
    /// 指定 path の receipt を読む。存在しない場合は `ReceiptNotFound`
    pub fn load(path: &Path) -> Result<Self, ManagedNixError> {
        if !path.exists() {
            return Err(ManagedNixError::ReceiptNotFound {
                path: path.to_path_buf(),
            });
        }
        let body = fs::read_to_string(path).map_err(|e| ManagedNixError::Io {
            context: format!("read receipt {}", path.display()),
            source: e.to_string(),
        })?;
        let receipt: Receipt = serde_json::from_str(&body).map_err(|e| {
            ManagedNixError::ManifestParse {
                source: format!("receipt json: {e}"),
            }
        })?;
        Ok(receipt)
    }

    /// default の `/nix/receipt.json` を読む (SchneeForge 既定)
    pub fn load_default() -> Result<Self, ManagedNixError> {
        Self::load(&default_receipt_path())
    }

    pub fn default_path() -> PathBuf {
        default_receipt_path()
    }
}

pub fn default_receipt_path() -> PathBuf {
    PathBuf::from("/nix/receipt.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_not_found() {
        let p = Path::new("/__definitely_no_receipt_here__.json");
        let res = Receipt::load(p);
        assert!(matches!(res, Err(ManagedNixError::ReceiptNotFound { .. })));
    }

    #[test]
    fn receipt_parse_minimal() {
        let tmp = std::env::temp_dir().join("schneeforge_receipt_test.json");
        fs::write(&tmp, r#"{"version":"0.1.0","actions":[],"planner":null}"#).unwrap();
        let r = Receipt::load(&tmp).unwrap();
        assert_eq!(r.version.as_deref(), Some("0.1.0"));
        assert!(r.actions.is_empty());
        assert!(r.planner.is_none());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn receipt_parse_with_actions() {
        let tmp = std::env::temp_dir().join("schneeforge_receipt_test2.json");
        fs::write(
            &tmp,
            r#"{"version":"0.2.0","actions":[{"action":"create_user"},{"action":"provision_file"}],"planner":{"planner":"linux"}}"#,
        )
        .unwrap();
        let r = Receipt::load(&tmp).unwrap();
        assert_eq!(r.actions.len(), 2);
        assert!(r.planner.is_some());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn default_receipt_path_is_nix() {
        assert_eq!(default_receipt_path(), PathBuf::from("/nix/receipt.json"));
    }
}
