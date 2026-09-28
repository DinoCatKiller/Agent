//! `chat` 功能切片：**一次对话**的编排、状态与（将来的）界面。
//!
//! 定位（`D5`）：一个功能 = 一个文件夹。对话相关的一切住在这里，
//! 而不是散落到 `transport / core / ui` 这些"按层切"的 crate 里。
//!
//! # 现状：骨架
//!
//! 刻意**还没有实现**：`A2` 契约与 `D3`（UI 形态）稳定前写下的接口大概率会被推翻。
//! 第一个任务见 `crates/features/chat/README.md` 的「待办」。
//!
//! # 边界
//!
//! - 依赖：`agent-common`（契约）、`infra/*`（传输 / 供应商 / 路由）。
//! - 禁止：依赖 `app`；被 `infra/*` 依赖；**直接持有 HTTP 或 SQLite**（那是 infra 的职责）。
//! - 允许：依赖其它 feature（如 `sessions`），但不允许成环 —— crate 依赖图天生无环。

/// 切片标识，用于日志与 `AGENTS.md` §2 地图对齐。
pub const SLICE: &str = "chat";
