//! 设计令牌的读取与解析（`D4` 方案 B 第 1 条：`design/tokens.json` 是跨语言唯一事实来源）。
//!
//! 本模块只做「文件 → 结构」；把结构转成 GPUI 能用的类型在 [`crate::theme`]。
//! 颜色一律是 `#rrggbb` 字符串，**不在渲染路径上解析**。

use std::path::Path;

use serde::Deserialize;

/// 编译期内嵌的令牌（缺省来源）。改 `design/tokens.json` 需重新编译才生效。
pub const EMBEDDED: &str = include_str!("../../../../design/tokens.json");

/// 令牌文件顶层结构，与 `design/tokens.json` 一一对应。
///
/// `#[serde(default)]` + 各段自带的 [`Default`] 让「文件缺段 / 缺键」降级为内置兜底盘，
/// 而不是解析失败（未知键一律忽略，便于旧解析器与新文件共存）。
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct Tokens {
    pub color: Colors,
    pub radius: Radius,
    pub spacing: Spacing,
    pub font: Font,
}

impl Tokens {
    /// 解析令牌文本。失败只报错，不 panic（调用方决定兜底）。
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 从磁盘读一条令牌链（热加载用）。
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

// `Tokens::default()` 由各段自带的 `Default` 组合而成（derive），
// 各段的默认值与 `design/tokens.json` 保持一致：文件损坏时观感不变。

/// 配色令牌（键为 camelCase，值为 `#rrggbb`）。
///
/// 前 13 个是一期 TUI 就在用的；`surface` 起是二期 GPUI 补的语义槽位
/// （对齐 gpui-component 的 `ColorTokens`，见 [`crate::theme::UiTheme::semantic`]）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Colors {
    pub bg: String,
    pub fg: String,
    pub dim: String,
    pub accent: String,
    pub border: String,
    pub border_focus: String,
    pub user: String,
    pub thinking: String,
    pub tool: String,
    pub ok: String,
    pub err: String,
    pub status_bg: String,
    pub status_fg: String,

    pub surface: String,
    pub surface_fg: String,
    pub muted: String,
    pub muted_fg: String,
    pub primary: String,
    pub primary_fg: String,
    pub secondary: String,
    pub secondary_fg: String,
    pub accent_fg: String,
    pub destructive: String,
    pub destructive_fg: String,
    pub input: String,
    pub ring: String,
}

impl Default for Colors {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            bg: s("#1b1e28"),
            fg: s("#d5d9e0"),
            dim: s("#6b7280"),
            accent: s("#7aa2f7"),
            border: s("#3b4252"),
            border_focus: s("#7aa2f7"),
            user: s("#9ece6a"),
            thinking: s("#bb9af7"),
            tool: s("#e0af68"),
            ok: s("#9ece6a"),
            err: s("#f7768e"),
            status_bg: s("#24283b"),
            status_fg: s("#a9b1d6"),
            surface: s("#24283b"),
            surface_fg: s("#d5d9e0"),
            muted: s("#24283b"),
            muted_fg: s("#6b7280"),
            primary: s("#7aa2f7"),
            primary_fg: s("#1b1e28"),
            secondary: s("#2f3549"),
            secondary_fg: s("#d5d9e0"),
            accent_fg: s("#1b1e28"),
            destructive: s("#f7768e"),
            destructive_fg: s("#1b1e28"),
            input: s("#3b4252"),
            ring: s("#7aa2f7"),
        }
    }
}

/// 圆角刻度（逻辑像素，无单位）。
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct Radius {
    pub none: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub full: f32,
}

impl Default for Radius {
    fn default() -> Self {
        Self {
            none: 0.,
            sm: 3.,
            md: 6.,
            lg: 8.,
            xl: 12.,
            full: 9999.,
        }
    }
}

/// 间距刻度（逻辑像素，无单位）。
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct Spacing {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xxs: 2.,
            xs: 4.,
            sm: 8.,
            md: 12.,
            lg: 16.,
            xl: 24.,
            xxl: 32.,
        }
    }
}

/// 字体：家族名与基准字号（`.SystemUIFont` 是 GPUI 的系统字体占位名）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Font {
    pub sans: String,
    pub mono: String,
    pub size: f32,
    pub mono_size: f32,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            sans: ".SystemUIFont".to_string(),
            mono: if cfg!(target_os = "windows") {
                "Consolas".to_string()
            } else if cfg!(target_os = "macos") {
                "Menlo".to_string()
            } else {
                "DejaVu Sans Mono".to_string()
            },
            size: 14.,
            mono_size: 13.,
        }
    }
}
