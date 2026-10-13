#!/usr/bin/env python3
from pathlib import Path

OPS = Path("crates/core/src/operations.rs")
ACTIONS = Path("crates/core/src/actions.rs")
ops = OPS.read_text()
actions = ACTIONS.read_text()


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one occurrence, got {count}: {old[:100]!r}")
    return text.replace(old, new, 1)


ops = replace_once(
    ops,
    "pub fn verify(repo: &str, tc: &ToolInventory) -> VerifyReport {\n    verify_with_store(repo, tc, &StateStore::default())\n}",
    "pub fn verify<P: AsRef<Path> + ?Sized>(repo: &P, tc: &ToolInventory) -> VerifyReport {\n    verify_with_store(repo, tc, &StateStore::default())\n}",
    "verify",
)
ops = replace_once(
    ops,
    "fn verify_with_store(repo: &str, tc: &ToolInventory, state_store: &StateStore) -> VerifyReport {\n    let state_result = state_store.load();",
    "fn verify_with_store<P: AsRef<Path> + ?Sized>(\n    repo: &P,\n    tc: &ToolInventory,\n    state_store: &StateStore,\n) -> VerifyReport {\n    let repo = repo.as_ref();\n    let state_result = state_store.load();",
    "verify_with_store",
)
ops = replace_once(
    ops,
    "fn repository_check(repo: &str, managed: bool) -> VerifyCheck {",
    "fn repository_check<P: AsRef<Path> + ?Sized>(repo: &P, managed: bool) -> VerifyCheck {",
    "repository_check signature",
)
ops = replace_once(
    ops,
    "ok: std::path::Path::new(repo).is_dir(),",
    "ok: repo.as_ref().is_dir(),",
    "repository_check body",
)
ops = replace_once(
    ops,
    '''fn sync_args(repo: &str) -> Vec<String> {
    vec![
        "-C".to_string(),
        repo.to_string(),
        "pull".to_string(),
        "--ff-only".to_string(),
    ]
}''',
    '''fn sync_args_native(repo: &Path) -> Vec<OsString> {
    vec![
        OsString::from("-C"),
        repo.as_os_str().to_os_string(),
        OsString::from("pull"),
        OsString::from("--ff-only"),
    ]
}

#[cfg(test)]
fn sync_args(repo: &str) -> Vec<String> {
    vec![
        "-C".to_string(),
        repo.to_string(),
        "pull".to_string(),
        "--ff-only".to_string(),
    ]
}''',
    "sync args",
)
ops = replace_once(
    ops,
    "fn current_branch(repo: &str, git: &crate::tool::ResolvedTool) -> Result<Option<String>> {\n    crate::source::current_branch(repo, git)\n}",
    "fn current_branch<P: AsRef<Path> + ?Sized>(\n    repo: &P,\n    git: &crate::tool::ResolvedTool,\n) -> Result<Option<String>> {\n    crate::source::current_branch(repo, git)\n}",
    "current_branch",
)
ops = replace_once(
    ops,
    "pub fn sync(repo: &str, tc: &ToolInventory, capture: bool) -> Result<Option<String>> {",
    "pub fn sync<P: AsRef<Path> + ?Sized>(\n    repo: &P,\n    tc: &ToolInventory,\n    capture: bool,\n) -> Result<Option<String>> {",
    "sync signature",
)
ops = replace_once(
    ops,
    '''fn sync_with_lock(
    repo: &str,
    tc: &ToolInventory,
    capture: bool,
    lock: &OperationLock,
) -> Result<Option<String>> {
    let git = tc.require_git()?;''',
    '''fn sync_with_lock<P: AsRef<Path> + ?Sized>(
    repo: &P,
    tc: &ToolInventory,
    capture: bool,
    lock: &OperationLock,
) -> Result<Option<String>> {
    let repo = repo.as_ref();
    let git = tc.require_git()?;''',
    "sync_with_lock",
)
ops = replace_once(ops, "let args = sync_args(repo);", "let args = sync_args_native(repo);", "sync native args #1")
ops = replace_once(
    ops,
    '''fn git_dirty(repo: &str, git: &crate::tool::ResolvedTool) -> Result<bool> {
    let out = run_capture(
        &git.path,
        &[
            "-C".to_string(),
            repo.to_string(),
            "status".to_string(),
            "--porcelain".to_string(),
        ],
    )?;
    Ok(!out.trim().is_empty())
}''',
    '''fn git_dirty<P: AsRef<Path> + ?Sized>(repo: &P, git: &crate::tool::ResolvedTool) -> Result<bool> {
    let repo = repo.as_ref();
    let args = [
        OsString::from("-C"),
        repo.as_os_str().to_os_string(),
        OsString::from("status"),
        OsString::from("--porcelain"),
    ];
    let out = run_capture(&git.path, &args)?;
    Ok(!out.trim().is_empty())
}''',
    "git_dirty",
)
ops = replace_once(
    ops,
    '''pub fn update(
    repo: &str,
    store: &StateStore,
    tc: &ToolInventory,
    capture: bool,
) -> Result<UpdateResult> {
    let git = tc.require_git()?;''',
    '''pub fn update<P: AsRef<Path> + ?Sized>(
    repo: &P,
    store: &StateStore,
    tc: &ToolInventory,
    capture: bool,
) -> Result<UpdateResult> {
    let repo = repo.as_ref();
    let git = tc.require_git()?;''',
    "update",
)
# update() contains the second production sync argv construction.
ops = replace_once(ops, "let args = sync_args(repo);", "let args = sync_args_native(repo);", "sync native args #2")
ops = replace_once(
    ops,
    '''pub fn source_init(
    repo: &str,
    store: &StateStore,
    git: &crate::tool::ResolvedTool,
    channel: Option<String>,
    tag: Option<String>,
) -> Result<SourceInitResult> {''',
    '''pub fn source_init<P: AsRef<Path> + ?Sized>(
    repo: &P,
    store: &StateStore,
    git: &crate::tool::ResolvedTool,
    channel: Option<String>,
    tag: Option<String>,
) -> Result<SourceInitResult> {''',
    "source_init",
)
ops = replace_once(
    ops,
    '''fn source_init_with(
    repo: &str,
    store: &StateStore,
    git: &crate::tool::ResolvedTool,
    remote: &RemoteTags,''',
    '''fn source_init_with<P: AsRef<Path> + ?Sized>(
    repo: &P,
    store: &StateStore,
    git: &crate::tool::ResolvedTool,
    remote: &RemoteTags,''',
    "source_init_with",
)
ops = replace_once(
    ops,
    '''fn update_release(
    repo: &str,
    git: &crate::tool::ResolvedTool,
    channel: &str,
    capture: bool,
) -> Result<Option<String>> {
    validate_release_channel(channel)?;''',
    '''fn update_release<P: AsRef<Path> + ?Sized>(
    repo: &P,
    git: &crate::tool::ResolvedTool,
    channel: &str,
    capture: bool,
) -> Result<Option<String>> {
    let repo = repo.as_ref();
    validate_release_channel(channel)?;''',
    "update_release",
)
ops = replace_once(
    ops,
    '''    let fetch_args = vec![
        "-C".to_string(),
        repo.to_string(),
        "fetch".to_string(),
        "--tags".to_string(),
        "--quiet".to_string(),
    ];''',
    '''    let fetch_args = vec![
        OsString::from("-C"),
        repo.as_os_str().to_os_string(),
        OsString::from("fetch"),
        OsString::from("--tags"),
        OsString::from("--quiet"),
    ];''',
    "release fetch argv",
)
ops = replace_once(
    ops,
    '''    let checkout_args = vec![
        "-C".to_string(),
        repo.to_string(),
        "checkout".to_string(),
        "--quiet".to_string(),
        latest.clone(),
    ];''',
    '''    let checkout_args = vec![
        OsString::from("-C"),
        repo.as_os_str().to_os_string(),
        OsString::from("checkout"),
        OsString::from("--quiet"),
        OsString::from(latest),
    ];''',
    "release checkout argv",
)
ops = replace_once(
    ops,
    '''fn list_tags(repo: &str, git: &crate::tool::ResolvedTool) -> Result<Vec<String>> {
    let out = run_capture(
        &git.path,
        &[
            "-C".to_string(),
            repo.to_string(),
            "tag".to_string(),
            "--list".to_string(),
        ],
    )?;''',
    '''fn list_tags<P: AsRef<Path> + ?Sized>(
    repo: &P,
    git: &crate::tool::ResolvedTool,
) -> Result<Vec<String>> {
    let repo = repo.as_ref();
    let args = [
        OsString::from("-C"),
        repo.as_os_str().to_os_string(),
        OsString::from("tag"),
        OsString::from("--list"),
    ];
    let out = run_capture(&git.path, &args)?;''',
    "list_tags",
)
ops = replace_once(
    ops,
    "fn current_checkout_ref(repo: &str, git: &crate::tool::ResolvedTool) -> Result<Option<String>> {\n    crate::source::exact_tag(repo, git)\n}",
    "fn current_checkout_ref<P: AsRef<Path> + ?Sized>(\n    repo: &P,\n    git: &crate::tool::ResolvedTool,\n) -> Result<Option<String>> {\n    crate::source::exact_tag(repo, git)\n}",
    "current_checkout_ref",
)
ops = replace_once(
    ops,
    "pub fn source_sync(repo: &str, tc: &ToolInventory, capture: bool) -> Result<Option<String>> {",
    "pub fn source_sync<P: AsRef<Path> + ?Sized>(\n    repo: &P,\n    tc: &ToolInventory,\n    capture: bool,\n) -> Result<Option<String>> {",
    "source_sync",
)
ops = replace_once(
    ops,
    '''fn source_sync_with(
    repo: &str,
    tc: &ToolInventory,
    capture: bool,
    store: &StateStore,
) -> Result<Option<String>> {''',
    '''fn source_sync_with<P: AsRef<Path> + ?Sized>(
    repo: &P,
    tc: &ToolInventory,
    capture: bool,
    store: &StateStore,
) -> Result<Option<String>> {''',
    "source_sync_with",
)
ops = replace_once(
    ops,
    "pub fn deps_update(repo: &str, tc: &ToolInventory, capture: bool) -> Result<Option<String>> {",
    "pub fn deps_update<P: AsRef<Path> + ?Sized>(\n    repo: &P,\n    tc: &ToolInventory,\n    capture: bool,\n) -> Result<Option<String>> {",
    "deps_update",
)
ops = replace_once(
    ops,
    '''fn deps_update_with(
    repo: &str,
    tc: &ToolInventory,
    capture: bool,
    store: &StateStore,
) -> Result<Option<String>> {''',
    '''fn deps_update_with<P: AsRef<Path> + ?Sized>(
    repo: &P,
    tc: &ToolInventory,
    capture: bool,
    store: &StateStore,
) -> Result<Option<String>> {''',
    "deps_update_with",
)
ops = replace_once(
    ops,
    "fn release_lock_warning(repo: &str, tc: &ToolInventory) -> Result<Option<String>> {",
    "fn release_lock_warning<P: AsRef<Path> + ?Sized>(\n    repo: &P,\n    tc: &ToolInventory,\n) -> Result<Option<String>> {",
    "release_lock_warning",
)

prod = ops.split("\n#[cfg(test)]\nmod tests {", 1)[0]
if "repo: &str" in prod:
    raise SystemExit("production operations.rs still contains repo: &str")
if "repo.to_string()" in prod:
    raise SystemExit("production operations.rs still stringifies a repository path")
if "Path::new(repo)" in prod:
    raise SystemExit("production operations.rs reconstructs Path from repository text")

# The old String-based action layer is retained only for its existing unit tests while
# production operations use the native action entry points.
for marker in [
    "fn darwin_switch_args(",
    "fn linux_build_args(",
    "fn activation_link() -> String {",
    "struct ActivationLink {",
    "impl ActivationLink {",
    "impl Drop for ActivationLink {",
    "pub(crate) fn apply(target: &ConfigurationTarget, repo: &str, tc: &ToolInventory) -> Result<()> {",
    "pub(crate) fn apply_captured(",
    "fn apply_linux(target: &ConfigurationTarget, repo: &str, nix: &ResolvedTool) -> Result<()> {",
    "fn apply_linux_captured(",
    "fn darwin_rollback_args(repo: &str, overrides: &[String]) -> Vec<String> {",
    "pub(crate) fn rollback(target: &ConfigurationTarget, repo: &str, tc: &ToolInventory) -> Result<()> {",
    "pub(crate) fn rollback_captured(",
    "fn upgrade_args(repo: &str) -> Vec<String> {",
    "pub(crate) fn upgrade(repo: &str, tc: &ToolInventory) -> Result<()> {",
    "pub(crate) fn upgrade_captured(repo: &str, tc: &ToolInventory) -> Result<String> {",
]:
    actions = replace_once(actions, marker, "#[cfg(test)]\n" + marker, f"test-only action {marker}")

OPS.write_text(ops)
ACTIONS.write_text(actions)
