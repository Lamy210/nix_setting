use crate::actions;
use crate::discovery::{detect_target, which, ConfigurationTarget};
use crate::error::{Error, Result};
use crate::lock::{OperationGuard, OperationLock};
use crate::process::{run_capture, run_stream};
use crate::repo::current_git_revision;
use crate::state::{State, StateStore};
use crate::time::now_iso8601;

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
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = acquire()?;

    let output = if capture {
        Some(actions::apply_captured(target, repo)?)
    } else {
        actions::apply(target, repo)?;
        None
    };

    let state = applied_state(target, current_git_revision(repo));
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
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = acquire()?;

    let output = if capture {
        Some(actions::rollback_captured(target, repo)?)
    } else {
        actions::rollback(target, repo)?;
        None
    };

    let state = rolled_back_state(target);
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}

/// upgrade (`nix flake update --flake <repo>`) をロック付きで実行する
pub fn upgrade(repo: &str, capture: bool) -> Result<Option<String>> {
    let _guard = acquire()?;
    let output = if capture {
        Some(actions::upgrade_captured(repo)?)
    } else {
        actions::upgrade(repo)?;
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
pub fn plan(repo: &str, capture: bool) -> Result<PlanResult> {
    let mut result = plan_target(repo)?;
    let args = [
        "build".to_string(),
        "--dry-run".to_string(),
        result.flake_target.clone(),
    ];
    result.output = if capture {
        Some(run_capture("nix", &args)?)
    } else {
        run_stream("nix", &args)?;
        None
    };
    Ok(result)
}

/// verify の個別チェック
#[derive(Debug, Clone)]
pub struct VerifyCheck {
    pub name: String,
    pub ok: bool,
}

/// verify の結果
#[derive(Debug, Clone)]
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

/// verify: 環境・repo/manifest・state を検証する
pub fn verify(repo: &str) -> Result<VerifyReport> {
    let mut checks = Vec::new();

    for cmd in ["nix", "zsh", "git"] {
        checks.push(VerifyCheck {
            name: cmd.to_string(),
            ok: which(cmd).is_some(),
        });
    }

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
        name: "manifest".to_string(),
        ok: std::path::Path::new(&format!("{repo}/config.toml")).is_file(),
    });
    checks.push(VerifyCheck {
        name: "state".to_string(),
        ok: StateStore::default().load().is_some(),
    });

    Ok(VerifyReport { checks })
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

/// sync: dirty check の後 `git -C <repo> pull --ff-only` で更新する
pub fn sync(repo: &str, capture: bool) -> Result<Option<String>> {
    let _guard = acquire()?;

    if git_dirty(repo)? {
        return Err(Error::Busy(
            "repository has uncommitted changes; commit or stash first".to_string(),
        ));
    }

    let args = sync_args(repo);
    let output = if capture {
        Some(run_capture("git", &args)?)
    } else {
        run_stream("git", &args)?;
        None
    };
    Ok(output)
}

/// repository の working tree に未コミット変更があるか
fn git_dirty(repo: &str) -> Result<bool> {
    let out = run_capture(
        "git",
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

    #[test]
    fn applied_state_contains_host_and_revision() {
        let target = detect_target_for("macos", "aarch64");
        let state = applied_state(&target, Some("abc123".to_string()));
        assert_eq!(state.host.as_deref(), Some("macbook-air"));
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
            "/tmp/repo#darwinConfigurations.macbook-air.system"
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
}
