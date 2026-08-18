use crate::actions;
use crate::discovery::{detect_target, ConfigurationTarget};
use crate::error::{Error, Result};
use crate::lock::{OperationGuard, OperationLock};
use crate::machine;
use crate::process::{run_capture, run_stream};
use crate::repo::current_git_revision;
use crate::state::{State, StateStore};
use crate::time::now_iso8601;
use crate::tool::ToolInventory;
use serde::Serialize;

/// machine input の `--override-input` 引数 (actions と同じものを plan でも使う)
fn machine_override_args() -> Result<Vec<String>> {
    let facts = machine::MachineFacts::detect()?;
    let path = machine::write_machine_input(&facts)?;
    Ok(vec![
        "--override-input".to_string(),
        "machine".to_string(),
        path.to_string_lossy().to_string(),
    ])
}

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

    let state = applied_state(
        target,
        tc.git
            .as_ref()
            .and_then(|g| current_git_revision(repo, &g.path)),
    );
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

    let state = rolled_back_state(target);
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}

/// upgrade (`nix flake update --flake <repo>`) をロック付きで実行する
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
    args.extend(machine_override_args()?);
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
