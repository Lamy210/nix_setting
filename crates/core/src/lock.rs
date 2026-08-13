use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::error::{Error, Result};
use crate::state::State;

/// 操作の直列化に使うクロスプロセス・ロック
///
/// mutating 操作 (apply/rollback/upgrade 等) は同一マシン上で同時実行されない。
/// ロックファイルに対する `flock` (排他ロック) を使うため、CLI (別 terminal) と GUI の
/// ように異なるプロセス間でも直列化される。`global()` が既定パスの singleton を返し、
/// テストでは `OperationLock::new(path)` で独立に検証できる。
#[derive(Debug)]
pub struct OperationLock {
    path: PathBuf,
}

impl Default for OperationLock {
    fn default() -> Self {
        Self::new(State::default_path().with_file_name("operation.lock"))
    }
}

impl OperationLock {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn global() -> &'static OperationLock {
        static GLOBAL: OnceLock<OperationLock> = OnceLock::new();
        GLOBAL.get_or_init(OperationLock::default)
    }

    /// ロックを取得する (取得できるまでブロック)
    pub fn acquire(&self) -> Result<OperationGuard> {
        let file = open_lock_file(&self.path)?;
        file.lock()
            .map_err(|e| Error::Io(format!("lock {}: {e}", self.path.display())))?;
        Ok(OperationGuard { _file: file })
    }

    /// ロックを取得する (既に取得済みなら Ok(None) を返す)
    pub fn try_acquire(&self) -> Result<Option<OperationGuard>> {
        let file = open_lock_file(&self.path)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(OperationGuard { _file: file })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(e)) => {
                Err(Error::Io(format!("try_lock {}: {e}", self.path.display())))
            }
        }
    }
}

fn open_lock_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::Io(format!("create_dir: {e}")))?;
    }
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| Error::Io(format!("open {}: {e}", path.display())))
}

/// ロック保持中のみ生存するガード。drop で flock 解放
pub struct OperationGuard {
    _file: File,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_lock(name: &str) -> OperationLock {
        let dir = std::env::temp_dir().join(format!("schneeforge-lock-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        OperationLock::new(dir.join("op.lock"))
    }

    #[test]
    fn try_acquire_fails_while_held() {
        let lock = temp_lock("held");
        let _guard = lock.try_acquire().unwrap().expect("first acquire");
        assert!(lock.try_acquire().unwrap().is_none());
    }

    #[test]
    fn try_acquire_succeeds_after_release() {
        let lock = temp_lock("release");
        {
            let _guard = lock.try_acquire().unwrap().expect("first acquire");
        }
        assert!(lock.try_acquire().unwrap().is_some());
    }

    #[test]
    fn locks_are_independent() {
        let a = temp_lock("indep-a");
        let b = temp_lock("indep-b");
        let _ga = a.try_acquire().unwrap().expect("lock a");
        assert!(b.try_acquire().unwrap().is_some());
    }
}
