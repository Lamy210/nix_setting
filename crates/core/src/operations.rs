use crate::actions;
use crate::discovery::ConfigurationTarget;
use crate::error::{Error, Result};
use crate::lock::{OperationGuard, OperationLock};
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
pub fn rollback(
    target: &ConfigurationTarget,
    store: &StateStore,
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = acquire()?;

    let output = if capture {
        Some(actions::rollback_captured(target)?)
    } else {
        actions::rollback(target)?;
        None
    };

    let state = rolled_back_state(target);
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}

/// upgrade (`nix flake update`) をロック付きで実行する
pub fn upgrade(capture: bool) -> Result<Option<String>> {
    let _guard = acquire()?;
    let output = if capture {
        Some(actions::upgrade_captured()?)
    } else {
        actions::upgrade()?;
        None
    };
    Ok(output)
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
}
