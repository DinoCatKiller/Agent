//! `sessions` 功能切片：**会话**的创建、列举、重命名与切换。
//!
//! 定位（`D5` + `D6`）：一个功能 = 一个文件夹，语义（业务）住在 `features/*`。
//!
//! # M5 已落地
//!
//! - [`session`]：会话语义规则——标题自动生成（`chat` 落盘时调用）；
//! - [`ui`]：会话侧栏——列表与选择（一期 TUI 的左栏）。
//!
//! 注意职责划分：**持久化的实现在 `features/chat` 的 `repo.rs`**（`S2` Q8 闭环：
//! 会话表随 chat 切片），本切片不直接写 SQL，也不持有连接；元类型
//! [`agent_chat::SessionMeta`] 也住在那边——本切片消费它渲染列表。
//!
//! # 边界
//!
//! - 依赖：`agent-common`、`agent-chat`（feature 间依赖，`D5`）、`ratatui`（仅 `ui`）。
//! - 禁止：依赖 `app`；被 `kernel/*` 依赖；直接持有数据库连接。

pub mod session;
pub mod ui;

pub use session::derive_title;

/// 切片标识，用于日志与 `AGENTS.md` §2 地图对齐。
pub const SLICE: &str = "sessions";
