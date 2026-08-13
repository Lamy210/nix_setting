use std::sync::{Mutex, OnceLock};

/// 操作の直列化に使う process-wide ロック
///
/// mutating 操作 (apply/rollback/upgrade 等) は同一プロセス内で同時実行されない。
/// `global()` がプロセス共通の singleton を返し、テストでは `OperationLock::new()` を
/// 使って独立に検証できる。
#[derive(Debug, Default)]
pub struct OperationLock {
    inner: Mutex<()>,
}

impl OperationLock {
    pub fn new() -> Self {
        Self::default()
    }

    /// ロックを取得する (取得できるまでブロック)
    pub fn acquire(&self) -> OperationGuard<'_> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        OperationGuard { _guard: guard }
    }

    /// ロックを取得する (既に取得済みなら None を返す)
    pub fn try_acquire(&self) -> Option<OperationGuard<'_>> {
        self.inner
            .try_lock()
            .ok()
            .map(|guard| OperationGuard { _guard: guard })
    }

    /// プロセス共通の singleton を返す
    pub fn global() -> &'static OperationLock {
        static GLOBAL: OnceLock<OperationLock> = OnceLock::new();
        GLOBAL.get_or_init(OperationLock::new)
    }
}

/// ロック保持中のみ生存するガード。drop でロック解放
pub struct OperationGuard<'a> {
    _guard: std::sync::MutexGuard<'a, ()>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_acquire_fails_while_held() {
        let lock = OperationLock::new();
        let _guard = lock.acquire();
        assert!(lock.try_acquire().is_none());
    }

    #[test]
    fn try_acquire_succeeds_after_release() {
        let lock = OperationLock::new();
        {
            let _guard = lock.acquire();
        }
        assert!(lock.try_acquire().is_some());
    }

    #[test]
    fn locks_are_independent() {
        let a = OperationLock::new();
        let b = OperationLock::new();
        let _ga = a.acquire();
        // 別インスタンスのロックは影響を受けない
        assert!(b.try_acquire().is_some());
    }
}
