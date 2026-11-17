//! 设计令牌 → **Ely** 的语义调色板（`D4`：令牌仍是唯一事实来源）。
//!
//! Ely 的组件只读它自己的 `theme::Palette`，不接受任意颜色注入；官方的口子是
//! [`ely::theme::Theme::set_palette`] —— 传入一份「owner palette」后，切到该模式下
//! 就用它替换 Ely 的内置盘。本模块把 `design/tokens.json` 映射成这份 palette。
//!
//! 两条约定：
//! - **以 Ely 的盘为底**（`Palette::dark`），只覆盖令牌声明过的槽位。这样我们没有的
//!   槽位（`chart` / `ansi` / `syntax`）保持组件库默认，不会因为缺键变成黑色。
//! - **只接管深色盘**：`design/tokens.json` 目前只有一盘深色。浅色仍用 Ely 内置的暖色盘，
//!   不假装我们有一套没设计过的浅色令牌。

use ely_gpui_component::theme::{Mode, Palette, Theme};
use gpui_kit::App;

use crate::theme::{UiTheme, hex};

/// 令牌 → Ely 调色板（纯函数，可单测）。
///
/// 未声明的槽位保持 `base` 的值：调用方给 `Palette::dark(false)` 即「Ely 深色盘做底」。
pub fn palette_from_tokens(tokens: &crate::tokens::Tokens, base: Palette) -> Palette {
    let c = &tokens.color;
    let mut palette = base;

    // 每次覆盖都走「非法值 → 保留底色」，与 `UiTheme` 的降级口径一致（`R3` §2）。
    macro_rules! set {
        ($field:ident, $value:expr) => {
            if let Some(color) = hex(&$value) {
                palette.$field = color;
            }
        };
    }

    // 页面与容器：靠色阶分层，不靠分隔线。
    set!(bg, c.bg);
    set!(surface, c.surface);
    set!(sunken, c.muted);
    set!(overlay, c.surface);
    set!(hover, c.secondary);
    set!(active, c.secondary);
    set!(border, c.border);
    set!(border_strong, c.input);

    // 文字：正文 / 次要 / 更弱 / 禁用四档。
    set!(fg, c.fg);
    set!(fg_muted, c.muted_fg);
    set!(fg_subtle, c.dim);
    set!(fg_disabled, c.dim);

    // 强调色与状态色。
    set!(accent, c.accent);
    set!(accent_hover, c.primary);
    set!(on_accent, c.accent_fg);
    set!(on_media, c.surface_fg);
    set!(focus, c.ring);
    set!(link, c.accent);
    set!(success, c.ok);
    set!(warning, c.tool);
    set!(danger, c.err);
    set!(info, c.accent);

    // 状态色的淡彩底（Ely 用它画提示条 / 状态签）。
    if let Some(color) = hex(&c.ok) {
        palette.success_subtle = color.alpha(0.15);
    }
    if let Some(color) = hex(&c.tool) {
        palette.warning_subtle = color.alpha(0.15);
    }
    if let Some(color) = hex(&c.err) {
        palette.danger_subtle = color.alpha(0.15);
    }
    if let Some(color) = hex(&c.accent) {
        palette.info_subtle = color.alpha(0.15);
        palette.selection = color.alpha(0.30);
    }

    // 浮层与阴影。
    if let Some(color) = hex(&c.bg) {
        palette.shadow = color.alpha(0.45);
        palette.backdrop = color.alpha(0.55);
        palette.media_backdrop = color.alpha(0.55);
    }
    if let Some(color) = hex(&c.surface) {
        palette.glass = color.alpha(0.80);
        palette.shimmer = color;
    }
    set!(tooltip_bg, c.status_bg);
    set!(tooltip_fg, c.status_fg);
    set!(paper, c.bg);
    set!(ink, c.fg);

    palette
}

/// 把令牌推进 Ely：模式固定深色 + 注入调色板 + 字体族。
///
/// 必须在 [`ely`](crate::ely) 的 `init` 之后调用（它先建全局 `Theme`）。
pub fn apply(cx: &mut App) {
    let tokens = UiTheme::global(cx).source.clone();
    let font = &tokens.font;

    // 浅色盘不接管：只把深色盘换成我们这套（`design/tokens.json` 只有深色）。
    Theme::set_mode(Mode::Dark, cx);
    Theme::set_palette(
        Mode::Dark,
        Some(palette_from_tokens(&tokens, Palette::dark(false))),
        cx,
    );
    Theme::update(cx, |theme| {
        theme.font_family = font.sans.clone().into();
        theme.mono_family = font.mono.clone().into();
    });
}
