//! 存储层统一错误。

use std::path::PathBuf;

/// 存储层错误。SQL 层错误直接透传（`#[from]`），迁移错误携带版本号便于定位。
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("存储打开失败 {path}: {source}")]
    Open {
        path: PathBuf,
        source: rusqlite::Error,
    },
    #[error("SQL 执行失败: {source}")]
    Sql {
        #[from]
        source: rusqlite::Error,
    },
    #[error("迁移 {from} → {to} 失败: {reason}")]
    Migrate { from: i32, to: i32, reason: String },
    #[error("SQLite 连接锁中毒（持锁线程 panic）")]
    Poisoned,
}
