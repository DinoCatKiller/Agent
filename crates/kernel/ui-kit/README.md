# agent-ui-kit · UI 组件层

| | |
|---|---|
| 状态 | ✅ 二期 GPUI 落地（2026-10-12 骨架）→ ✅ **Ely 组件库接管界面**（2026-10-21）：`ely` 重导出口 + 双资产源 + 34 个语义槽位的深色盘 |
| 边界 | **机制**：设计令牌怎么变成主题、GPUI / Ely 从哪儿进。**不含**任何业务语义（会话 / 对话 / 设置的界面归各自 `features/*/ui.rs`） |
| 上游 | `gpui-kit`（`gpui` 本体 + 窗口入口，只此一个口子）、`ely-gpui-component`（组件）、`design/tokens.json` |
| 下游 | `features/*`（界面）、`app`（启动时 `init`） |
| 何时读 | 加组件、改配色 / 圆角 / 间距 / 字号、升 GPUI 版本时 |

## 这个 crate 是什么

`D4` 方案 B 的第 1 条：**设计令牌是唯一事实来源，GPUI 只消费它**。三件事：

| 事 | 在哪 |
|----|------|
| 令牌文件 → 结构 | [`src/tokens.rs`](src/tokens.rs) |
| 结构 → 主题（`Hsla` / `Pixels`，解析一次） | [`src/theme.rs`](src/theme.rs) |
| 推进 gpui-component 的 17 个语义槽位 | [`UiTheme::apply_to_components`](src/theme.rs) |
| 令牌 → **Ely `Palette`**（34 个语义槽位，深色盘） | [`src/ely_theme.rs`](src/ely_theme.rs) |

`features/*` 与 `app` 只从本 crate 取三个口子：`kit`（GPUI 本体 + 窗口入口）、
`component`（gpui-component 的 60+ 组件）与 `ely`（**Ely 组件库**）。
**不要直接 `use gpui_kit` / `ely_gpui_component`，也不要直接写 `gpui = "0.2"`** —— 见下面第 2 条。
`ely` 是 Ely 的**唯一重导出口**：`features/*` 只依赖 `agent-ui-kit`（`D5`）。

## 核心用法

```rust
use agent_ui_kit::ely::buttons::{Button, ButtonVariant};
use agent_ui_kit::kit::*;

fn main() {
    application()
        // 两份资产必须叠成一份：Ely 的字体 / 图标 + Kit 的图标。
        // `ely::init` 会校验每个文件都能加载，漏了它直接报错。
        .with_assets(agent_ui_kit::ely::Assets::before(agent_ui_kit::kit::assets::Assets))
        .run(|cx| {
            agent_ui_kit::init(cx);            // kit → ely → 令牌主题（顺序固定，必须在开窗前）
            open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| MyView::new(window, cx))
            })
            .expect("open window");
        });
}
```

`init` 的顺序是硬约束：

1. `kit::init(cx)` —— GPUI 组件库先就位；
2. `ely::init(cx)` —— Ely 的字体 / 图标 / 主题系统（校验资产源）；
3. `UiTheme::init(cx)` → `ely_theme::apply(cx)` —— 用 `design/tokens.json` 覆盖 Ely 的 `Palette`。

`ely::init` 失败只 `tracing::error`，不 panic（`R3` §2：主题问题不该挡「跑起来」）。

## 四个必须知道的约束

1. **渲染路径不解析令牌**：`#rrggbb` → `Hsla` 只在 `load` / `reload_from` 时做一次，结果缓存在 `UiTheme`。
2. **GPUI 只有一个口子**：`gpui-kit` 会把配套的 `gpui-pre` 版本 pin 死并**重命名为 `gpui`**。自己另写 `gpui = "0.2"` 会解析到**另一个包**、类型不通用（`X2` §8 实测）。`ely-gpui-component` 钉的是同一份 `gpui-pre =0.3.8`，所以两套组件**类型互通**、可以混用。
3. **令牌损坏只告警，不 panic**：缺段 / 缺键 / 单色值非法都降级到内置兜底盘（`R3` §2）。热加载读不到文件时**保留旧主题**。
4. **业务色不在通用槽位里**：聊天侧的 user / thinking / tool / status 放在 `UiTheme::chat`，不污染 Ely / gpui-component 的语义槽位。Ely 只接管**深色盘**，`design/tokens.json` 仍是唯一事实来源。

## 它不能做什么

- ❌ 业务组件（消息气泡、会话列表条目）—— 归 `features/*/ui.rs`；能用 Ely 现成组件的就别自己造。
- ❌ `tw!` 宏。Ely 与 gpui-component 都已自带样式层，自建宏暂无必要；先用手上有的，真觉得缺再评估（记录在 `S2`）。
- ❌ 多主题切换（当前令牌是单盘深色）。要做浅色盘得先扩 `design/tokens.json` 的结构。

## 相关

- 决策 → `D4`（样式层路线）、`D3`（UI 形态）｜ 依据 → `X2`（GPUI 路线 + §8 实测）、`X4`
- 令牌文件 → [`design/tokens.json`](../../../design/tokens.json) ｜ 计划 → `GUI` §2（G-1…G-5）
- 分层判据 → `D6` §1 ｜ 地图 → [`AGENTS.md`](../../../AGENTS.md) §2
