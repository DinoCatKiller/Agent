//! `Store`：SQLite 连接管理（打开 / PRAGMA / 迁移 / 事务入口）。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite::types::FromSql;

use crate::StoreError;
use crate::migrate::Migrator;
use crate::tx::Tx;

/// 本地存储：单连接（`Arc<Mutex<Connection>>`），同步 API，无连接池（`X1` §1）。
/// `Clone` 只增引用计数，不复制连接。
#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Store {
    /// 打开（或创建）指定路径的数据库，并应用基础 PRAGMA。
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let path = path.into();
        let conn = Connection::open(&path).map_err(|source| StoreError::Open {
            path: path.clone(),
            source,
        })?;
        Self::init(conn, path)
    }

    /// 打开内存库（测试用，连接关闭即销毁）。
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?, PathBuf::from(":memory:"))
    }

    fn init(conn: Connection, path: PathBuf) -> Result<Self, StoreError> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 执行版本化迁移到 `Migrator` 的最新版本（重复调用是幂等的）。
    pub fn migrate(&self, migrator: &Migrator) -> Result<(), StoreError> {
        let mut conn = self.lock()?;
        migrator.apply(&mut conn)
    }

    /// 开启一个事务（持锁直到 commit / rollback / Drop）。
    pub fn tx(&self) -> Result<Tx<'_>, StoreError> {
        Tx::begin(self.lock()?)
    }

    /// 便捷写语句（业务查询走 `features/chat` 的 `repo.rs`）。
    pub fn execute(&self, sql: &str) -> Result<usize, StoreError> {
        Ok(self.lock()?.execute(sql, [])?)
    }

    /// 便捷查单值。
    pub fn query_single<T>(&self, sql: &str) -> Result<T, StoreError>
    where
        T: FromSql,
    {
        Ok(self.lock()?.query_row(sql, [], |row| row.get(0))?)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.conn.lock().map_err(|_| StoreError::Poisoned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_version(store: &Store) -> i32 {
        store.query_single("PRAGMA user_version").unwrap()
    }

    #[test]
    fn migrate_applies_in_order_and_is_idempotent() {
        let store = Store::open_in_memory().unwrap();
        let mut m = Migrator::new();
        m.add(1, &["CREATE TABLE t1 (id INTEGER PRIMARY KEY);"]);
        m.add(2, &["CREATE TABLE t2 (id INTEGER PRIMARY KEY);"]);
        store.migrate(&m).unwrap();
        assert_eq!(user_version(&store), 2);
        store.migrate(&m).unwrap();
        assert_eq!(user_version(&store), 2);
    }

    #[test]
    fn failed_migration_rolls_back_version() {
        let store = Store::open_in_memory().unwrap();
        let mut m = Migrator::new();
        m.add(1, &["CREATE TABLE t1 (id INTEGER PRIMARY KEY);"]);
        m.add(2, &["CREATE TABLE t1 (id INTEGER PRIMARY KEY);"]); // 重名 → 失败
        assert!(store.migrate(&m).is_err());
        assert_eq!(user_version(&store), 1);
    }

    #[test]
    fn tx_commit_persists_rollback_discards() {
        let store = Store::open_in_memory().unwrap();
        store
            .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
            .unwrap();

        let mut tx = store.tx().unwrap();
        tx.conn()
            .execute_batch("INSERT INTO t (id) VALUES (1);")
            .unwrap();
        tx.commit().unwrap();

        let mut tx = store.tx().unwrap();
        tx.conn()
            .execute_batch("INSERT INTO t (id) VALUES (2);")
            .unwrap();
        tx.rollback().unwrap();

        let count: i64 = store.query_single("SELECT COUNT(*) FROM t").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn drop_without_commit_rolls_back() {
        let store = Store::open_in_memory().unwrap();
        store
            .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
            .unwrap();

        let mut tx = store.tx().unwrap();
        tx.conn()
            .execute_batch("INSERT INTO t (id) VALUES (1);")
            .unwrap();
        drop(tx); // 未 commit → 自动回滚

        let count: i64 = store.query_single("SELECT COUNT(*) FROM t").unwrap();
        assert_eq!(count, 0);
    }
}
