#!/usr/bin/env python3
from pathlib import Path

OPS = Path("crates/core/src/operations.rs")
text = OPS.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected exactly one occurrence, got {count}: {old[:80]!r}")
    text = text.replace(old, new, 1)


def replace_section(start_marker: str, end_marker: str, replacement: str) -> None:
    global text
    start = text.find(start_marker)
    if start < 0:
        raise SystemExit(f"start marker not found: {start_marker!r}")
    end = text.find(end_marker, start)
    if end < 0:
        raise SystemExit(f"end marker not found: {end_marker!r}")
    if text.find(start_marker, start + 1) >= 0:
        raise SystemExit(f"start marker is not unique: {start_marker!r}")
    text = text[:start] + replacement + text[end:]


replace_once(
    "use serde::Serialize;\n",
    "use serde::Serialize;\nuse std::ffi::OsString;\nuse std::path::Path;\n",
)

replace_section(
    "fn source_for_persistence(",
    "\n/// apply を実行し、成功後に State を core 内で保存する",
    '''fn source_for_persistence<P: AsRef<Path> + ?Sized>(
    repo: &P,
    previous: Option<&State>,
    tc: &ToolInventory,
) -> Result<Option<crate::source::SourceState>> {
    let stored = previous.and_then(|state| state.source.as_ref());
    match tc.git.as_ref() {
        Some(git) => crate::source::SourceResolver::new()
            .resolve(repo, git, stored)
            .map(Some),
        None => Ok(stored.cloned()),
    }
}
''',
)

replace_section(
    "pub fn apply(",
    "\n/// rollback を実行し、State を更新して core 内で保存する",
    '''pub fn apply<P: AsRef<Path> + ?Sized>(
    target: &ConfigurationTarget,
    repo: &P,
    store: &StateStore,
    tc: &ToolInventory,
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = acquire()?;
    let repo = repo.as_ref();
    let prev = store.load()?;
    let source = source_for_persistence(repo, prev.as_ref(), tc)?;

    let source_ref = crate::source::effective_source_ref(repo, store)?;
    let output = if capture {
        Some(actions::apply_captured_native(target, repo, &source_ref, tc)?)
    } else {
        actions::apply_native(target, repo, &source_ref, tc)?;
        None
    };

    // managed source は revision 記録を、それ以外は checkout の git revision
    // を applied revision に記録する
    let revision = match source.as_ref().filter(|src| src.is_managed_release()) {
        Some(src) => src.revision.clone(),
        None => tc
            .git
            .as_ref()
            .and_then(|g| current_git_revision(repo, &g.path)),
    };
    let mut state = applied_state(target, revision);
    // profile 選択と source の現在状態は applied 情報と独立して保持する。
    state.profile = prev.as_ref().and_then(|s| s.profile.clone());
    state.source = source;
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}
''',
)

replace_section(
    "pub fn rollback(",
    "\n/// upgrade (`nix flake update --flake <repo>`) をロック付きで実行する。",
    '''pub fn rollback<P: AsRef<Path> + ?Sized>(
    target: &ConfigurationTarget,
    repo: &P,
    store: &StateStore,
    tc: &ToolInventory,
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = acquire()?;
    let repo = repo.as_ref();
    let prev = store.load()?;
    let source = source_for_persistence(repo, prev.as_ref(), tc)?;

    let source_ref = crate::source::effective_source_ref(repo, store)?;
    let output = if capture {
        Some(actions::rollback_captured_native(target, repo, &source_ref, tc)?)
    } else {
        actions::rollback_native(target, repo, &source_ref, tc)?;
        None
    };

    let mut state = rolled_back_state(target);
    // profile 選択と source の現在状態は rollback を跨いでも保持する。
    state.profile = prev.as_ref().and_then(|s| s.profile.clone());
    state.source = source;
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}
''',
)

replace_section(
    "pub fn upgrade(repo: &str, tc: &ToolInventory, capture: bool) -> Result<Option<String>> {",
    "\n/// plan の結果 (dry-run build)",
    '''pub fn upgrade<P: AsRef<Path> + ?Sized>(
    repo: &P,
    tc: &ToolInventory,
    capture: bool,
) -> Result<Option<String>> {
    let _guard = acquire()?;
    let repo = repo.as_ref();
    let output = if capture {
        Some(actions::upgrade_captured_native(repo, tc)?)
    } else {
        actions::upgrade_native(repo, tc)?;
        None
    };
    Ok(output)
}
''',
)

replace_section(
    "pub fn plan_target(repo: &str) -> Result<PlanResult> {",
    "\n/// plan: 適用内容の dry-run を実行する (CWD 非依存)",
    '''pub fn plan_target<P: AsRef<Path> + ?Sized>(repo: &P) -> Result<PlanResult> {
    plan_target_with(repo, &StateStore::default())
}

fn plan_source(
    repo: &Path,
    store: &StateStore,
) -> Result<(ConfigurationTarget, crate::source::EffectiveSourceRef)> {
    let target = detect_target();
    if !target.is_supported() {
        return Err(Error::UnsupportedPlatform {
            os: target.platform().to_string(),
            arch: target.architecture().to_string(),
        });
    }
    let source_ref = crate::source::effective_source_ref(repo, store)?;
    Ok((target, source_ref))
}

fn plan_result(
    target: &ConfigurationTarget,
    source_ref: &crate::source::EffectiveSourceRef,
) -> PlanResult {
    let fragment = target.build_fragment();
    PlanResult {
        host: target.name().to_string(),
        flake_target: format!("{}{}", source_ref.display_lossy(), fragment),
        output: None,
    }
}

/// [`plan_target`] の state store 注入版 (test 用)
pub fn plan_target_with<P: AsRef<Path> + ?Sized>(
    repo: &P,
    store: &StateStore,
) -> Result<PlanResult> {
    let (target, source_ref) = plan_source(repo.as_ref(), store)?;
    Ok(plan_result(&target, &source_ref))
}
''',
)

replace_section(
    "pub fn plan(repo: &str, tc: &ToolInventory, capture: bool) -> Result<PlanResult> {",
    "\n/// verify の個別チェック",
    '''pub fn plan<P: AsRef<Path> + ?Sized>(
    repo: &P,
    tc: &ToolInventory,
    capture: bool,
) -> Result<PlanResult> {
    let repo = repo.as_ref();
    let store = StateStore::default();
    let (target, source_ref) = plan_source(repo, &store)?;
    let mut result = plan_result(&target, &source_ref);
    let nix = tc.require_nix()?;
    let mut args = vec![OsString::from("build"), OsString::from("--dry-run")];
    args.extend(profile::override_args(repo)?.into_iter().map(OsString::from));
    args.push(source_ref.with_fragment(&target.build_fragment()));
    result.output = if capture {
        Some(run_capture(&nix.path, &args)?)
    } else {
        run_stream(&nix.path, &args)?;
        None
    };
    Ok(result)
}
''',
)

OPS.write_text(text)
