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

pub mod ely_theme;
pub mod theme;
pub mod tokens;

pub use theme::{ChatColors, UiTheme};
pub use tokens::Tokens;

/// 界面组件库（`D4`）：Ely GPUI Component。`assets` / 主题 / 各组件都在这里面。
///
/// `features/*` 统一从 `agent_ui_kit::ely::…` 取，不直接写依赖名 —— `D6` 的
/// 「界面只依赖组件层」因此仍然成立。
pub use ely_gpui_component as ely;
/// GPUI 入口（gpui-kit）。它的 `*` 里带 `gpui` 的重导出，所以
/// `use agent_ui_kit::kit::*;` 一次拿到 `div` / `Render` / `Window` 这些。
pub use gpui_kit as kit;
/// 行为基座：`gpui-base`（含 `SemanticThemeTokens` 等令牌类型）。
pub use gpui_kit::base;
/// 组件库：gpui-component 的 60+ 组件。
pub use gpui_kit::component;

/// UI 入口初始化：三个库按依赖顺序起，最后把令牌推进两套主题。
///
/// 必须在 [`kit::open_window`] 之前调用。资产源要在 `application()` 上设成
/// `ely::Assets::before(kit::assets::Assets)`（GPUI 只接受一个资产源）。
pub fn init(cx: &mut kit::App) {
    kit::init(cx);
    // Ely 注册自己的字体与主题（它先建全局 Theme，我们的调色板随后覆盖）。
    if let Err(error) = ely::init(cx) {
        // 初始化失败只影响 Ely 的字体/资产，不该把窗口挡在门外（`R3` §2 的降级口径）。
        tracing::error!(%error, "Ely 组件库初始化失败，界面可能缺少字体或图标");
    }
    UiTheme::init(cx);
    ely_theme::apply(cx);
}
