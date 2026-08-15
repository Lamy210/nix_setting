use serde::Serialize;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

/// ツールを発見した場所の分類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ToolSource {
    /// `SCHNEEFORGE_<NAME>_BIN` 等の明示的 env override
    EnvOverride,
    /// `PATH` 環境変数
    Path,
    /// `$XDG_STATE_HOME/nix/profile/bin` or `~/.local/state/nix/profile/bin`
    XdgStateProfile,
    /// `$NIX_PROFILE/bin`
    NixProfileEnv,
    /// `~/.nix-profile/bin`
    NixProfileHome,
    /// `/etc/profiles/per-user/$USER/bin`
    PerUserProfile,
    /// `/nix/var/nix/profiles/default/bin`
    SystemProfile,
    /// `/opt/homebrew/bin` or `/usr/local/bin`
    Homebrew,
}

impl fmt::Display for ToolSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ToolSource::EnvOverride => "env-override",
            ToolSource::Path => "PATH",
            ToolSource::XdgStateProfile => "xdg-state-profile",
            ToolSource::NixProfileEnv => "nix-profile-env",
            ToolSource::NixProfileHome => "nix-profile-home",
            ToolSource::PerUserProfile => "per-user-profile",
            ToolSource::SystemProfile => "system-profile",
            ToolSource::Homebrew => "homebrew",
        };
        f.write_str(s)
    }
}

/// 解決済みツール。絶対パス・発見場所・version を保持する
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedTool {
    pub path: PathBuf,
    pub source: ToolSource,
    pub version: Option<String>,
}

impl ResolvedTool {
    pub fn new(path: PathBuf, source: ToolSource) -> Self {
        Self {
            path,
            source,
            version: None,
        }
    }

    /// `<path> --version` の先頭行で version を埋める (subprocess spawn を伴う)
    pub fn with_version(mut self) -> Self {
        self.version = version_of(&self.path);
        self
    }
}

/// 現在の PC で発見されたツールのスナップショット。1回 discover したら以降全操作でこれを使う。
///
/// 全フィールドが `Option` であり、**Nix / Git が未検出でも構築できる**
/// (Fresh install 環境での診断を可能にするため)。実行時に必須の操作は
/// [`ToolInventory::require_nix`] / [`ToolInventory::require_git`] で明示的に昇格する。
#[derive(Debug, Clone, Default)]
pub struct ToolInventory {
    pub nix: Option<ResolvedTool>,
    pub git: Option<ResolvedTool>,
    pub homebrew: Option<ResolvedTool>,
    pub nh: Option<ResolvedTool>,
}

impl ToolInventory {
    /// 現在の環境からツールを discover する。各ツールは見つからなければ `None`。
    /// version も1回だけ取得する (subprocess 4 回: nix/git/brew/nh)。
    pub fn discover() -> Self {
        let resolver = ToolResolver::new();
        Self {
            nix: resolver.resolve_tool_with_version("nix"),
            git: resolver.resolve_tool_with_version("git"),
            homebrew: resolver.resolve_tool_with_version("brew"),
            nh: resolver.resolve_tool_with_version("nh"),
        }
    }

    /// Nix を要求する。未発見の場合は `Err(NixNotFound)`。
    pub fn require_nix(&self) -> Result<&ResolvedTool, ToolRequirementError> {
        self.nix.as_ref().ok_or(ToolRequirementError::NixNotFound)
    }

    /// Git を要求する。未発見の場合は `Err(GitNotFound)`。
    pub fn require_git(&self) -> Result<&ResolvedTool, ToolRequirementError> {
        self.git.as_ref().ok_or(ToolRequirementError::GitNotFound)
    }
}

/// 必須ツールの欠落。operations 側で `require_*` を呼んだ際に返される。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolRequirementError {
    NixNotFound,
    GitNotFound,
}

impl fmt::Display for ToolRequirementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolRequirementError::NixNotFound => f.write_str(
                "nix not found in PATH or known locations (install: curl -L https://nixos.org/nix/install | sh)",
            ),
            ToolRequirementError::GitNotFound => f.write_str(
                "git not found in PATH or known locations (install via your OS package manager)",
            ),
        }
    }
}

impl std::error::Error for ToolRequirementError {}

// ============================================================================
// 後方互換のための ToolStatus（GUI serialize 維持）
// ============================================================================

/// ツール解決結果（後方互換・GUI serialize 用）
///
/// 新規コードでは [`ResolvedTool`] / [`ToolInventory`] を使うこと。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

impl ToolStatus {
    fn not_found() -> Self {
        Self {
            available: false,
            path: None,
            version: None,
        }
    }

    /// 解決済みパスから version を取得して埋める (subprocess spawn を伴う)
    pub fn with_version(mut self) -> Self {
        if let Some(path) = &self.path {
            self.version = version_of(Path::new(path));
        }
        self
    }
}

impl From<&ResolvedTool> for ToolStatus {
    fn from(tool: &ResolvedTool) -> Self {
        Self {
            available: true,
            path: Some(tool.path.to_string_lossy().to_string()),
            version: tool.version.clone(),
        }
    }
}

impl From<Option<&ResolvedTool>> for ToolStatus {
    fn from(tool: Option<&ResolvedTool>) -> Self {
        match tool {
            Some(t) => Self::from(t),
            None => Self::not_found(),
        }
    }
}

// ============================================================================
// ToolResolver
// ============================================================================

/// PATH と既知パスの両方からツールを解決する
///
/// macOS GUI (.app) は Terminal と PATH が異なるため、既知パスを併用する。
/// 加えて Tauri 公式の `fix-path-env-rs` で PATH 補正を併用することで
/// `/etc/paths.d/*` 等の環境固有設定も取り込む二段構え。
///
/// 解決順序（詳細は [`ToolResolver::resolve_tool_with_source`]）:
/// `SCHNEEFORGE_<NAME>_BIN` → PATH → XDG state → NIX_PROFILE → ~/.nix-profile
/// → /etc/profiles/per-user/$USER → /nix/var/nix/profiles/default → /opt/homebrew/bin → /usr/local/bin
#[derive(Debug, Clone)]
pub struct ToolResolver {
    /// (dir, source) のペア。探索順序は呼び出し側で制御しない（resolve_tool 内で整序）
    known_paths: Vec<(PathBuf, ToolSource)>,
}

impl Default for ToolResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolResolver {
    pub fn new() -> Self {
        Self::with_known_paths(default_known_paths())
    }

    pub fn with_known_paths(known_paths: Vec<(PathBuf, ToolSource)>) -> Self {
        Self { known_paths }
    }

    /// ツール名を解決して ToolStatus を返す（後方互換・subprocess なし）
    pub fn resolve(&self, tool: &str) -> ToolStatus {
        match self.resolve_tool(tool) {
            Some(rt) => ToolStatus::from(&rt),
            None => ToolStatus::not_found(),
        }
    }

    /// ツール名を解決し、version も取得して返す（`--version` subprocess を伴う）
    pub fn resolve_with_version(&self, tool: &str) -> ToolStatus {
        self.resolve(tool).with_version()
    }

    /// ツール名を `ResolvedTool` として解決する（subprocess なし）
    pub fn resolve_tool(&self, tool: &str) -> Option<ResolvedTool> {
        self.resolve_tool_with_source(tool)
            .map(|(path, source)| ResolvedTool::new(path, source))
    }

    /// ツール名を (canonical path, source) として解決する純関数（subprocess なし）
    ///
    /// 探索順:
    /// 1. `SCHNEEFORGE_<NAME>_BIN` env（大文字の tool 名）
    /// 2. PATH
    /// 3..N. known_paths（整序済み）
    #[allow(clippy::doc_lazy_continuation)]
    pub fn resolve_tool_with_source(&self, tool: &str) -> Option<(PathBuf, ToolSource)> {
        let upper = tool.to_ascii_uppercase().replace('-', "_");
        let env_key = format!("SCHNEEFORGE_{upper}_BIN");
        if let Ok(p) = env::var(&env_key) {
            let path = PathBuf::from(p);
            if is_executable(&path) {
                return Some((canonicalize(path), ToolSource::EnvOverride));
            }
        }

        let path_dirs: Vec<String> = env::var("PATH")
            .map(|p| p.split(':').map(String::from).collect())
            .unwrap_or_default();
        if let Some(dir) = find_in_dirs(tool, &path_dirs) {
            return Some((canonicalize(dir), ToolSource::Path));
        }

        for (dir, source) in &self.known_paths {
            let candidate = dir.join(tool);
            if is_executable(&candidate) {
                return Some((canonicalize(candidate), *source));
            }
        }
        None
    }

    /// ツール名を解決し、version も取得した `ResolvedTool` を返す（subprocess を伴う）
    pub fn resolve_tool_with_version(&self, tool: &str) -> Option<ResolvedTool> {
        self.resolve_tool(tool).map(|t| t.with_version())
    }
}

/// `find_executable` は `discovery::which` から使われる後方互換 API。
///
/// 新規コードは [`ToolResolver::resolve_tool`] を使うこと。
pub fn find_executable(tool: &str, path_dirs: &[String], known_dirs: &[PathBuf]) -> Option<String> {
    if let Some(p) = find_in_dirs(tool, path_dirs) {
        return Some(p.to_string_lossy().to_string());
    }
    for dir in known_dirs {
        let candidate = dir.join(tool);
        if is_executable(&candidate) {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// PATH の各ディレクトリを実行可能ファイル探索に使う純関数
fn find_in_dirs(tool: &str, dirs: &[String]) -> Option<PathBuf> {
    for dir in dirs {
        let candidate = format!("{dir}/{tool}");
        if is_executable(Path::new(&candidate)) {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

/// 既知パスの既定セット（Nix 2.x の XDG 遷移を反映）
///
/// 参考: https://github.com/nix-community/home-manager/issues/4403
/// - root 以外のユーザープロファイルは `$XDG_STATE_HOME/nix/profiles` へ
/// - `/nix/var/nix/profiles/per-user/<user>` は root 専用になった
fn default_known_paths() -> Vec<(PathBuf, ToolSource)> {
    let mut paths: Vec<(PathBuf, ToolSource)> = Vec::new();

    // XDG_STATE_HOME（設定時）
    if let Ok(p) = env::var("XDG_STATE_HOME") {
        if !p.is_empty() {
            paths.push((
                PathBuf::from(&p).join("nix/profile/bin"),
                ToolSource::XdgStateProfile,
            ));
        }
    }

    let home = env::var("HOME").ok();

    // XDG デフォルト (~/.local/state/nix/profile/bin)
    if let Some(h) = &home {
        paths.push((
            PathBuf::from(h).join(".local/state/nix/profile/bin"),
            ToolSource::XdgStateProfile,
        ));
    }

    // NIX_PROFILE env
    if let Ok(p) = env::var("NIX_PROFILE") {
        if !p.is_empty() {
            paths.push((PathBuf::from(p).join("bin"), ToolSource::NixProfileEnv));
        }
    }

    // ~/.nix-profile/bin
    if let Some(h) = &home {
        paths.push((
            PathBuf::from(h).join(".nix-profile/bin"),
            ToolSource::NixProfileHome,
        ));
    }

    // /etc/profiles/per-user/$USER/bin
    if let Ok(user) = env::var("USER") {
        if !user.is_empty() {
            paths.push((
                PathBuf::from("/etc/profiles/per-user")
                    .join(user)
                    .join("bin"),
                ToolSource::PerUserProfile,
            ));
        }
    }

    // /nix/var/nix/profiles/default/bin
    paths.push((
        PathBuf::from("/nix/var/nix/profiles/default/bin"),
        ToolSource::SystemProfile,
    ));

    // Homebrew
    paths.push((PathBuf::from("/opt/homebrew/bin"), ToolSource::Homebrew));
    paths.push((PathBuf::from("/usr/local/bin"), ToolSource::Homebrew));

    paths
}

/// symlink を解決して realpath を返す。解決失敗時はそのまま返す
fn canonicalize(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path).unwrap_or(path)
}

/// 実行可能ファイルか (存在 + 実行ビット)
fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// `<path> --version` の先頭行を version として返す (失敗時 None)
pub fn version_of(path: &Path) -> Option<String> {
    let out = std::process::Command::new(path)
        .arg("--version")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .lines()
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_executable(dir: &Path, name: &str) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).unwrap();
        }
        path
    }

    #[test]
    fn find_in_path_dirs() {
        let dir = std::env::temp_dir().join("sf-tool-path");
        let _ = tmp_executable(&dir, "mytool");
        let path_dirs = vec![dir.to_string_lossy().to_string()];
        let found = find_executable("mytool", &path_dirs, &[]).unwrap();
        assert_eq!(found, dir.join("mytool").to_string_lossy().to_string());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_in_known_dirs_when_missing_from_path() {
        let dir = std::env::temp_dir().join("sf-tool-known");
        let _ = tmp_executable(&dir, "mytool");
        let known = vec![dir.clone()];
        let found = find_executable("mytool", &[], &known).unwrap();
        assert_eq!(found, dir.join("mytool").to_string_lossy().to_string());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_wins_over_known_paths() {
        let path_dir = std::env::temp_dir().join("sf-tool-path2");
        let known_dir = std::env::temp_dir().join("sf-tool-known2");
        let _ = tmp_executable(&path_dir, "mytool");
        let _ = tmp_executable(&known_dir, "mytool");
        let path_dirs = vec![path_dir.to_string_lossy().to_string()];
        let found =
            find_executable("mytool", &path_dirs, std::slice::from_ref(&known_dir)).unwrap();
        assert_eq!(found, path_dir.join("mytool").to_string_lossy().to_string());
        let _ = fs::remove_dir_all(&path_dir);
        let _ = fs::remove_dir_all(&known_dir);
    }

    #[test]
    fn not_found_returns_none() {
        assert!(find_executable("__no_such_tool__", &[], &[]).is_none());
    }

    #[test]
    fn non_executable_file_is_not_found() {
        let dir = std::env::temp_dir().join("sf-tool-noexec");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("plainfile");
        fs::write(&path, b"not executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&path, perms).unwrap();
        }
        let known = vec![dir.clone()];
        assert!(find_executable("plainfile", &[], &known).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolver_returns_not_found_for_missing_tool() {
        let resolver = ToolResolver::with_known_paths(vec![]);
        let status = resolver.resolve("__no_such_tool__");
        assert!(!status.available);
        assert!(status.path.is_none());
    }

    // --- ResolvedTool / ToolInventory のテスト ---

    #[test]
    fn resolve_tool_via_env_override() {
        let dir = std::env::temp_dir().join("sf-tool-env");
        let path = tmp_executable(&dir, "mytool");
        let key = "SCHNEEFORGE_MYTOOL_BIN";
        env::set_var(key, &path);
        let resolver = ToolResolver::with_known_paths(vec![]);
        let resolved = resolver.resolve_tool("mytool").expect("should resolve");
        assert_eq!(resolved.source, ToolSource::EnvOverride);
        env::remove_var(key);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_tool_via_path() {
        let dir = std::env::temp_dir().join("sf-tool-via-path");
        let _ = tmp_executable(&dir, "mytool-via-path");
        let resolver = ToolResolver::with_known_paths(vec![]);
        env::set_var(
            "PATH",
            format!("{}:{}", dir.display(), env::var("PATH").unwrap_or_default()),
        );
        let resolved = resolver
            .resolve_tool("mytool-via-path")
            .expect("should resolve via PATH");
        assert_eq!(resolved.source, ToolSource::Path);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_tool_via_known_path_xdg_state() {
        let dir = std::env::temp_dir().join("sf-tool-xdg");
        let _ = tmp_executable(&dir, "mytool-xdg");
        let resolver =
            ToolResolver::with_known_paths(vec![(dir.clone(), ToolSource::XdgStateProfile)]);
        let resolved = resolver
            .resolve_tool("mytool-xdg")
            .expect("should resolve via known");
        assert_eq!(resolved.source, ToolSource::XdgStateProfile);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn env_override_beats_path() {
        let path_dir = std::env::temp_dir().join("sf-tool-env-path");
        let env_dir = std::env::temp_dir().join("sf-tool-env-env");
        let _ = tmp_executable(&path_dir, "mytool-prio");
        let env_path = tmp_executable(&env_dir, "mytool-prio");
        env::set_var(
            "PATH",
            format!(
                "{}:{}",
                path_dir.display(),
                env::var("PATH").unwrap_or_default()
            ),
        );
        env::set_var("SCHNEEFORGE_MYTOOL_PRIO_BIN", &env_path);
        let resolver = ToolResolver::with_known_paths(vec![]);
        let resolved = resolver.resolve_tool("mytool-prio").unwrap();
        assert_eq!(resolved.source, ToolSource::EnvOverride);
        assert_eq!(resolved.path, env_path);
        env::remove_var("SCHNEEFORGE_MYTOOL_PRIO_BIN");
        let _ = fs::remove_dir_all(&path_dir);
        let _ = fs::remove_dir_all(&env_dir);
    }

    #[test]
    fn tool_status_from_resolved_tool() {
        let resolved = ResolvedTool::new(PathBuf::from("/usr/bin/nix"), ToolSource::SystemProfile);
        let status = ToolStatus::from(&resolved);
        assert!(status.available);
        assert_eq!(status.path.as_deref(), Some("/usr/bin/nix"));
        assert_eq!(status.version, None);
    }

    #[test]
    fn tool_status_from_none_resolved() {
        let status: ToolStatus = None.into();
        assert!(!status.available);
        assert!(status.path.is_none());
    }

    #[test]
    fn default_known_paths_includes_modern_nix_locations() {
        let paths = default_known_paths();
        let dir_strs: Vec<String> = paths
            .iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect();

        // Nix 2.x のモダンな配置を含むこと
        assert!(
            dir_strs
                .iter()
                .any(|s| s.contains(".local/state/nix/profile/bin")),
            "XDG state default should be present: {dir_strs:?}"
        );
        assert!(
            dir_strs.iter().any(|s| s.ends_with(".nix-profile/bin")),
            "~/.nix-profile/bin should be present: {dir_strs:?}"
        );
        assert!(
            dir_strs
                .iter()
                .any(|s| s.contains("/nix/var/nix/profiles/default/bin")),
            "system profile should be present: {dir_strs:?}"
        );
        assert!(
            dir_strs.iter().any(|s| s == "/opt/homebrew/bin"),
            "Apple Silicon Homebrew path should be present: {dir_strs:?}"
        );
        assert!(
            dir_strs.iter().any(|s| s == "/usr/local/bin"),
            "Intel Homebrew path should be present: {dir_strs:?}"
        );
    }

    #[test]
    fn default_known_paths_uses_xdg_state_home_when_set() {
        env::set_var("XDG_STATE_HOME", "/tmp/custom-xdg");
        let paths = default_known_paths();
        env::remove_var("XDG_STATE_HOME");
        assert!(paths
            .iter()
            .any(|(p, s)| p.ends_with("nix/profile/bin") && *s == ToolSource::XdgStateProfile));
    }

    #[test]
    fn default_known_paths_uses_per_user_with_real_user() {
        env::set_var("USER", "testuser");
        let paths = default_known_paths();
        let per_user = paths
            .iter()
            .find(|(_, s)| *s == ToolSource::PerUserProfile)
            .expect("per-user profile should exist");
        assert!(per_user.0.to_string_lossy().ends_with("testuser/bin"));
    }

    #[test]
    fn resolve_tool_canonicalizes_symlink() {
        // realpath は tempfile の挙動上 /tmp 内でないと失敗する可能性があるため、
        // 簡易的に同じディレクトリで symlink を作る
        let dir = std::env::temp_dir().join("sf-tool-canonical");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = tmp_executable(&dir, "mytool-real");
        let link = dir.join("mytool-link");
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let _ = symlink(&target, &link);
        }
        let resolver = ToolResolver::with_known_paths(vec![]);
        env::set_var("SCHNEEFORGE_MYTOOL_LINK_BIN", &link);
        let resolved = resolver.resolve_tool("mytool-link").unwrap();
        assert_eq!(resolved.path, std::fs::canonicalize(&target).unwrap());
        env::remove_var("SCHNEEFORGE_MYTOOL_LINK_BIN");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tool_source_display() {
        assert_eq!(ToolSource::EnvOverride.to_string(), "env-override");
        assert_eq!(ToolSource::Path.to_string(), "PATH");
        assert_eq!(ToolSource::XdgStateProfile.to_string(), "xdg-state-profile");
        assert_eq!(ToolSource::SystemProfile.to_string(), "system-profile");
        assert_eq!(ToolSource::Homebrew.to_string(), "homebrew");
    }

    #[test]
    fn tool_requirement_error_messages_are_user_friendly() {
        assert!(ToolRequirementError::NixNotFound
            .to_string()
            .contains("nix"));
        assert!(ToolRequirementError::NixNotFound
            .to_string()
            .contains("install"));
        assert!(ToolRequirementError::GitNotFound
            .to_string()
            .contains("git"));
    }
}
