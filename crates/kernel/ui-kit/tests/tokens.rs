//! 设计令牌的解析与映射（断网、纯函数，`Q1`）。
//!
//! 这里不碰 `App`：`UiTheme::from_tokens` / `load*` 都是纯的，
//! 需要 GPUI 上下文的 `apply_to_components` 留给后续的 `#[gpui::test]`。

use std::path::Path;

use agent_ui_kit::kit::{Hsla, SharedString, px, rgb};
use agent_ui_kit::{Tokens, UiTheme};

/// `0xrrggbb` → `Hsla`，断言里少写一层转换。
fn hsla(v: u32) -> Hsla {
    Hsla::from(rgb(v))
}

/// 换色不改结构：临时文件写一份令牌，读完删掉。
fn temp_tokens(name: &str, body: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, body).expect("写临时令牌文件");
    path
}

#[test]
fn embedded_tokens_parse_and_map() {
    let t = UiTheme::load();

    // 颜色：`#rrggbb` 精确落到语义槽位
    assert_eq!(t.colors.background, hsla(0x1b1e28));
    assert_eq!(t.colors.foreground, hsla(0xd5d9e0));
    assert_eq!(t.colors.ring, hsla(0x7aa2f7));
    assert_eq!(t.colors.destructive, hsla(0xf7768e));

    // 选区由 ring 派生：同色相 / 饱和 / 亮度，只是带 30% alpha
    let selection = t.colors.selection;
    assert_eq!(selection.h, t.colors.ring.h);
    assert_eq!(selection.s, t.colors.ring.s);
    assert_eq!(selection.l, t.colors.ring.l);
    assert!((selection.a - 0.3).abs() < 1e-6);

    // 本产品独有色：err 与 destructive 同源不同槽
    assert_eq!(t.chat.err, hsla(0xf7768e));
    assert_eq!(t.chat.status_bg, hsla(0x24283b));

    // 刻度：f32 → Pixels
    assert_eq!(t.radius.md, px(6.));
    assert_eq!(t.radius.full, px(9999.));
    assert_eq!(t.spacing.xxl, px(32.));

    // 字体接进组件库的排版令牌
    assert_eq!(t.typography.mono, SharedString::from("Consolas"));
    assert_eq!(t.typography.mono_md.size, px(13.));
}

#[test]
fn unknown_keys_are_ignored() {
    // 旧解析器（一期 TUI）只认 color，新文件多了 radius/spacing/font，反之亦然
    let json = r##"{
        "$comment": "注释键",
        "futureKey": 1,
        "color": { "bg": "#000000", "future": "#ffffff" }
    }"##;
    let t = Tokens::parse(json).expect("未知键不该导致解析失败");
    assert_eq!(t.color.bg, "#000000");
    // 缺的段走内置兜底，而不是报错
    assert_eq!(t.radius.md, Tokens::default().radius.md);
    assert_eq!(t.font.size, Tokens::default().font.size);
}

#[test]
fn broken_input_falls_back_to_default_palette() {
    // 读不到文件
    let missing = UiTheme::load_from(Path::new("i:/Agent/tmp/__no_such_tokens__.json"));
    assert_eq!(missing.colors.background, hsla(0x1b1e28));

    // 文件存在但不是合法 JSON
    let bad = temp_tokens("ui-kit-bad-tokens.json", "{ this is not json");
    let theme = UiTheme::load_from(&bad);
    assert_eq!(theme.colors.background, hsla(0x1b1e28));
    let _ = std::fs::remove_file(&bad);
}

#[test]
fn illegal_color_falls_back_per_slot() {
    // 单个色值非法 → 只那一槽回退默认，其余照常生效
    let json = r##"{ "color": { "bg": "#zzzzzz", "fg": "#123456" } }"##;
    let t = Tokens::parse(json).expect("解析");
    let theme = UiTheme::from_tokens(t);
    assert_eq!(theme.colors.background, hsla(0x1b1e28));
    assert_eq!(theme.colors.foreground, hsla(0x123456));
}

#[test]
fn hot_reload_reads_file() {
    let path = temp_tokens(
        "ui-kit-reload-tokens.json",
        r##"{ "color": { "bg": "#010203" }, "radius": { "md": 9 } }"##,
    );
    let theme = UiTheme::load_from(&path);
    assert_eq!(theme.colors.background, hsla(0x010203));
    assert_eq!(theme.radius.md, px(9.));
    // 未覆盖的槽位仍是兜底盘
    assert_eq!(theme.source.color.border, Tokens::default().color.border);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn semantic_tokens_mirror_parsed_values() {
    let theme = UiTheme::load();
    let semantic = theme.semantic();
    assert_eq!(semantic.colors, theme.colors);
    assert_eq!(semantic.radius, theme.radius);
    assert_eq!(semantic.spacing, theme.spacing);
}
