# agent-store · 本地存储

| | |
|---|---|
| 状态 | ✅ M5 落地：SQLite 连接 / 迁移 / 事务（rusqlite bundled） |
| 边界 | **机制**：连接生命周期、版本化迁移执行器、事务原语。**不含**业务表结构与会话 CRUD（归 `features/chat` 的 `repo.rs`，`D6` §1） |
| 上游 | —（纯机制，不依赖任何 crate） |
| 下游 | `features/chat`（`repo.rs`）、一期 TUI |
| 何时读 | 动持久化 / 建表 / 事务前 |

## 这个 crate 是什么

单机桌面的本地持久化基座：一个 `Store` 持有一条 SQLite 连接
（`Arc<Mutex<Connection>>`，无连接池，`X1` §1），提供三类原语：

| 项 | 说明 |
|----|------|
| 连接管理 ✅ | `open` / `open_in_memory`，初始化 `foreign_keys`、`WAL` PRAGMA；`Clone` 只增引用计数 |
| 版本化迁移 ✅ | `Migrator`：版本号 + SQL 脚本列表，`PRAGMA user_version` 记录进度；每版本一个事务，失败回滚 |
| 事务 ✅ | `Tx` guard：`commit` / `rollback`，Drop 未完成自动回滚（防悬挂事务） |

## 三个必须知道的约束

1. **只做机制，不做语义**：表结构由 `repo.rs` 定义并通过 `Migrator` 传入，store 不知道任何业务表。
2. **同步 API**：rusqlite 是同步的；async 调用方（`repo.rs`）自行决定 `spawn_blocking`。
3. **迁移幂等**：重复 `apply` 是空操作；单版本失败整体回滚，`user_version` 不前进。

## 相关

- 选型 → `X1` §1（rusqlite bundled）｜ 机制与语义判据 → `D6` ｜ 测试 → `Q1`
- 会话持久化（业务侧）→ `features/chat` 的 `repo.rs`（`S2` Q8 已闭环）
- 概览 → `S1` ｜ 地图 → [`AGENTS.md`](../../../AGENTS.md) §2