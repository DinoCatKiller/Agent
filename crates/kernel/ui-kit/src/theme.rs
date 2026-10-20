//! 设计令牌 → GPUI 主题（`D4` 方案 B 第 1 条）。
//!
//! 三件事，一次做完：
//! 1. 令牌 `#rrggbb` → `Hsla`、`f32` → `Pixels`，**只在加载时解析一次**，缓存在 [`UiTheme`]；
//! 2. 把语义色推进 gpui-component 的 `Theme`（它的 60+ 组件因此自动跟随我们的配色）；
//! 3. 把我们独有的语义色（聊天侧的 user / thinking / tool 等）留在 [`ChatColors`]，给 `features/*` 用。
//!
//! **禁止每帧解析**：渲染路径只读 [`UiTheme`] 里已解析好的值。

use std::path::Path;

use gpui_kit::base::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
use gpui_kit::component::{Theme as ComponentTheme, ThemeMode};
use gpui_kit::{App, Global, Hsla, SharedString, hsla, px, rgb};
use tracing::warn;

use crate::tokens::{Colors, EMBEDDED, Tokens};

/// 令牌的解析结果 + 原始内容。
///
/// 作为 GPUI `Global` 挂载（[`UiTheme::init`]），UI 侧用 [`UiTheme::global`] 取。
/// `source` 保留原始令牌，方便热加载与调试；渲染路径请读解析好的字段。
#[derive(Debug, Clone)]
pub struct UiTheme {
    pub source: Tokens,
    /// 17 个语义槽位，直接对齐 gpui-component 的 `ColorTokens`。
    pub colors: ColorTokens,
    /// 本产品独有的语义色（聊天界面用），不属于通用槽位。
    pub chat: ChatColors,
    pub radius: RadiusTokens,
    pub spacing: SpacingTokens,
    pub typography: TypographyTokens,
}

/// 聊天界面独有的语义色（`D4`：令牌仍是唯一来源，槽位归本产品）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatColors {
    /// 用户消息
    pub user: Hsla,
    /// 思考过程
    pub thinking: Hsla,
    /// 工具调用
    pub tool: Hsla,
    /// 成功 / 完成
    pub ok: Hsla,
    /// 错误
    pub err: Hsla,
    /// 次要文字（比 `foreground` 暗）
    pub dim: Hsla,
    /// 状态栏底色 / 前景
    pub status_bg: Hsla,
    pub status_fg: Hsla,
}

impl UiTheme {
    /// 用一份令牌构造（纯函数，可单测）。
    pub fn from_tokens(source: Tokens) -> Self {
        let c = &source.color;
        let d = Colors::default();

        let colors = ColorTokens {
            background: hex_or("color.bg", &c.bg, &d.bg),
            foreground: hex_or("color.fg", &c.fg, &d.fg),
            surface: hex_or("color.surface", &c.surface, &d.surface),
            surface_foreground: hex_or("color.surfaceFg", &c.surface_fg, &d.surface_fg),
            primary: hex_or("color.primary", &c.primary, &d.primary),
            primary_foreground: hex_or("color.primaryFg", &c.primary_fg, &d.primary_fg),
            secondary: hex_or("color.secondary", &c.secondary, &d.secondary),
            secondary_foreground: hex_or("color.secondaryFg", &c.secondary_fg, &d.secondary_fg),
            muted: hex_or("color.muted", &c.muted, &d.muted),
            muted_foreground: hex_or("color.mutedFg", &c.muted_fg, &d.muted_fg),
            accent: hex_or("color.accent", &c.accent, &d.accent),
            accent_foreground: hex_or("color.accentFg", &c.accent_fg, &d.accent_fg),
            destructive: hex_or("color.destructive", &c.destructive, &d.destructive),
            destructive_foreground: hex_or(
                "color.destructiveFg",
                &c.destructive_fg,
                &d.destructive_fg,
            ),
            border: hex_or("color.border", &c.border, &d.border),
            input: hex_or("color.input", &c.input, &d.input),
            ring: hex_or("color.ring", &c.ring, &d.ring),
            // 选区是半透明罩，压在一个已有色上 —— 由 ring 派生，避免多一个令牌
            selection: hex_or("color.ring", &c.ring, &d.ring).alpha(0.3),
        };

        let chat = ChatColors {
            user: hex_or("color.user", &c.user, &d.user),
            thinking: hex_or("color.thinking", &c.thinking, &d.thinking),
            tool: hex_or("color.tool", &c.tool, &d.tool),
            ok: hex_or("color.ok", &c.ok, &d.ok),
            err: hex_or("color.err", &c.err, &d.err),
            dim: hex_or("color.dim", &c.dim, &d.dim),
            status_bg: hex_or("color.statusBg", &c.status_bg, &d.status_bg),
            status_fg: hex_or("color.statusFg", &c.status_fg, &d.status_fg),
        };

        let r = source.radius;
        let radius = RadiusTokens {
            none: px(r.none),
            sm: px(r.sm),
            md: px(r.md),
            lg: px(r.lg),
            xl: px(r.xl),
            full: px(r.full),
        };

        let s = source.spacing;
        let spacing = SpacingTokens {
            xxs: px(s.xxs),
            xs: px(s.xs),
            sm: px(s.sm),
            md: px(s.md),
            lg: px(s.lg),
            xl: px(s.xl),
            xxl: px(s.xxl),
        };

        // 字号分档暂用组件库默认，只把家族名与基准字号接上令牌
        let base = TypographyTokens::default();
        let typography = TypographyTokens {
            sans: SharedString::from(source.font.sans.clone()),
            mono: SharedString::from(source.font.mono.clone()),
            mono_md: TextStyleToken {
                size: px(source.font.mono_size),
                ..base.mono_md
            },
            ..base
        };

        Self {
            source,
            colors,
            chat,
            radius,
            spacing,
            typography,
        }
    }

    /// 读编译期内嵌的令牌；损坏则告警并回退内置兜底盘（不 panic，`R3` §2）。
    pub fn load() -> Self {
        match Tokens::parse(EMBEDDED) {
            Ok(t) => Self::from_tokens(t),
            Err(e) => {
                warn!(error = %e, "design/tokens.json 解析失败，回退内置默认盘");
                Self::from_tokens(Tokens::default())
            }
        }
    }

    /// 从磁盘读令牌（热加载用）；读不到 / 解不开同样回退内置盘。
    pub fn load_from(path: &Path) -> Self {
        match Tokens::from_path(path) {
            Ok(t) => Self::from_tokens(t),
            Err(e) => {
                warn!(path = %path.display(), error = %e, "令牌文件读取失败，回退内置默认盘");
                Self::from_tokens(Tokens::default())
            }
        }
    }

    /// 推进 gpui-component 的语义槽位（唯一需要转一手的地方）。
    pub fn semantic(&self) -> SemanticThemeTokens {
        SemanticThemeTokens {
            colors: self.colors,
            radius: self.radius,
            spacing: self.spacing,
            typography: self.typography.clone(),
            shadow: ShadowTokens::default(),
        }
    }

    /// 把令牌写进组件库主题（`Theme::change` 定深色基调 + 语义槽位 + 字体）。
    pub fn apply_to_components(&self, cx: &mut App) {
        let semantic = self.semantic();
        let font = &self.source.font;
        let font_family = SharedString::from(font.sans.clone());
        let mono_family = SharedString::from(font.mono.clone());
        let (font_size, mono_size) = (px(font.size), px(font.mono_size));

        // 本产品令牌是深色基调；不先切模式，浅色基准盘会污染派生色（如 button_*）
        ComponentTheme::change(ThemeMode::Dark, None, cx);
        ComponentTheme::update(cx, |theme| {
            theme.apply_semantic_tokens(&semantic);
            theme.font_family = font_family;
            theme.font_size = font_size;
            theme.mono_font_family = mono_family;
            theme.mono_font_size = mono_size;
        });
    }

    /// 挂全局 + 应用到组件库；UI 入口调一次即可。
    pub fn init(cx: &mut App) {
        let theme = Self::load();
        theme.apply_to_components(cx);
        cx.set_global(theme);
    }

    /// 热加载：重读文件、重挂全局、重新应用到组件库。
    ///
    /// 返回是否成功读到文件；失败时**保留旧主题**（不把界面刷成兜底盘）。
    pub fn reload_from(path: &Path, cx: &mut App) -> bool {
        let Ok(tokens) = Tokens::from_path(path) else {
            warn!(path = %path.display(), "热加载失败，保留当前主题");
            return false;
        };
        let theme = Self::from_tokens(tokens);
        theme.apply_to_components(cx);
        cx.set_global(theme);
        true
    }

    /// 取全局主题（UI 入口已 [`UiTheme::init`] 过）。
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }
}

impl Global for UiTheme {}

/// `#rrggbb` → `Hsla`；非法值告警并回退兜底色。
fn hex_or(key: &str, raw: &str, fallback: &str) -> Hsla {
    if let Some(color) = hex(raw) {
        return color;
    }
    warn!(token = key, value = raw, "非法颜色令牌，回退默认值");
    hex(fallback).unwrap_or_else(|| hsla(0., 0., 0., 1.))
}

/// 只接受 `#rrggbb`（可省 `#`）。8 位十六进制（带 alpha）暂不支持。
fn hex(s: &str) -> Option<Hsla> {
    let digits = s.trim().trim_start_matches('#');
    if digits.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(rgb(value).into())
}
