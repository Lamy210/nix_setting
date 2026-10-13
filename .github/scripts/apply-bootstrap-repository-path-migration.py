#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/core/src/bootstrap.rs")
text = path.read_text()


def replace_once(old: str, new: str, label: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one occurrence, got {count}")
    text = text.replace(old, new, 1)


replace_once(
    "use std::path::{Path, PathBuf};",
    "use std::ffi::OsString;\nuse std::path::{Path, PathBuf};",
    "OsString import",
)

replace_once(
    '''pub fn setup(repo: &str, store: &StateStore, tc: &ToolInventory) -> Result<ApplyResult> {
    ensure_setup_nix_and_flakes_with(tc, preflight, enable_flakes)?;
    let target = detect_target();
    apply(&target, repo, store, tc, false)
}''',
    '''pub fn setup<P: AsRef<Path> + ?Sized>(
    repo: &P,
    store: &StateStore,
    tc: &ToolInventory,
) -> Result<ApplyResult> {
    ensure_setup_nix_and_flakes_with(tc, preflight, enable_flakes)?;
    let target = detect_target();
    apply(&target, repo, store, tc, false)
}''',
    "setup",
)

replace_once(
    '''pub fn clone_repo(url: &str, dest: &str, tc: &ToolInventory) -> Result<String> {
    let effective_url = if url.trim().is_empty() {
        crate::source::repo_url()
    } else {
        url.to_string()
    };
    if effective_url.trim().is_empty()
        || effective_url.starts_with('-')
        || effective_url.contains("::")
    {
        return Err(Error::Precondition("invalid repository URL".to_string()));
    }
    let git = tc.require_git()?;
    run_capture(
        &git.path,
        &[
            "clone".to_string(),
            effective_url.to_string(),
            dest.to_string(),
        ],
    )
}''',
    '''pub fn clone_repo<P: AsRef<Path> + ?Sized>(
    url: &str,
    dest: &P,
    tc: &ToolInventory,
) -> Result<String> {
    let effective_url = if url.trim().is_empty() {
        crate::source::repo_url()
    } else {
        url.to_string()
    };
    if effective_url.trim().is_empty()
        || effective_url.starts_with('-')
        || effective_url.contains("::")
    {
        return Err(Error::Precondition("invalid repository URL".to_string()));
    }
    let git = tc.require_git()?;
    let args = [
        OsString::from("clone"),
        OsString::from(effective_url),
        dest.as_ref().as_os_str().to_os_string(),
    ];
    run_capture(&git.path, &args)
}''',
    "clone_repo",
)

needle = '''    #[test]
    fn clone_repo_rejects_invalid_url() {
'''
if text.count(needle) != 1:
    raise SystemExit("clone_repo test insertion marker changed unexpectedly")
new_test = r'''    #[cfg(unix)]
    #[test]
    fn clone_repo_preserves_non_utf8_destination_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "schneeforge-clone-native-dest-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let capture = root.join("argv-capture");
        let git = root.join("fake-git");
        std::fs::write(
            &git,
            format!(
                "#!/bin/sh\nprintf '%s' \"$3\" > '{}'\nprintf 'ok\\n'\n",
                capture.display()
            ),
        )
        .unwrap();
        let mut perms = std::fs::metadata(&git).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&git, perms).unwrap();

        let tc = ToolInventory {
            nix: None,
            git: Some(ResolvedTool::new(git, ToolSource::Path)),
            homebrew: None,
            nh: None,
        };
        let dest = PathBuf::from(OsString::from_vec(b"/tmp/repository-\xff".to_vec()));

        let output = clone_repo("https://example.com/repo.git", &dest, &tc).unwrap();
        assert_eq!(output.trim(), "ok");
        assert_eq!(
            std::fs::read(&capture).unwrap(),
            dest.as_os_str().as_bytes()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

'''
text = text.replace(needle, new_test + needle, 1)

prod = text.split("\n#[cfg(test)]\nmod tests {", 1)[0]
if "clone_repo(url: &str, dest: &str" in prod:
    raise SystemExit("String clone destination remains")
if "dest.to_string()" in prod:
    raise SystemExit("clone destination is still stringified")

path.write_text(text)
