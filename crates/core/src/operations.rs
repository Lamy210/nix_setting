use crate::actions;
use crate::discovery::ConfigurationTarget;
use crate::error::Result;
use crate::lock::OperationLock;
use crate::repo::current_git_revision;
use crate::state::{State, StateStore};
use crate::time::now_iso8601;

/// apply / rollback の結果。output は capture 時のみ Some
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub output: Option<String>,
    pub state: State,
}

/// apply を実行し、成功後に State を core 内で保存する (CLI/GUI 共通)
///
/// - `capture == true`: 出力をキャプチャして返す (GUI 用)
/// - `capture == false`: stdio 継承のストリーミング実行 (CLI 用)
/// - 操作は process-wide ロックで直列化される
pub fn apply(
    target: &ConfigurationTarget,
    repo: &str,
    store: &StateStore,
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = OperationLock::global().acquire();

    let output = if capture {
        Some(actions::apply_captured(target, repo)?)
    } else {
        actions::apply(target, repo)?;
        None
    };

    let state = State {
        host: Some(target.name().to_string()),
        applied_revision: current_git_revision(repo),
        applied_at: Some(now_iso8601()),
        product_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}

/// rollback を実行し、State を更新して core 内で保存する (CLI/GUI 共通)
///
/// 世代ロールバック後の applied_revision は特定できないため None にする
/// (generation 追跡は rollback(repo) の世代ロールバック実装で対応)。
pub fn rollback(
    target: &ConfigurationTarget,
    store: &StateStore,
    capture: bool,
) -> Result<ApplyResult> {
    let _guard = OperationLock::global().acquire();

    let output = if capture {
        Some(actions::rollback_captured(target)?)
    } else {
        actions::rollback(target)?;
        None
    };

    let state = State {
        host: Some(target.name().to_string()),
        applied_revision: None,
        applied_at: Some(now_iso8601()),
        product_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };
    store.save(&state)?;

    Ok(ApplyResult { output, state })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::detect_target_for;
    use crate::error::Error;

    #[test]
    fn apply_unsupported_target_returns_error() {
        let target = detect_target_for("windows", "x86_64");
        let store = StateStore::new(std::env::temp_dir().join("sf-op-state.json"));
        let err = apply(&target, "/tmp/repo", &store, true).unwrap_err();
        assert!(matches!(err, Error::UnsupportedPlatform { .. }));
    }

    #[test]
    fn rollback_unsupported_target_returns_error() {
        let target = detect_target_for("windows", "x86_64");
        let store = StateStore::new(std::env::temp_dir().join("sf-op-state.json"));
        let err = rollback(&target, &store, true).unwrap_err();
        assert!(matches!(err, Error::UnsupportedPlatform { .. }));
    }
}
