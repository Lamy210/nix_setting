use crate::actions;
use crate::discovery::{detect_target, ConfigurationTarget};
use crate::error::{Error, Result};
use crate::lock::{OperationGuard, OperationLock};
use crate::machine;
use crate::process::{run_capture, run_stream};
use crate::profile;
use crate::repo::current_git_revision;
use crate::state::{State, StateStore};
use crate::time::now_iso8601;
use crate::tool::ToolInventory;
use serde::Serialize;

/// apply / rollback の結果。output は capture 時のみ Some
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub output: Option<String>,
    pub state: State,
}

/// 排他ロックを取得する。取得できない場合は Busy エラーを返す
fn acquire() -> Result<OperationGuard> {
    match OperationLock::global().try_acquire()? {
        Some(guard) => Ok(guard),
        None => Err(Error::Busy("another operation is in progress".to_string())),
    }
}

/// apply 成功後の State を構築する純関数
pub fn applied_state(target: &ConfigurationTarget, revision: Option<String>) -> State {
    State {
        host: Some(target.name().to_string()),
        applied_revision: revision,
        applied_at: Some(now_iso8601()),
        product_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        source: None,
        profile: None,
    }
}

/// rollback 後の State を構築する純関数
/// (世代ロールバック後の applied_revision は特定できないため None)
pub fn rolled_back_state(target: &ConfigurationTarget) -> State {
    State {
        host: Some(target.name().to_string()),
        applied_revision: None,
        applied_at: Some(now_iso8601()),
        product_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        source: None,
        profile: None,
    }
}

/// apply を実行し、成功後に State を core 内で保存する (CLI/GUI 共通)
///
/// - `capture == true`: 出力をキャプチャして返す (GUI 用)
/// - `capture == false`: stdio 継承のストリーミング実行 (CLI 用)
/// - 操作はクロスプロセス・ロックで直列化される
pub fn apply(
    target: &ConfigurationTarget,
    repo: &str,
    store: &StateStore,
    tc: &ToolInventory,
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = acquire()?;

    let output = if capture {
        Some(actions::apply_captured(target, repo, tc)?)
    } else {
        actions::apply(target, repo, tc)?;
        None
    };

    let mut state = applied_state(
        target,
        tc.git
            .as_ref()
            .and_then(|g| current_git_revision(repo, &g.path)),
    );
    // profile 選択は user の恒久的な選択のため apply を跨いで保持する
    state.profile = store.load().and_then(|s| s.profile);
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}

/// rollback を実行し、State を更新して core 内で保存する (CLI/GUI 共通)
///
/// `repo` は macOS の pinned rollback (`--inputs-from <repo>`) で使用する。
pub fn rollback(
    target: &ConfigurationTarget,
    repo: &str,
    store: &StateStore,
    tc: &ToolInventory,
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = acquire()?;

    let output = if capture {
        Some(actions::rollback_captured(target, repo, tc)?)
    } else {
        actions::rollback(target, repo, tc)?;
        None
    };

    let mut state = rolled_back_state(target);
    // profile 選択は rollback を跨いでも保持する
    state.profile = store.load().and_then(|s| s.profile);
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}

/// upgrade (`nix flake update --flake <repo>`) をロック付きで実行する。
/// v0.3 までの alias。本体は [`deps_update`]。
pub fn upgrade(repo: &str, tc: &ToolInventory, capture: bool) -> Result<Option<String>> {
    let _guard = acquire()?;
    let output = if capture {
        Some(actions::upgrade_captured(repo, tc)?)
    } else {
        actions::upgrade(repo, tc)?;
        None
    };
    Ok(output)
}

/// plan の結果 (dry-run build)
#[derive(Debug, Clone)]
pub struct PlanResult {
    pub host: String,
    pub flake_target: String,
    pub output: Option<String>,
}

/// plan 対象 (host / flake target) を計算する純関数。コマンドは実行しない
pub fn plan_target(repo: &str) -> Result<PlanResult> {
    let target = detect_target();
    if !target.is_supported() {
        return Err(Error::UnsupportedPlatform {
            os: target.platform().to_string(),
            arch: target.architecture().to_string(),
        });
    }
    Ok(PlanResult {
        host: target.name().to_string(),
        flake_target: target.build_ref(repo),
        output: None,
    })
}

/// plan: 適用内容の dry-run を実行する (CWD 非依存)
pub fn plan(repo: &str, tc: &ToolInventory, capture: bool) -> Result<PlanResult> {
    let mut result = plan_target(repo)?;
    let nix = tc.require_nix()?;
    let mut args = vec!["build".to_string(), "--dry-run".to_string()];
    args.extend(profile::override_args(repo)?);
    args.push(result.flake_target.clone());
    result.output = if capture {
        Some(run_capture(&nix.path, &args)?)
    } else {
        run_stream(&nix.path, &args)?;
        None
    };
    Ok(result)
}

/// verify の個別チェック
#[derive(Debug, Clone, Serialize)]
pub struct VerifyCheck {
    pub name: String,
    pub ok: bool,
}

/// verify の結果
#[derive(Debug, Clone, Serialize)]
pub struct VerifyReport {
    pub checks: Vec<VerifyCheck>,
}

impl VerifyReport {
    pub fn passed(&self) -> usize {
        self.checks.iter().filter(|c| c.ok).count()
    }

    pub fn failed(&self) -> usize {
        self.checks.iter().filter(|c| !c.ok).count()
    }

    pub fn is_ok(&self) -> bool {
        self.failed() == 0
    }
}

/// verify: 環境・repo/manifest・state を検証する (各検査は infallible)
pub fn verify(repo: &str, tc: &ToolInventory) -> VerifyReport {
    let mut checks = Vec::new();

    // discover 済み inventory の各ツールが実際に実行可能か
    checks.push(VerifyCheck {
        name: "nix".to_string(),
        ok: tc.nix.as_ref().is_some_and(|t| t.path.is_file()),
    });
    checks.push(VerifyCheck {
        name: "git".to_string(),
        ok: tc.git.as_ref().is_some_and(|t| t.path.is_file()),
    });
    // zsh は shell 必須だが inventory 対象外なので PATH 探索
    checks.push(VerifyCheck {
        name: "zsh".to_string(),
        ok: crate::discovery::which("zsh").is_some(),
    });

    let home = std::env::var("HOME").unwrap_or_default();
    for (name, path) in [
        (".zshrc", format!("{home}/.zshrc")),
        (".gitconfig", format!("{home}/.gitconfig")),
        ("starship.toml", format!("{home}/.config/starship.toml")),
    ] {
        checks.push(VerifyCheck {
            name: name.to_string(),
            ok: std::path::Path::new(&path).exists(),
        });
    }

    checks.push(VerifyCheck {
        name: "repository".to_string(),
        ok: std::path::Path::new(repo).is_dir(),
    });
    checks.push(VerifyCheck {
        name: "machine input".to_string(),
        ok: machine::default_machine_nix_path().is_file(),
    });
    checks.push(VerifyCheck {
        name: "state".to_string(),
        ok: StateStore::default().load().is_some(),
    });

    VerifyReport { checks }
}

/// sync の引数を構築する (`git -C <repo> pull --ff-only`)
fn sync_args(repo: &str) -> Vec<String> {
    vec![
        "-C".to_string(),
        repo.to_string(),
        "pull".to_string(),
        "--ff-only".to_string(),
    ]
}

/// checkout 中の branch 名。detached HEAD (release tag の depth-1 clone 等) では None
fn current_branch(repo: &str, git: &crate::tool::ResolvedTool) -> Result<Option<String>> {
    let out = run_capture(
        &git.path,
        &[
            "-C".to_string(),
            repo.to_string(),
            "symbolic-ref".to_string(),
            "--short".to_string(),
            "HEAD".to_string(),
        ],
    );
    match out {
        Ok(branch) => {
            let branch = branch.trim();
            if branch.is_empty() {
                Ok(None)
            } else {
                Ok(Some(branch.to_string()))
            }
        }
        Err(_) => Ok(None),
    }
}

/// sync: dirty check と branch checkout の確認の後 `git pull --ff-only` で更新する。
/// detached HEAD (install.sh の release tag pin clone) は pull できず失敗するため、
/// clean no-op として pinned である旨を返す。
pub fn sync(repo: &str, tc: &ToolInventory, capture: bool) -> Result<Option<String>> {
    sync_with_lock(repo, tc, capture, OperationLock::global())
}

/// [`sync`] の lock を注入可能にした内部実装 (test は独立した lock path を使う)。
/// precondition (git 解決) は lock 取得の前に評価する — lock file の作成先が
/// read-only の環境 (nix build の checkPhase sandbox 等) でも precondition error
/// を正しく返せるようにするため。
fn sync_with_lock(
    repo: &str,
    tc: &ToolInventory,
    capture: bool,
    lock: &OperationLock,
) -> Result<Option<String>> {
    let git = tc.require_git()?;
    let _guard = match lock.try_acquire()? {
        Some(guard) => guard,
        None => return Err(Error::Busy("another operation is in progress".to_string())),
    };

    if git_dirty(repo, git)? {
        return Err(Error::Busy(
            "repository has uncommitted changes; commit or stash first".to_string(),
        ));
    }

    if current_branch(repo, git)?.is_none() {
        let note = "Repository is pinned to a release checkout (detached HEAD). No branch sync was performed.";
        if capture {
            return Ok(Some(note.to_string()));
        }
        println!("{note}");
        return Ok(None);
    }

    let args = sync_args(repo);
    let output = if capture {
        Some(run_capture(&git.path, &args)?)
    } else {
        run_stream(&git.path, &args)?;
        None
    };
    Ok(output)
}

/// repository の working tree に未コミット変更があるか
fn git_dirty(repo: &str, git: &crate::tool::ResolvedTool) -> Result<bool> {
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
}

// ---------------------------------------------------------------------------
// v2 P1: ConfigurationSource に基づく update 体系 (ADR-0003)
// ---------------------------------------------------------------------------

/// update の dispatch 先を表す純関数の結果 (test 可能にするため分離)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateAction {
    /// 同 channel の最新 release tag へ checkout
    CheckoutLatestTag { channel: String },
    /// fetch + pull --ff-only
    FastForward,
    /// 更新しない (案内表示のみ)
    NoOp(String),
}

/// source kind から update の動作を決める純関数
pub fn dispatch_update(kind: crate::source::SourceKind) -> UpdateAction {
    use crate::source::SourceKind;
    match kind {
        SourceKind::ReleaseStable => UpdateAction::CheckoutLatestTag {
            channel: "stable".to_string(),
        },
        SourceKind::ReleasePreview => UpdateAction::CheckoutLatestTag {
            channel: "preview".to_string(),
        },
        SourceKind::GitTracking => UpdateAction::FastForward,
        SourceKind::GitPinned => UpdateAction::NoOp(
            "Source is pinned to a tag/commit. Re-pin manually (git checkout <ref>) to change it."
                .to_string(),
        ),
        SourceKind::Local => {
            UpdateAction::NoOp("Source is a local directory. Manage updates yourself.".to_string())
        }
    }
}

/// update の結果
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub output: Option<String>,
    pub source: Option<crate::source::SourceState>,
}

/// update: source kind に応じて configuration source を更新する。
/// flake.lock はどの経路でも更新しない (release 単位の検証を保持)。
pub fn update(
    repo: &str,
    store: &StateStore,
    tc: &ToolInventory,
    capture: bool,
) -> Result<UpdateResult> {
    let git = tc.require_git()?;
    let _guard = acquire()?;

    let state = crate::source::SourceResolver::new().detect(repo, git)?;
    let action = dispatch_update(state.kind);

    let output = match action {
        UpdateAction::CheckoutLatestTag { channel } => {
            update_release(repo, git, &channel, capture)?
        }
        UpdateAction::FastForward => {
            // GitTracking: sync と同じ dirty check + pull --ff-only
            if git_dirty(repo, git)? {
                return Err(Error::Busy(
                    "repository has uncommitted changes; commit or stash first".to_string(),
                ));
            }
            let args = sync_args(repo);
            if capture {
                Some(run_capture(&git.path, &args)?)
            } else {
                run_stream(&git.path, &args)?;
                None
            }
        }
        UpdateAction::NoOp(note) => {
            if capture {
                Some(note)
            } else {
                println!("{note}");
                None
            }
        }
    };

    // 更新後の source 状態を State へ反映 (applied 情報は変えない)
    let new_source = crate::source::SourceResolver::new().detect(repo, git).ok();
    let mut saved = store.load().unwrap_or_default();
    saved.source = new_source.clone();
    store.save(&saved)?;

    Ok(UpdateResult {
        output,
        source: new_source,
    })
}

/// release channel 内の最新 tag へ checkout する。
/// dirty working tree は中止。候補は fetch 済み tag のみ (offline 安全)。
fn update_release(
    repo: &str,
    git: &crate::tool::ResolvedTool,
    channel: &str,
    capture: bool,
) -> Result<Option<String>> {
    if git_dirty(repo, git)? {
        return Err(Error::Busy(
            "repository has uncommitted changes; commit or stash first".to_string(),
        ));
    }

    // 新しい tag を取得 (network。失敗したら local tag のみで続行)
    let fetch_args = vec![
        "-C".to_string(),
        repo.to_string(),
        "fetch".to_string(),
        "--tags".to_string(),
        "--quiet".to_string(),
    ];
    let _ = run_capture(&git.path, &fetch_args);

    let tags = list_tags(repo, git)?;
    let current = current_checkout_ref(repo, git);
    let latest = crate::source::latest_tag_for_channel(&tags, channel);

    let Some(latest) = latest else {
        let current_display = current.unwrap_or_else(|| "(unknown)".to_string());
        let note = format!(
            "No {channel} release tags found; nothing to update (current: {current_display})."
        );
        if capture {
            return Ok(Some(note));
        }
        println!("{note}");
        return Ok(None);
    };

    if Some(latest.as_str()) == current.as_deref() {
        let note = format!("Already on the latest {channel} release ({latest}).");
        if capture {
            return Ok(Some(note));
        }
        println!("{note}");
        return Ok(None);
    }

    let checkout_args = vec![
        "-C".to_string(),
        repo.to_string(),
        "checkout".to_string(),
        "--quiet".to_string(),
        latest.clone(),
    ];
    let output = if capture {
        Some(run_capture(&git.path, &checkout_args)?)
    } else {
        run_stream(&git.path, &checkout_args)?;
        None
    };
    if !capture {
        println!("updated to {latest}");
    }
    Ok(output.map(|_| format!("updated to {latest}")))
}

/// local の tag 一覧 (`git tag`)
fn list_tags(repo: &str, git: &crate::tool::ResolvedTool) -> Result<Vec<String>> {
    let out = run_capture(
        &git.path,
        &[
            "-C".to_string(),
            repo.to_string(),
            "tag".to_string(),
            "--list".to_string(),
        ],
    )?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// 現在 checkout されている ref (exact tag があれば tag 名)
fn current_checkout_ref(repo: &str, git: &crate::tool::ResolvedTool) -> Option<String> {
    let out = run_capture(
        &git.path,
        &[
            "-C".to_string(),
            repo.to_string(),
            "describe".to_string(),
            "--tags".to_string(),
            "--exact-match".to_string(),
        ],
    )
    .ok()?;
    let tag = out.trim();
    if tag.is_empty() {
        None
    } else {
        Some(tag.to_string())
    }
}

/// source sync (Advanced): 従来 sync の git pull --ff-only。
/// Tracking 以外の source では kind を説明する no-op note を返す。
pub fn source_sync(repo: &str, tc: &ToolInventory, capture: bool) -> Result<Option<String>> {
    let git = tc.require_git()?;
    let state = crate::source::SourceResolver::new().detect(repo, git)?;
    if state.kind != crate::source::SourceKind::GitTracking {
        let note = format!(
            "source sync is only meaningful for git-tracking sources (current: {}). \
             Use `schneeforge update` instead.",
            state.kind
        );
        if capture {
            return Ok(Some(note));
        }
        println!("{note}");
        return Ok(None);
    }
    sync(repo, tc, capture)
}

/// source deps update (Advanced): `nix flake update`。
/// Release channel では release 検証単位から外れる警告を先頭に付ける。
pub fn deps_update(repo: &str, tc: &ToolInventory, capture: bool) -> Result<Option<String>> {
    let warning = release_lock_warning(repo, tc);
    let output = upgrade(repo, tc, capture)?;
    Ok(match (warning, output) {
        (Some(w), Some(o)) => Some(format!("{w}\n{o}")),
        (Some(w), None) => {
            println!("{w}");
            None
        }
        (None, o) => o,
    })
}

/// Release source で flake.lock を更新する場合の警告文
fn release_lock_warning(repo: &str, tc: &ToolInventory) -> Option<String> {
    let git = tc.git.as_ref()?;
    let state = crate::source::SourceResolver::new()
        .detect(repo, git)
        .ok()?;
    if state.kind.is_release() {
        Some(
            "warning: this source is a release checkout (verified as a unit: source revision \
             + flake.lock). Updating flake.lock moves it off the verified release. \
             Prefer `schneeforge update` to move to a newer release."
                .to_string(),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::detect_target_for;
    use crate::tool::{ResolvedTool, ToolSource};
    use std::path::PathBuf;

    fn dummy_tc() -> ToolInventory {
        ToolInventory {
            nix: Some(ResolvedTool::new(
                PathBuf::from("/usr/local/bin/nix"),
                ToolSource::Homebrew,
            )),
            git: Some(ResolvedTool::new(
                PathBuf::from("/usr/bin/git"),
                ToolSource::Path,
            )),
            homebrew: None,
            nh: None,
        }
    }

    #[test]
    fn applied_state_contains_host_and_revision() {
        let target = detect_target_for("macos", "aarch64");
        let state = applied_state(&target, Some("abc123".to_string()));
        assert_eq!(state.host.as_deref(), Some("darwin-aarch64"));
        assert_eq!(state.applied_revision.as_deref(), Some("abc123"));
        assert!(state.applied_at.is_some());
        assert!(state.product_version.is_some());
    }

    #[test]
    fn rolled_back_state_clears_revision() {
        let target = detect_target_for("linux", "x86_64");
        let state = rolled_back_state(&target);
        assert_eq!(state.host.as_deref(), Some("linux"));
        assert_eq!(state.applied_revision, None);
        assert!(state.applied_at.is_some());
    }

    #[test]
    fn plan_build_ref_macos() {
        let target = detect_target_for("macos", "aarch64");
        assert_eq!(
            target.build_ref("/tmp/repo"),
            "/tmp/repo#darwinConfigurations.darwin-aarch64.system"
        );
    }

    #[test]
    fn plan_build_ref_linux() {
        let target = detect_target_for("linux", "x86_64");
        assert_eq!(
            target.build_ref("/tmp/repo"),
            "/tmp/repo#homeConfigurations.linux.activationPackage"
        );
    }

    #[test]
    fn sync_args_are_repo_aware() {
        assert_eq!(
            sync_args("/tmp/repo"),
            vec![
                "-C".to_string(),
                "/tmp/repo".to_string(),
                "pull".to_string(),
                "--ff-only".to_string(),
            ]
        );
    }

    #[test]
    fn dispatch_update_matches_source_kinds() {
        use crate::source::SourceKind;
        assert_eq!(
            dispatch_update(SourceKind::ReleaseStable),
            UpdateAction::CheckoutLatestTag {
                channel: "stable".to_string()
            }
        );
        assert_eq!(
            dispatch_update(SourceKind::ReleasePreview),
            UpdateAction::CheckoutLatestTag {
                channel: "preview".to_string()
            }
        );
        assert_eq!(
            dispatch_update(SourceKind::GitTracking),
            UpdateAction::FastForward
        );
        assert!(matches!(
            dispatch_update(SourceKind::GitPinned),
            UpdateAction::NoOp(_)
        ));
        assert!(matches!(
            dispatch_update(SourceKind::Local),
            UpdateAction::NoOp(_)
        ));
    }

    #[test]
    fn release_lock_warning_only_for_release_sources() {
        // Local source (git 管理外) では警告なし
        let dir = std::env::temp_dir().join(format!("sf-warn-local-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(release_lock_warning(dir.to_str().unwrap(), &dummy_tc()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_report_counts_checks() {
        let report = VerifyReport {
            checks: vec![
                VerifyCheck {
                    name: "a".to_string(),
                    ok: true,
                },
                VerifyCheck {
                    name: "b".to_string(),
                    ok: true,
                },
                VerifyCheck {
                    name: "c".to_string(),
                    ok: false,
                },
            ],
        };
        assert_eq!(report.passed(), 2);
        assert_eq!(report.failed(), 1);
        assert!(!report.is_ok());
    }

    #[test]
    fn verify_uses_resolved_inventory_paths() {
        // inventory が指すパスが file として存在するかで判定される。
        // dummy_tc の /usr/local/bin/nix は存在しないので ok=false になるはず
        let report = verify("/tmp", &dummy_tc());
        let nix_check = report
            .checks
            .iter()
            .find(|c| c.name == "nix")
            .expect("nix check should exist");
        assert!(!nix_check.ok, "dummy /usr/local/bin/nix should not exist");
    }

    #[test]
    fn sync_returns_git_not_found_when_git_missing() {
        // Git 未解決の環境では sync は GitNotFound (Precondition) で弾かれる。
        // 独立 lock + precondition を lock 前に評価することで、lock file の作成先が
        // read-only な環境 (nix build の checkPhase sandbox) でも正しく弾ける
        let lock = OperationLock::new(
            std::env::temp_dir().join(format!("sf-lock-{}-git-missing", std::process::id())),
        );
        let tc = ToolInventory {
            nix: Some(ResolvedTool::new(
                PathBuf::from("/usr/local/bin/nix"),
                ToolSource::Homebrew,
            )),
            git: None,
            homebrew: None,
            nh: None,
        };
        let err = sync_with_lock("/tmp/repo", &tc, false, &lock).unwrap_err();
        assert!(
            err.to_string().contains("git not found"),
            "expected git-not-found message, got: {err}"
        );
    }

    /// 実 git で temp repository を作る helper。git binary が無い環境では skip する
    fn git_repo_fixture(name: &str) -> Option<(PathBuf, PathBuf)> {
        let git_bin = PathBuf::from("git");
        let dir = std::env::temp_dir().join(format!("sf-sync-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        let ok = |args: &[&str]| -> bool {
            std::process::Command::new(&git_bin)
                .current_dir(&dir)
                .args(args)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };
        if !ok(&["init", "-q"]) {
            return None;
        }
        if !ok(&["config", "user.email", "test@schneeforge.invalid"]) {
            return None;
        }
        if !ok(&["config", "user.name", "SchneeForge Test"]) {
            return None;
        }
        std::fs::write(dir.join("README.md"), "# test\n").ok()?;
        if !ok(&["add", "."]) || !ok(&["commit", "-q", "-m", "init"]) {
            return None;
        }
        Some((dir, git_bin))
    }

    fn resolved_git(git_bin: &std::path::Path) -> ResolvedTool {
        ResolvedTool::new(git_bin.to_path_buf(), ToolSource::Path)
    }

    #[test]
    fn current_branch_is_some_on_branch_checkout() {
        let Some((repo, git_bin)) = git_repo_fixture("branch") else {
            eprintln!("skipping: git not available");
            return;
        };
        let branch = current_branch(repo.to_str().unwrap(), &resolved_git(&git_bin)).unwrap();
        // git init 直後は branch checkout (master / main 等) のはず
        assert!(branch.is_some(), "expected branch checkout after git init");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn sync_is_noop_on_release_tag_detached_checkout() {
        // regression (PR #18 review P1): install.sh は fresh clone を
        // `git clone --branch <tag> --depth 1` で行うため detached HEAD になる。
        // `git pull --ff-only` は追跡 branch 無しで失敗するため、sync は
        // error ではなく clean no-op (pinned 案内) として扱わなければならない
        let Some((src, git_bin)) = git_repo_fixture("tagged") else {
            eprintln!("skipping: git not available");
            return;
        };
        let git = resolved_git(&git_bin);
        let tag = "v0.2.0-rc.2";
        let run = |args: &[&str], cwd: &std::path::Path| -> bool {
            std::process::Command::new(&git_bin)
                .current_dir(cwd)
                .args(args)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };
        assert!(run(&["tag", tag], &src), "tag creation failed");

        // install.sh と同じ形式の clone: --branch <tag> --depth 1 → detached HEAD
        let clone_dir = std::env::temp_dir().join(format!("sf-sync-clone-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&clone_dir);
        assert!(run(
            &[
                "clone",
                "--branch",
                tag,
                "--depth",
                "1",
                src.to_str().unwrap(),
                clone_dir.to_str().unwrap(),
            ],
            &std::env::temp_dir(),
        ));

        // 前提確認: この clone は実際に detached HEAD になっている
        let branch = current_branch(clone_dir.to_str().unwrap(), &git).unwrap();
        assert!(
            branch.is_none(),
            "clone --branch <tag> should be detached, got branch: {branch:?}"
        );

        // sync は raw git pull error にならず pinned として扱われる
        let tc = ToolInventory {
            git: Some(git),
            ..dummy_tc()
        };
        let out = sync(clone_dir.to_str().unwrap(), &tc, true).unwrap();
        let msg = out.expect("capture mode should return the pinned note");
        assert!(
            msg.contains("pinned to a release checkout"),
            "expected pinned note, got: {msg}"
        );
        assert!(
            !msg.contains("fatal"),
            "should not surface raw git error: {msg}"
        );

        // 対称性: 通常の branch checkout は pinned 扱いにならず pull が走る。
        // sync は global lock を取るため、同一 test 内で直列に検証する
        // (cargo test は test を並列実行し、別 test での lock 競合が Busy になる)
        let branch_clone =
            std::env::temp_dir().join(format!("sf-sync-branch-clone-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&branch_clone);
        assert!(
            run(
                &[
                    "clone",
                    "-q",
                    src.to_str().unwrap(),
                    branch_clone.to_str().unwrap(),
                ],
                &std::env::temp_dir(),
            ),
            "branch clone failed"
        );
        let tc_branch = ToolInventory {
            git: Some(resolved_git(&git_bin)),
            ..dummy_tc()
        };
        let out = sync(branch_clone.to_str().unwrap(), &tc_branch, true).unwrap();
        let msg = out.expect("capture mode should return pull output");
        assert!(
            !msg.contains("pinned to a release checkout"),
            "branch checkout must not be treated as pinned: {msg}"
        );

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&clone_dir);
        let _ = std::fs::remove_dir_all(&branch_clone);
    }
}
