//! agent-ui-kit · UI 组件层（二期 GPUI，`D4` 方案 B）
//!
//! 边界：**机制**。设计令牌 → GPUI 主题的桥、组件库的统一入口。
//! 不含任何业务语义（会话、对话、设置这些归 `features/*`）。
//!
//! 两个口子，`features/*` 与 `app` 只用这两个：
//! - [`kit`]：GPUI 本体 + gpui-kit 的入口（`open_window` / `application` / `init`）
//! - [`component`]：60+ 现成组件（Button / Input / Table / Markdown …）
//!
//! 设计令牌的唯一事实来源是仓库根的 `design/tokens.json`；本 crate 只读它，
//! 解析一次后缓存在 [`UiTheme`]，渲染路径不做解析。

pub mod theme;
pub mod tokens;

pub use theme::{ChatColors, UiTheme};
pub use tokens::Tokens;

/// GPUI 入口（gpui-kit）。它的 `*` 里带 `gpui` 的重导出，所以
/// `use agent_ui_kit::kit::*;` 一次拿到 `div` / `Render` / `Window` 这些。
pub use gpui_kit as kit;
/// 行为基座：`gpui-base`（含 `SemanticThemeTokens` 等令牌类型）。
pub use gpui_kit::base;
/// 组件库：`gpui-component` 的 60+ 组件。
pub use gpui_kit::component;

/// UI 入口初始化：先起组件库（gpui-kit 要求 `init` 先于一切），
/// 再把设计令牌推进 gpui-component 的主题，最后挂成全局。
///
/// 必须在 [`kit::open_window`] 之前调用。
pub fn init(cx: &mut kit::App) {
    kit::init(cx);
    UiTheme::init(cx);
}
