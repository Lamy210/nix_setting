#!/usr/bin/env python3
from pathlib import Path

LEGACY = Path("crates/cli/src/legacy_main.rs")
MAIN = Path("crates/cli/src/main.rs")
NIX = Path("crates/cli/src/nix_cmd.rs")
DESKTOP = Path("apps/desktop/src-tauri/src/lib.rs")

legacy = LEGACY.read_text()
main = MAIN.read_text()
nix = NIX.read_text()
desktop = DESKTOP.read_text()


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one occurrence, got {count}")
    return text.replace(old, new, 1)


# Native CLI: clap and repository resolution preserve OS-native filesystem identity.
legacy = replace_once(
    legacy,
    "mod nix_cmd;\n\nuse clap::{Parser, Subcommand};",
    "mod nix_cmd;\n\nuse std::path::{Path, PathBuf};\n\nuse clap::{Parser, Subcommand};",
    "legacy path imports",
)
legacy = replace_once(
    legacy,
    "    repo: Option<String>,",
    "    repo: Option<PathBuf>,",
    "CLI repo PathBuf",
)
legacy = replace_once(
    legacy,
    "    let repo = schneeforge_core::resolve_repo(cli.repo.as_deref());",
    "    let repo = schneeforge_core::resolve_repo_path(cli.repo.as_deref()).into_path_buf();",
    "legacy path resolver",
)
repo_sig_count = legacy.count("repo: &str")
if repo_sig_count < 10:
    raise SystemExit(f"unexpected legacy repo signature count: {repo_sig_count}")
legacy = legacy.replace("repo: &str", "repo: &Path")
legacy = replace_once(
    legacy,
    '        Err(_) if !std::path::Path::new(&format!("{repo}/schneeforge.toml")).exists() => {',
    '        Err(_) if !repo.join("schneeforge.toml").exists() => {',
    "status manifest path",
)
legacy = replace_once(
    legacy,
    '    let manifest_path = std::path::Path::new(repo).join("schneeforge.toml");',
    '    let manifest_path = repo.join("schneeforge.toml");',
    "load_manifest path",
)
if "repo: &str" in legacy or "resolve_repo(" in legacy:
    raise SystemExit("legacy native CLI still has String repository boundary")

# Main adapter uses the same path-native resolver. Windows pre-delegation parsing stays untouched.
main = replace_once(
    main,
    "        let repo = schneeforge_core::resolve_repo(cli.repo.as_deref());",
    "        let repo = schneeforge_core::resolve_repo_path(cli.repo.as_deref()).into_path_buf();",
    "main native resolver",
)
if "windows_launcher_args(args: &[OsString])" not in main:
    raise SystemExit("Windows Unicode launcher boundary changed unexpectedly")

# Managed Nix install receives the already-resolved native repository path.
nix = replace_once(
    nix,
    "pub fn run_install(repo_root: &str, args: InstallArgs) -> Result {",
    "pub fn run_install(repo_root: &Path, args: InstallArgs) -> Result {",
    "nix install path signature",
)
nix = replace_once(
    nix,
    "    let mn = ManagedNix::load_prefer_repo(Some(Path::new(repo_root)))",
    "    let mn = ManagedNix::load_prefer_repo(Some(repo_root))",
    "nix install path use",
)

# Desktop: keep native paths inside the adapter and preserve textual protocol boundaries.
desktop = replace_once(
    desktop,
    "    detect_target, release_page_url, repo_url, resolve_repo, scan, Diagnostics, Error, Manifest,",
    "    detect_target, release_page_url, repo_url, resolve_repo_path, scan, Diagnostics, Error, Manifest,",
    "desktop resolver import",
)
resolve_count = desktop.count("resolve_repo(None)")
if resolve_count < 5:
    raise SystemExit(f"unexpected desktop resolve_repo count: {resolve_count}")
desktop = desktop.replace(
    "resolve_repo(None)",
    "resolve_repo_path(None).into_path_buf()",
)
desktop = replace_once(
    desktop,
    "fn load_dashboard_manifest(repo: &str, store: &StateStore) -> Result<Option<Manifest>, String> {",
    "fn load_dashboard_manifest(repo: &std::path::Path, store: &StateStore) -> Result<Option<Manifest>, String> {",
    "desktop dashboard path",
)
desktop = replace_once(
    desktop,
    "    let repo_dir = resolve_repo_path(None).into_path_buf();\n    let repo_path = std::path::PathBuf::from(&repo_dir);",
    "    let repo_dir = resolve_repo_path(None).into_path_buf();",
    "desktop escalation round trip",
)
desktop = replace_once(
    desktop,
    "        match escalate_command(&cli_bin, op, &repo_path) {",
    "        match escalate_command(&cli_bin, op, &repo_dir) {",
    "desktop escalation path",
)
desktop = replace_once(
    desktop,
    '        cmd.env("NIX_SETTING_DIR", &repo_dir);',
    '        cmd.env("NIX_SETTING_DIR", repo_dir.as_os_str());',
    "desktop native repo env",
)
if "resolve_repo(None)" in desktop or "PathBuf::from(&repo_dir)" in desktop:
    raise SystemExit("desktop repository String/path round trip remains")

LEGACY.write_text(legacy)
MAIN.write_text(main)
NIX.write_text(nix)
DESKTOP.write_text(desktop)
