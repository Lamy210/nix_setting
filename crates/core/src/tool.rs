use serde::Serialize;
use std::path::{Path, PathBuf};

/// ツール解決結果
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

    fn found(path: String) -> Self {
        Self {
            available: true,
            path: Some(path),
            version: None,
        }
    }

    /// 解決済みパスから version を取得して埋める (subprocess spawn を伴う)
    pub fn with_version(mut self) -> Self {
        if let Some(path) = &self.path {
            self.version = version_of(path);
        }
        self
    }
}

/// PATH と既知パスの両方からツールを解決する
///
/// macOS GUI (.app) は Terminal と PATH が異なるため、既知パスを併用する。
/// 解決順: PATH → /nix/var/nix/profiles/default/bin → ~/.nix-profile/bin
///         → /opt/homebrew/bin → /usr/local/bin
#[derive(Debug, Clone)]
pub struct ToolResolver {
    known_paths: Vec<PathBuf>,
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

    pub fn with_known_paths(known_paths: Vec<PathBuf>) -> Self {
        Self { known_paths }
    }

    /// ツール名を解決して ToolStatus を返す (可用性 + パス。subprocess なし)
    pub fn resolve(&self, tool: &str) -> ToolStatus {
        let path_dirs: Vec<String> = std::env::var("PATH")
            .map(|p| p.split(':').map(String::from).collect())
            .unwrap_or_default();
        match find_executable(tool, &path_dirs, &self.known_paths) {
            Some(path) => ToolStatus::found(path),
            None => ToolStatus::not_found(),
        }
    }

    /// ツール名を解決し、version も取得して返す (`--version` の subprocess spawn を伴う)
    pub fn resolve_with_version(&self, tool: &str) -> ToolStatus {
        self.resolve(tool).with_version()
    }
}

/// PATH の各ディレクトリ → 既知パスの順で実行可能ファイルを探索する純関数
pub fn find_executable(tool: &str, path_dirs: &[String], known_dirs: &[PathBuf]) -> Option<String> {
    for dir in path_dirs {
        let candidate = format!("{dir}/{tool}");
        if is_executable(Path::new(&candidate)) {
            return Some(candidate);
        }
    }
    for dir in known_dirs {
        let candidate = dir.join(tool);
        if is_executable(&candidate) {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// 既知パスの既定セット
fn default_known_paths() -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from("/nix/var/nix/profiles/default/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        paths.insert(1, PathBuf::from(home).join(".nix-profile/bin"));
    }
    paths
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
pub fn version_of(path: &str) -> Option<String> {
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
        let found = find_executable("__no_such_tool__", &[], &[]);
        assert!(found.is_none());
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
}
