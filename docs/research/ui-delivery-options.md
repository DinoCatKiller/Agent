# X4 · UI 交付形态调研（GPUI / TUI / WebView）

> 索引 ID：`X4` ｜ 状态：✅ 调研完成 ｜ 最后更新：2026-09-28

## 目的

回答两个问题：**TUI 一般用什么做？GPUI 能不能做 TUI？** 并顺带澄清「GPUI 样式少」这个判断是否成立。

## 1. 直接回答：GPUI 做不了 TUI

GPUI 的渲染目标是**窗口像素**（`blade-graphics` → Metal / Vulkan / DirectX / WebGL），文本走 DirectWrite（Windows）/ cosmic-text（Linux）/ font-kit（macOS）做字形栅格化，**不产出 ANSI 转义序列，也没有「终端后端」**。

所以 GPUI 只有两种「看起来像终端」的做法，都不是 TUI：

| 做法 | 实质 |
|------|------|
| 在 GPUI 窗口里嵌一个终端模拟器（Zed 用 `alacritty_terminal`） | 是「GUI 里的终端控件」，输出到窗口，不是终端里的 TUI |
| GPUI 的 headless / `test-support` 模式 | 只用于测试断言，不面向终端输出 |

**结论：TUI 与 GPUI 是互斥的交付形态。** 想输出到终端，就得换栈（Ratatui / Ink / OpenTUI）；想用 GPUI，交付物就是 GUI 窗口。

## 2. TUI 一般用什么做（2026-09 现状）

| 语言 | 框架 | 现状 | 代表产品 |
|------|------|------|---------|
| Rust | **`ratatui`** + `crossterm` | 事实标准（22.7k★，`tui-rs` 停更后社区接手）；2026 已模块化为 `ratatui-core` / `ratatui-widgets` / `ratatui-crossterm` / `ratatui-termion` / `ratatui-termwiz`，渲染核心与后端解耦；图片走 `ratatui-image`（Sixel / Kitty / iTerm2） | **Codex CLI** |
| TypeScript | **Ink v7 + React 19** | DX 优先、抽象层级高（写 JSX，框架管协调/布局/渲染）；生态成熟 | **Claude Code** |
| TypeScript | **OpenTUI**（Zig 核心 + TS/React/Solid 绑定） | `@opentui/solid` 0.5.8；官方仓库仍标注「in development」；Yoga 布局引擎适配终端 | **OpenCode / Gemini CLI** |
| Go | `bubbletea`（+ lipgloss） | 成熟，生态好 | 若干 CLI |
| Python | `Textual` / `prompt-toolkit` | 成熟，适合内部工具 | 若干 CLI |

值得注意的行业信号：**AI 客户端几乎全选 TUI** —— Claude Code（Ink）、Codex CLI（Ratatui）、Gemini CLI 与 OpenCode（OpenTUI）。TUI 在这一品类已经是主流形态，不是「低配版 GUI」。

另一个业界共识（来自多份 CLI 工具社区周报）：**TUI 的流式输出渲染是高频痛点**（长文本增量、滚动、复制、多语言宽度），无论用哪个框架都要自己处理节流与重排。

## 3. 「GPUI 样式少」这个判断，对一半

| 维度 | 事实 |
|------|------|
| 表达能力 | **不弱**。类 Tailwind 方法链、Flexbox（与 Web 语义一致）、编译期类型校验（传错单位直接编译失败）、语义主题 + 多尺寸；`gpui-kit` 提供 60+ 组件、Markdown/HTML 渲染、Tree-sitter 编辑器、Dock、虚拟表格 |
| 现成资源 | **少**。没有 Tailwind 生态、没有 shadcn 那种「复制粘贴即用」的组件洪流、没有主题市场、没有 Figma 联动 |
| 人力供给 | **少**。会 React/TS 的人远多于会 GPUI 的人 |
| 第三方库 | **少**。图表、动效、富文本编辑器等都要自己写或找不到现成的 |

→ 所以「传统桌面样式更适合 TS」在**资源、人力、生态**层面成立；在**渲染与表达能力**层面不成立。这是选型的关键区分：你缺的是**素材库和人**，不是渲染能力。

## 4. 三条交付形态对比

| 维度 | 原生 GUI（GPUI） | TUI（Ratatui） | WebView GUI（Tauri/Electron + TS） |
|------|-----------------|----------------|-----------------------------------|
| 样式/组件资源 | 中（组件够，素材少） | 低（字符网格 + 颜色） | **高**（整个 Web 生态） |
| 富文本 / Markdown / 代码高亮 | 强（gpui-component 原生） | 弱（要自己排版，代码高亮可用 syntect 但受字符限制） | 强 |
| 图表 / 动效 / 图片 | 强 | 弱（图片需 Sixel/Kitty 协议，需终端支持） | 强 |
| 与 Rust 后端复用度 | **最高**（同一进程、同一类型） | **最高**（同一进程） | 中（跨进程/IPC，或 Rust 做后端服务） |
| 产物体积 / 启动速度 | 极小 / 快 | **最小 / 最快** | 大 / 中（WebView 或 Chromium） |
| 可测试性 | 中（`#[gpui::test]` + TestAppContext） | 高（纯状态，易单测） | 高（Web 测试栈成熟） |
| 主要风险 | 生态与上游治理不定（见 §5） | 体验上限低（无图表/花哨动效） | 体积、内存、WebView 一致性 |
| 适合场景 | 长驻桌面工具、要好看 | CLI/开发者工具、要快要轻 | 富交互、要快速迭代样式 |

## 5. GPUI 上游治理风险（本轮新增发现）

- 出现社区分支 **`gpui-ce`（GPUI – Community Edition，0.3）**，由前 Zed 员工发起，公开呼吁「社区接手继续发展」。
- 另有 `gpui-unofficial` 在 crates.io 高频发布（如 1.21.0-pre，2026-09-16）。
- 结合此前已知：`gpui` 官方仍 pre-1.0、版本间常有破坏性变更、官方文档滞后。

→ 这些不构成「不能用」，但**强化了原结论**：GPUI 必须被隔离在一个 `agent-ui` crate 内，并预留「换 UI 实现」的出口。

## 6. 折中方案：GPUIX（用 TS/React 驱动 GPUI 渲染）

- 原理：基于 `react-reconciler` 实现自定义 React Renderer；React 负责组件/状态/Diff，更新经 **napi-rs（ThreadsafeFunction）** 桥到 Rust，Rust 侧维护 retained 树，由 GPUI 直接 GPU 绘制 —— **没有 WebView**。
- 优点：Rust 后端 + TS/React 写 UI + 单可执行文件，理论上兼得两边。
- 风险：非常新（2026 年才出现），依赖 Node/Bun 与 napi 桥；桥接层是新故障面；组件生态近乎空白，遇到问题基本自己啃。
- 已知替代：Tauri（成熟稳定，但渲染走 WebView，样式资源最多）。

结论：GPUIX **可作为观察项，不建议作为一期主路线**。

## 7. 建议（分阶段，不锁死 UI）

1. **先把 UI 与 Core 的边界做成「事件流 + 命令通道」**（`agent-core` 只吐 `StreamEvent` / `SessionEvent`，只收命令），使 UI 形态成为可替换的实现细节。这条已由 ADR-002 的契约自研保证了。
2. **一期先做 TUI（Ratatui）**：Rust 全栈、同进程零桥接、开发与验证成本最低，能最快把「多模型接入 + 流式 + 工具调用」跑通；且 TUI 就是本类产品的主流形态（Claude Code / Codex CLI 同款路线）。
3. **二期再决定 GUI**：在 GPUI（原生、素材少）/ Tauri+TS（Web 渲染、素材多）/ GPUIX（前沿）之间选，此时 Core 已稳定，UI 只是消费者。
4. 若「一期就必须有富交互 GUI」是硬约束，则权衡变为：**GPUI（好看但要自己造素材）vs Tauri+TS（素材多但背 WebView）**。

## 8. 待确认

- 你的优先级排序：**最快跑通** > **最终体验** > **人力供给**，还是别的顺序？
- 交付形态是否允许「先 TUI 后 GUI」的两阶段？
- 若走 WebView GUI，是否接受 Tauri（Rust + Web 前端），还是只接受纯 TS（Electron）？

## 相关

- 上游：`00-goals-and-routes.md`
- 决策：`docs/adr/0003-ui-delivery-form.md`
- 影响：`docs/adr/0001-tech-stack.md`（UI 部分待复核）
