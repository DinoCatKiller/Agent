//! 本地存储：SQLite 连接管理、版本化迁移、事务原语（对应 M5）。
//!
//! 职责边界（`D6` §1）：
//! - 只做**机制**：连接生命周期、PRAGMA、版本化迁移执行器、事务 guard；
//!   表结构 / 会话 CRUD 是业务语义，归 `features/chat` 的 `repo.rs`。
//! - 单机桌面：单连接（`Arc<Mutex<Connection>>`），无连接池（`X1` §1）。
//! - 同步 API：rusqlite 是同步的；async 调用方自行决定 `spawn_blocking`。
//!
//! 模块：
//! - [`error`] 统一错误 `StoreError`
//! - [`store`] `Store`：打开 / 内存库 / 迁移入口 / 事务入口 / 便捷读写
//! - [`migrate`] `Migrator`：版本列表 + `PRAGMA user_version` 执行器
//! - [`tx`] `Tx`：事务 guard（commit / rollback / Drop 兜底回滚）

pub mod error;
pub mod migrate;
pub mod store;
pub mod tx;

pub use error::StoreError;
pub use migrate::Migrator;
pub use store::Store;
pub use tx::Tx;
