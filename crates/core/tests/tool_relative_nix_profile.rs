use schneeforge_core::ToolResolver;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output};

const CHILD_CASE: &str = "SCHNEEFORGE_TOOL_RELATIVE_NIX_PROFILE_CHILD";
const CHILD_TOOL: &str = "SCHNEEFORGE_TOOL_RELATIVE_NIX_PROFILE_NAME";

fn make_executable(path: &Path) {
    fs::create_dir_all(path.parent().expect("tool parent")).unwrap();
    fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
}

fn fixture(case: &str) -> (PathBuf, PathBuf, String) {
    let root = std::env::temp_dir().join(format!(
        "schneeforge-tool-relative-nix-profile-{}-{case}",
        process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let work = root.join("work");
    let tool_name = format!(
        "schneeforge-relative-nix-profile-tool-{}-{case}",
        process::id()
    );
    let candidate = work.join("relative-profile").join("bin").join(&tool_name);
    make_executable(&candidate);
    (root, work, tool_name)
}

fn run_rust_isolated(case: &str) {
    let (root, work, tool_name) = fixture(case);

    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("isolated_relative_nix_profile_tool_resolution_child")
        .arg("--nocapture")
        .current_dir(&work)
        .env(CHILD_CASE, case)
        .env(CHILD_TOOL, &tool_name)
        .env("HOME", "")
        .env("XDG_STATE_HOME", "")
        .env("NIX_PROFILE", "relative-profile")
        .env("USER", "")
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("run isolated relative-NIX_PROFILE tool-resolution test");

    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "isolated relative-NIX_PROFILE tool-resolution check failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_shell_resolver(case: &str, inline_install: bool) -> Output {
    let (root, work, tool_name) = fixture(case);
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shared_resolver = repo_root.join("scripts/resolve-tools.sh");
    let install_script = repo_root.join("install.sh");

    let shell = if inline_install {
        r#"
eval "$(sed -n '1,/^# --- end inline resolver ---$/p' "$INSTALL_SCRIPT")"
if resolve_tool "$TOOL_NAME"; then
  printf 'unexpected tool resolved from relative NIX_PROFILE\n' >&2
  exit 1
fi
"#
    } else {
        r#"
. "$RESOLVER_SCRIPT"
if resolve_tool "$TOOL_NAME"; then
  printf 'unexpected tool resolved from relative NIX_PROFILE\n' >&2
  exit 1
fi
"#
    };

    let output = Command::new("bash")
        .arg("-c")
        .arg(shell)
        .current_dir(&work)
        .env("RESOLVER_SCRIPT", &shared_resolver)
        .env("INSTALL_SCRIPT", &install_script)
        .env("TOOL_NAME", &tool_name)
        .env("HOME", "")
        .env("XDG_STATE_HOME", "")
        .env("NIX_PROFILE", "relative-profile")
        .env("USER", "")
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("run shell relative-NIX_PROFILE tool-resolution test");

    let _ = fs::remove_dir_all(&root);
    output
}

fn assert_shell_ignores_relative_nix_profile(case: &str, inline_install: bool) {
    let output = run_shell_resolver(case, inline_install);
    assert!(
        output.status.success(),
        "shell resolver accepted a relative NIX_PROFILE:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn relative_nix_profile_is_ignored_by_rust_resolver() {
    run_rust_isolated("rust");
}

#[test]
fn shared_shell_resolver_ignores_relative_nix_profile() {
    assert_shell_ignores_relative_nix_profile("shared", false);
}

#[test]
fn install_inline_resolver_ignores_relative_nix_profile() {
    assert_shell_ignores_relative_nix_profile("inline", true);
}

#[test]
fn isolated_relative_nix_profile_tool_resolution_child() {
    if std::env::var(CHILD_CASE).is_err() {
        return;
    }
    let tool_name = std::env::var(CHILD_TOOL).expect("child tool name");
    let resolved = ToolResolver::new().resolve_tool(&tool_name);
    assert!(
        resolved.is_none(),
        "relative NIX_PROFILE must not contribute known tool paths; resolved {resolved:?}"
    );
}
