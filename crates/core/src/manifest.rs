use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// nix_setting manifest (config.toml)。
/// v2 では machine 情報 (`[user]`) を持たない。distribution 情報
/// (`schneeforge.toml`) への置換は後続 change で行うため、この
/// change では username 読み込みを廃止した状態のみ。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub schema: u32,
    /// v1 互換: `[user]` が在れば読むが検証には使わない
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub username: String,
}

/// Manifest の実行時検証結果
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Validation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Manifest {
    pub fn parse(content: &str) -> std::result::Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// repository の config.toml を読み込み・parse する (構造化エラーを返す)
    pub fn load(repo: &str) -> Result<Self> {
        let content = std::fs::read_to_string(format!("{repo}/config.toml"))
            .map_err(|e| Error::Manifest(format!("failed to read {repo}/config.toml: {e}")))?;
        Self::parse(&content)
            .map_err(|e| Error::Manifest(format!("failed to parse config.toml: {e}")))
    }

    /// 実行時検証: schema == 1。machine 情報は検証しない (MachineFacts で管理)
    pub fn validate(&self) -> Validation {
        let errors = if self.schema == 1 {
            Vec::new()
        } else {
            vec![format!("unsupported schema version: {}", self.schema)]
        };
        Validation {
            valid: errors.is_empty(),
            errors,
            warnings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest_without_user() {
        let m = Manifest::parse("schema = 1\n").unwrap();
        assert_eq!(m.schema, 1);
        assert!(m.user.is_none());
    }

    #[test]
    fn parse_manifest_with_legacy_user_ignored() {
        // v1 config.toml (username あり) も parse は通る。値は使わない
        let m = Manifest::parse("schema = 1\n\n[user]\nusername = \"alice\"\n").unwrap();
        assert_eq!(m.schema, 1);
        assert_eq!(m.user.as_ref().unwrap().username, "alice");
        let v = m.validate();
        assert!(v.valid);
    }

    #[test]
    fn parse_rejects_bad_schema() {
        let r = Manifest::parse("schema = \"not-an-int\"");
        assert!(r.is_err());
    }

    #[test]
    fn validate_rejects_wrong_schema() {
        let m = Manifest::parse("schema = 2\n").unwrap();
        let v = m.validate();
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("schema")));
    }

    #[test]
    fn validate_accepts_schema_only_manifest() {
        let m = Manifest::parse("schema = 1\n").unwrap();
        let v = m.validate();
        assert!(v.valid);
    }
}
