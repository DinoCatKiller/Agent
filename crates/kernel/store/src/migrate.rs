//! 版本化迁移：`PRAGMA user_version` 记录当前版本，逐版本在事务内执行。

use rusqlite::Connection;

use crate::StoreError;

/// 迁移列表。每个条目 = 目标版本号 + 该版本的 SQL 脚本（按序执行）。
/// 版本号从 1 起严格递增；`apply` 从 `user_version + 1` 执行到最新版。
#[derive(Debug, Default)]
pub struct Migrator {
    versions: Vec<(i32, Vec<&'static str>)>,
}

impl Migrator {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个版本的迁移脚本。
    pub fn add(&mut self, version: i32, sql: &[&'static str]) {
        debug_assert!(self.versions.last().is_none_or(|(v, _)| *v < version));
        self.versions.push((version, sql.to_vec()));
    }

    pub fn target(&self) -> i32 {
        self.versions.last().map_or(0, |(v, _)| *v)
    }

    /// 把连接迁移到最新版本。每版本一个事务：失败回滚，`user_version` 保持原值。
    pub fn apply(&self, conn: &mut Connection) -> Result<(), StoreError> {
        let mut current = current_version(conn)?;
        for (version, scripts) in &self.versions {
            if *version <= current {
                continue;
            }
            let tx = conn.transaction()?;
            for script in scripts {
                tx.execute_batch(script)
                    .map_err(|source| StoreError::Migrate {
                        from: current,
                        to: *version,
                        reason: source.to_string(),
                    })?;
            }
            tx.pragma_update(None, "user_version", *version)
                .map_err(|source| StoreError::Migrate {
                    from: current,
                    to: *version,
                    reason: source.to_string(),
                })?;
            tx.commit()?;
            current = *version;
        }
        Ok(())
    }
}

fn current_version(conn: &Connection) -> Result<i32, StoreError> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}
