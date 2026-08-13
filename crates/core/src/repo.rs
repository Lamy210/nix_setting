/// リポジトリパスを解決する
/// 優先順: CLI 引数 > NIX_SETTING_DIR > ~/nix_setting > "."
pub fn resolve_repo(cli_repo: Option<&str>) -> String {
    resolve_repo_with(
        cli_repo,
        std::env::var("NIX_SETTING_DIR").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
}

/// テスト可能な純関数版
pub fn resolve_repo_with(
    cli_repo: Option<&str>,
    env_dir: Option<&str>,
    home: Option<&str>,
) -> String {
    if let Some(r) = cli_repo {
        return r.to_string();
    }
    if let Some(r) = env_dir {
        return r.to_string();
    }
    if let Some(h) = home {
        return format!("{h}/nix_setting");
    }
    ".".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_repo_wins() {
        assert_eq!(resolve_repo_with(Some("/custom"), None, None), "/custom");
    }

    #[test]
    fn env_dir_second() {
        assert_eq!(
            resolve_repo_with(None, Some("/from/env"), Some("/home/u")),
            "/from/env"
        );
    }

    #[test]
    fn home_fallback() {
        assert_eq!(
            resolve_repo_with(None, None, Some("/home/u")),
            "/home/u/nix_setting"
        );
    }

    #[test]
    fn default_dot() {
        assert_eq!(resolve_repo_with(None, None, None), ".");
    }

    #[test]
    fn cli_repo_overrides_all() {
        assert_eq!(
            resolve_repo_with(Some("/custom"), Some("/env"), Some("/home/u")),
            "/custom"
        );
    }
}
