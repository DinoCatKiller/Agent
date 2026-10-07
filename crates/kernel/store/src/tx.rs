//! 事务 guard：commit / rollback / Drop 兜底回滚。

use std::sync::MutexGuard;

use rusqlite::Connection;

use crate::StoreError;

/// 一个进行中的事务，持锁直到结束。
/// 未显式 `commit` / `rollback` 即被 Drop 时，自动回滚（防悬挂事务）。
pub struct Tx<'a> {
    guard: MutexGuard<'a, Connection>,
    finished: bool,
}

impl<'a> Tx<'a> {
    pub(super) fn begin(guard: MutexGuard<'a, Connection>) -> Result<Self, StoreError> {
        guard.execute_batch("BEGIN")?;
        Ok(Self {
            guard,
            finished: false,
        })
    }

    /// 事务内的连接（可执行任意 SQL）。
    pub fn conn(&mut self) -> &mut Connection {
        &mut self.guard
    }

    pub fn commit(mut self) -> Result<(), StoreError> {
        self.finish("COMMIT")?;
        Ok(())
    }

    pub fn rollback(mut self) -> Result<(), StoreError> {
        self.finish("ROLLBACK")?;
        Ok(())
    }

    fn finish(&mut self, sql: &str) -> Result<(), StoreError> {
        self.guard.execute_batch(sql)?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for Tx<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.guard.execute_batch("ROLLBACK");
        }
    }
}
