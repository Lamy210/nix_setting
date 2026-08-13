use serde::Deserialize;

use crate::error::{Error, Result};

/// nix_setting manifest (config.toml)
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub user: User,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub username: String,
}

/// Manifest の実行時検証結果
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// 実行時検証: schema == 1, username 非空, 実行ユーザー一致 (不一致は警告)
    pub fn validate(&self, running_user: Option<&str>) -> Validation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if self.schema != 1 {
            errors.push(format!("unsupported schema version: {}", self.schema));
        }
        if self.user.username.is_empty() {
            errors.push("username is empty".to_string());
        }
        if let Some(running) = running_user {
            if !self.user.username.is_empty() && self.user.username != running {
                warnings.push(format!(
                    "config username '{}' differs from running user '{}'",
                    self.user.username, running
                ));
            }
        }

        Validation {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest() {
        let m = Manifest::parse(
            r#"
schema = 1

[user]
username = "alice"
"#,
        )
        .unwrap();
        assert_eq!(m.schema, 1);
        assert_eq!(m.user.username, "alice");
    }

    #[test]
    fn parse_rejects_bad_schema() {
        let r = Manifest::parse("schema = \"not-an-int\"");
        assert!(r.is_err());
    }

    #[test]
    fn parse_rejects_missing_user() {
        let r = Manifest::parse("schema = 1");
        assert!(r.is_err());
    }

    #[test]
    fn parse_rejects_missing_username() {
        let r = Manifest::parse("schema = 1\n[user]\n");
        assert!(r.is_err());
    }

    #[test]
    fn parse_allows_empty_username_but_validate_rejects() {
        let r = Manifest::parse("schema = 1\n[user]\nusername = \"\"\n");
        assert!(r.is_ok());
        let m = r.unwrap();
        let v = m.validate(Some("alice"));
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("empty")));
    }

    #[test]
    fn validate_accepts_valid_manifest() {
        let m = Manifest::parse("schema = 1\n[user]\nusername = \"alice\"\n").unwrap();
        let v = m.validate(Some("alice"));
        assert!(v.valid);
        assert!(v.errors.is_empty());
        assert!(v.warnings.is_empty());
    }

    #[test]
    fn validate_rejects_wrong_schema() {
        let m = Manifest::parse("schema = 2\n[user]\nusername = \"alice\"\n").unwrap();
        let v = m.validate(Some("alice"));
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("schema")));
    }

    #[test]
    fn validate_warns_on_username_mismatch() {
        let m = Manifest::parse("schema = 1\n[user]\nusername = \"alice\"\n").unwrap();
        let v = m.validate(Some("bob"));
        assert!(v.valid);
        assert!(v.warnings.iter().any(|w| w.contains("differs")));
    }
}
