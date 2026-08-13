/// 解決済み repository (パス + 存在有無)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub path: String,
    pub exists: bool,
}

/// repository を解決する (ツール解決 ToolResolver とは別責務)
#[derive(Debug, Clone, Default)]
pub struct RepoResolver;

impl RepoResolver {
    pub fn new() -> Self {
        Self
    }

    /// repository path を解決し、存在有無も返す
    pub fn resolve(&self, cli_repo: Option<&str>) -> Repo {
        let path = resolve_repo(cli_repo);
        let exists = std::path::Path::new(&path).is_dir();
        Repo { path, exists }
    }
}

/// repository path から現在の git revision (HEAD) を取得する。
///
/// `git` は解決済みの Toolchain パスを `git_bin` で受け取る（文字列リテラル
/// `Command::new("git")` は CI の forbid-raw-spawn lint で禁止）。
pub fn current_git_revision(repo: &str, git_bin: &std::path::Path) -> Option<String> {
    let out = std::process::Command::new(git_bin)
        .current_dir(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

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

    #[test]
    fn repo_resolver_reports_missing_repo() {
        let resolver = RepoResolver::new();
        let repo = resolver.resolve(Some("/definitely/not/a/real/repo"));
        assert_eq!(repo.path, "/definitely/not/a/real/repo");
        assert!(!repo.exists);
    }

    #[test]
    fn repo_resolver_reports_existing_repo() {
        let dir = std::env::temp_dir().join("sf-repo-exists");
        std::fs::create_dir_all(&dir).unwrap();
        let resolver = RepoResolver::new();
        let repo = resolver.resolve(Some(dir.to_str().unwrap()));
        assert!(repo.exists);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
