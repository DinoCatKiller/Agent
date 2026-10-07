//! `chat` 功能切片：**一次对话**的编排、状态与（将来的）界面。
//!
//! 定位（`D5` + `D6`）：一个功能 = 一个文件夹，语义（业务）住在 `features/*`。
//! 对话相关的一切住在这里，而不是散落到 transport / routing 这些「按层切」的 crate 里。
//!
//! # M4 已落地
//!
//! - [`Chat`]：会话状态机（纯数据，`A3` 运行时侧）——消息历史、累计用量、请求构造；
//! - [`ChatService::run_round`]：轮次编排，事件流输出（[`LoopEvent`]）——
//!   模型调用 → 工具执行 → 结果回填 → 循环，含取消、回滚、`max_turns` 兜底；
//! - [`ToolSet`] / [`Tool`]：工具执行边界（`A5`），宿主提供实现；
//! - [`context`]：超窗裁剪（`A7`），按交换块整块丢弃，保 tool 配对。
//!
//! # 边界
//!
//! - 依赖：`agent-common`（契约）、`agent-providers`（`Provider` / `CallContext`）。
//! - 禁止：依赖 `app`；被 `kernel/*` 依赖；**直接持有 HTTP 或 SQLite**（那是 kernel 的职责）。
//! - 允许：依赖其它 feature（如 `sessions`），但不允许成环 —— crate 依赖图天生无环。

pub mod chat;
pub mod context;
pub mod service;
pub mod tools;

pub use chat::{Chat, ChatSettings, TurnDraft};
pub use context::trim_to_fit;
pub use service::{ChatService, LoopEvent, RoundStop, ServiceConfig};
pub use tools::{Tool, ToolOutput, ToolSet};

/// 切片标识，用于日志与 `AGENTS.md` §2 地图对齐。
pub const SLICE: &str = "chat";
