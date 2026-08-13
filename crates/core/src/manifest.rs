use serde::Deserialize;

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

impl Manifest {
    pub fn parse(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
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
    fn parse_rejects_empty_username() {
        let r = Manifest::parse("schema = 1\n[user]\nusername = \"\"\n");
        assert!(r.is_ok());
    }
}
