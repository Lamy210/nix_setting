use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// OS-native repository filesystem identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPath(PathBuf);

impl RepositoryPath {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }

    pub fn display_lossy(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl AsRef<Path> for RepositoryPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<PathBuf> for RepositoryPath {
    fn from(path: PathBuf) -> Self {
        Self::new(path)
    }
}

/// 解決済み repository (パス + 存在有無)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub path: RepositoryPath,
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
    pub fn resolve(&self, cli_repo: Option<&Path>) -> Repo {
        let path = resolve_repo_path(cli_repo);
        let exists = path.as_path().is_dir();
        Repo { path, exists }
    }
}

/// repository path から現在の git revision (HEAD) を取得する path-native API。
pub fn current_git_revision_path(repo: &Path, git_bin: &Path) -> Option<String> {
    let out = std::process::Command::new(git_bin)
        .current_dir(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

/// repository path から現在の git revision (HEAD) を取得する。
/// migration 中は `&str` caller も `AsRef<Path>` 経由で受けるが、内部では
/// OS-native path identity を保ったまま `current_dir` へ渡す。
pub fn current_git_revision<P: AsRef<Path> + ?Sized>(repo: &P, git_bin: &Path) -> Option<String> {
    current_git_revision_path(repo.as_ref(), git_bin)
}

/// リポジトリパスを OS-native path として解決する。
/// 優先順: CLI 引数 > NIX_SETTING_DIR > ~/nix_setting > "."
pub fn resolve_repo_path(cli_repo: Option<&Path>) -> RepositoryPath {
    resolve_repo_path_with(
        cli_repo,
        std::env::var_os("NIX_SETTING_DIR").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

/// path-native のテスト可能な純関数版。
pub fn resolve_repo_path_with(
    cli_repo: Option<&Path>,
    env_dir: Option<&OsStr>,
    home: Option<&OsStr>,
) -> RepositoryPath {
    if let Some(repo) = cli_repo.filter(|path| !path.as_os_str().is_empty()) {
        return RepositoryPath::new(repo.to_path_buf());
    }
    if let Some(repo) = env_dir.filter(|value| !value.is_empty()) {
        return RepositoryPath::new(PathBuf::from(repo));
    }
    if let Some(home) = home
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return RepositoryPath::new(home.join("nix_setting"));
    }
    RepositoryPath::new(PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolute_home() -> &'static str {
        #[cfg(windows)]
        {
            r"C:\Users\u"
        }
        #[cfg(not(windows))]
        {
            "/home/u"
        }
    }

    // UTF-8 examples exercise the same OS-native API used by the CLI/Desktop.
    // Non-UTF-8 inputs are covered below with OsString/OsStr byte checks.
    fn resolve_test_repo(cli: Option<&str>, env: Option<&str>, home: Option<&str>) -> PathBuf {
        resolve_repo_path_with(
            cli.map(Path::new),
            env.map(OsStr::new),
            home.map(OsStr::new),
        )
        .into_path_buf()
    }

    #[test]
    fn cli_repo_wins() {
        assert_eq!(
            resolve_test_repo(Some("/custom"), None, None),
            PathBuf::from("/custom")
        );
    }

    #[test]
    fn env_dir_second() {
        assert_eq!(
            resolve_test_repo(None, Some("/from/env"), Some("/home/u")),
            PathBuf::from("/from/env")
        );
    }

    #[test]
    fn home_fallback() {
        let home = absolute_home();
        assert_eq!(
            resolve_test_repo(None, None, Some(home)),
            Path::new(home).join("nix_setting")
        );
    }

    #[test]
    fn relative_home_falls_back_to_current_directory() {
        assert_eq!(
            resolve_test_repo(None, None, Some("relative-home")),
            PathBuf::from(".")
        );
    }

    #[test]
    fn default_dot() {
        assert_eq!(resolve_test_repo(None, None, None), PathBuf::from("."));
    }

    #[test]
    fn cli_repo_overrides_all() {
        assert_eq!(
            resolve_test_repo(Some("/custom"), Some("/env"), Some("/home/u")),
            PathBuf::from("/custom")
        );
    }

    #[test]
    fn empty_cli_repo_falls_back_to_env_dir() {
        assert_eq!(
            resolve_test_repo(Some(""), Some("/from/env"), Some("/home/u")),
            PathBuf::from("/from/env")
        );
    }

    #[test]
    fn empty_env_dir_falls_back_to_home() {
        let home = absolute_home();
        assert_eq!(
            resolve_test_repo(None, Some(""), Some(home)),
            Path::new(home).join("nix_setting")
        );
    }

    #[test]
    fn empty_home_falls_back_to_current_directory() {
        assert_eq!(resolve_test_repo(None, None, Some("")), PathBuf::from("."));
    }

    #[cfg(unix)]
    #[test]
    fn current_git_revision_rejects_blank_success_output() {
        use std::os::unix::fs::PermissionsExt;

        let dir =
            std::env::temp_dir().join(format!("sf-repo-empty-revision-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fake_git = dir.join("fake-git");
        std::fs::write(&fake_git, "#!/bin/sh\nexit 0\n").unwrap();
        let mut permissions = std::fs::metadata(&fake_git).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_git, permissions).unwrap();

        assert_eq!(
            current_git_revision_path(&dir, &fake_git),
            None,
            "successful git inspection without a usable revision must not produce Some(\"\")"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repo_resolver_reports_missing_repo() {
        let resolver = RepoResolver::new();
        let repo = resolver.resolve(Some(Path::new("/definitely/not/a/real/repo")));
        assert_eq!(
            repo.path.as_path(),
            Path::new("/definitely/not/a/real/repo")
        );
        assert!(!repo.exists);
    }

    #[test]
    fn repo_resolver_reports_existing_repo() {
        let dir = std::env::temp_dir().join("sf-repo-exists");
        std::fs::create_dir_all(&dir).unwrap();
        let resolver = RepoResolver::new();
        let repo = resolver.resolve(Some(&dir));
        assert!(repo.exists);
        assert_eq!(repo.path.as_path(), dir.as_path());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_repo_with_preserves_non_utf8_cli_path() {
        use std::os::unix::ffi::OsStringExt;
        let cli = PathBuf::from(std::ffi::OsString::from_vec(
            b"/tmp/schneeforge-cli-\xff".to_vec(),
        ));
        let resolved = resolve_repo_path_with(Some(cli.as_path()), None, None);
        assert_eq!(resolved.as_path().as_os_str(), cli.as_os_str());
    }

    #[cfg(unix)]
    #[test]
    fn resolve_repo_with_preserves_non_utf8_env_dir() {
        use std::os::unix::ffi::OsStringExt;
        let env_dir = std::ffi::OsString::from_vec(b"/tmp/schneeforge-env-\xff".to_vec());
        let resolved = resolve_repo_path_with(None, Some(env_dir.as_os_str()), None);
        assert_eq!(resolved.as_path().as_os_str(), env_dir.as_os_str());
    }

    #[cfg(unix)]
    #[test]
    fn resolve_repo_with_preserves_non_utf8_absolute_home() {
        use std::os::unix::ffi::OsStringExt;
        let home = std::ffi::OsString::from_vec(b"/tmp/schneeforge-home-\xff".to_vec());
        let resolved = resolve_repo_path_with(None, None, Some(home.as_os_str()));
        let expected = PathBuf::from(home).join("nix_setting");
        assert_eq!(resolved.as_path(), expected.as_path());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn current_git_revision_preserves_non_utf8_working_directory() {
        use std::os::unix::ffi::OsStringExt;
        use std::os::unix::fs::PermissionsExt;

        let root =
            std::env::temp_dir().join(format!("sf-repo-native-revision-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let repo = root.join(std::ffi::OsString::from_vec(b"repo-\xff".to_vec()));
        std::fs::create_dir_all(&repo).unwrap();
        let fake_git = root.join("fake-git");
        std::fs::write(
            &fake_git,
            "#!/bin/sh\nprintf '0123456789012345678901234567890123456789\\n'\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&fake_git).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_git, permissions).unwrap();

        let actual = current_git_revision_path(&repo, &fake_git);
        if actual.is_none() {
            // Record the OS-level execution result only when this regression fails.
            // Re-running the fake executable is diagnostic evidence, not a pass criterion.
            let direct = std::process::Command::new(&fake_git)
                .current_dir(&repo)
                .args(["rev-parse", "HEAD"])
                .output();
            panic!(
                "path-native git revision lookup returned None; repo={:?}, fake_git={:?}, direct_execution={:?}",
                repo, fake_git, direct
            );
        }
        assert_eq!(
            actual.as_deref(),
            Some("0123456789012345678901234567890123456789")
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
