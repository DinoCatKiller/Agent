//! `settings` 功能切片：**设置**（默认模型、Base URL、代理、超时）与密钥录入。
//!
//! 定位（`D5` + `D6`）：一个功能 = 一个文件夹，语义（业务）住在 `features/*`。
//!
//! # 现状：A 语义落地（2026-10-12）
//!
//! - [`settings`]：设置项的业务模型与校验（[`Draft`] → [`Settings`]）；
//! - [`service`]：读写编排（[`SettingsService`]）+ 密钥后端抽象（[`SecretBackend`]）；
//! - [`ui`]：GPUI 设置面板（**仅 `gui` feature**）。
//!
//! **生效时机 = 下次启动**（`S1` 切片 5 拍板）：本切片只写盘，不做运行中热切换。
//! 「保存后立刻生效」的 B 方案是给 `app` 留的接口，尚未实现。
//!
//! # 边界
//!
//! - 依赖：`agent-config`（**机制**：文件与钥匙串的读写）。
//! - 禁止：依赖 `app`；被 `kernel/*` 依赖；**直接读写文件或钥匙串**
//!   （不走 `agent_config::load/save` 会绕过 `deny_unknown_fields` 与「错误文案不回显原文」的保护）。
//! - 密钥：明文只经 [`SecretString`](agent_config::SecretString) 承载，任何情况下**不得**
//!   写进日志、错误或状态快照，也不进配置文件（`R1` / 硬约束 5）。

pub mod service;
pub mod settings;
#[cfg(feature = "gui")]
pub mod ui;

pub use service::{KeySource, KeyringBackend, MemoryBackend, SecretBackend, SettingsService};
pub use settings::{Draft, FieldError, Settings};

/// 切片标识，用于日志与 `AGENTS.md` §2 地图对齐。
pub const SLICE: &str = "settings";
