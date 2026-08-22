use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Distribution manifest (`schneeforge.toml`)。
/// repository が「何を提供するか」(distribution 名 / profiles / 対応
/// systems) を記述する。machine 情報 (username 等) は持たない
/// (MachineFacts + machine input で管理)。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub schema: u32,
    #[serde(default)]
    pub distribution: Distribution,
    #[serde(default)]
    pub profiles: Profiles,
    /// 対応 system 名 -> 有効か の map (例: `aarch64-darwin = true`)
    #[serde(default)]
    pub systems: std::collections::BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Distribution {
    /// distribution の表示名
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Profiles {
    /// available の中から既定で選択される profile
    #[serde(default)]
    pub default: Option<String>,
    /// 提供する profile の一覧
    #[serde(default)]
    pub available: Vec<String>,
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

    /// repository の schneeforge.toml を読み込み・parse する (構造化エラーを返す)
    pub fn load(repo: &str) -> Result<Self> {
        let content = std::fs::read_to_string(format!("{repo}/schneeforge.toml"))
            .map_err(|e| Error::Manifest(format!("failed to read {repo}/schneeforge.toml: {e}")))?;
        Self::parse(&content)
            .map_err(|e| Error::Manifest(format!("failed to parse schneeforge.toml: {e}")))
    }

    /// 実行時検証。`system` は実行環境の system (例: "aarch64-darwin")。
    pub fn validate(&self, system: &str) -> Validation {
        let mut errors = Vec::new();
        if self.schema != 1 {
            errors.push(format!("unsupported schema version: {}", self.schema));
        }
        match (&self.profiles.default, &self.profiles.available) {
            (Some(default), available) if !available.is_empty() => {
                if !available.contains(default) {
                    errors.push(format!(
                        "default profile '{default}' is not in available profiles"
                    ));
                }
            }
            (Some(_), _) => {
                errors.push("profiles.available must not be empty when default is set".to_string());
            }
            (None, _) => {
                errors.push("profiles.default is required".to_string());
            }
        }
        if !self.systems.values().any(|v| *v) {
            errors.push("no supported systems are enabled in [systems]".to_string());
        }
        match self.systems.get(system) {
            Some(true) => {}
            Some(false) => errors.push(format!("system '{system}' is disabled in [systems]")),
            None => errors.push(format!("system '{system}' is not declared in [systems]")),
        }
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

    const VALID: &str = r#"
schema = 1

[distribution]
name = "SchneeForge Developer Environment"

[profiles]
default = "developer"
available = ["minimal", "developer"]

[systems]
aarch64-darwin = true
x86_64-linux = true
aarch64-linux = true
"#;

    #[test]
    fn parse_full_manifest() {
        let m = Manifest::parse(VALID).unwrap();
        assert_eq!(m.schema, 1);
        assert_eq!(
            m.distribution.name.as_deref(),
            Some("SchneeForge Developer Environment")
        );
        assert_eq!(m.profiles.default.as_deref(), Some("developer"));
        assert_eq!(m.profiles.available, vec!["minimal", "developer"]);
        assert!(m.systems.get("aarch64-darwin").copied().unwrap_or(false));
    }

    #[test]
    fn validate_accepts_supported_system() {
        let m = Manifest::parse(VALID).unwrap();
        assert!(m.validate("aarch64-darwin").valid);
        assert!(m.validate("x86_64-linux").valid);
    }

    #[test]
    fn validate_rejects_wrong_schema() {
        let m = Manifest::parse("schema = 2\n").unwrap();
        let v = m.validate("aarch64-darwin");
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("schema")));
    }

    #[test]
    fn validate_rejects_default_not_in_available() {
        let m = Manifest::parse(
            r#"
schema = 1
[profiles]
default = "developer"
available = ["minimal"]
"#,
        )
        .unwrap();
        let v = m.validate("aarch64-darwin");
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("default profile")));
    }

    #[test]
    fn validate_requires_default_profile() {
        let m = Manifest::parse("schema = 1\n").unwrap();
        let v = m.validate("aarch64-darwin");
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("profiles.default")));
    }

    #[test]
    fn validate_rejects_undeclared_system() {
        let m = Manifest::parse(VALID).unwrap();
        let v = m.validate("x86_64-darwin");
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("x86_64-darwin")));
    }

    #[test]
    fn validate_rejects_disabled_system() {
        let m = Manifest::parse(
            r#"
schema = 1
[profiles]
default = "minimal"
available = ["minimal"]
[systems]
aarch64-darwin = false
"#,
        )
        .unwrap();
        let v = m.validate("aarch64-darwin");
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("disabled")));
    }

    #[test]
    fn manifest_has_no_user_field() {
        // v1 config.toml の [user] は unknown field として error になる
        // (deny_unknown_fields ではなく default 動作: 余剰 field は無視)。
        // そのため「読み込まない」ことは load 先の file 名で保証する
        let m = Manifest::parse("schema = 1\n[user]\nusername = \"alice\"\n").unwrap();
        assert!(m.profiles.default.is_none());
    }
}
