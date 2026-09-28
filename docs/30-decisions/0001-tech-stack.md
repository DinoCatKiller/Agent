# ADR-001：技术栈选型

> 索引 ID：`D1` ｜ 状态：**已接受**（UI 交付形态复核中 → 见 ADR-003） ｜ 日期：2026-09-27

## 背景

项目目标为「Agent 接入各种模型」的桌面应用。需要确定语言、运行时、UI 框架与包管理，才能定稿目录结构与契约表达形式。

## 备选方案

| 方案 | 优点 | 缺点 |
|------|------|------|
| A：TypeScript + Node + Electron/Tauri | 流式与供应商 SDK 生态最全 | 体积大、内存高；Electron 与「高性能原生体验」目标冲突 |
| B：Python + Qt/TUI | 模型侧生态最强 | 前端体验与分发是弱项 |
| **C：Rust 后端 + GPUI 前端** | 单可执行文件、无 WebView/Electron；GPU 加速、120 FPS；同一套类型贯通前后端；IO 密集场景 `tokio` 成熟 | GPUI 仍 pre-1.0；Rust 侧无官方 Anthropic Rust SDK，需自建适配层 |

## 决定

采用 **方案 C**：

- 语言 / 运行时：**Rust（edition 2024，stable ≥ 1.95）+ tokio**
- HTTP：`reqwest`（rustls）
- 序列化：`serde` / `serde_json` / `schemars`
- 前端：**GPUI**，经 `gpui-kit`（含 `gpui-component` 0.6.x）间接依赖，`gpui` 基线 **0.2.2**
- 存储 / 密钥：`rusqlite`（bundled）+ `keyring`
- 打包：Windows 用 `cargo-wix`（MSI）

理由（按重要性排序）：

1. GPUI 让「流式渲染 + 大列表 + 代码/ Markdown 原生渲染」不经过 WebView，直接落到 GPU；`gpui-component` 已提供 Markdown、HTML、Tree-sitter 编辑器、虚拟表格等本项目刚需能力。
2. Rust 的类型系统可以把 `docs/20-spec/provider-contract.md` 的契约固化成**编译期约束**，直接服务于 G-1/G-2。
3. 单可执行产物 + 系统钥匙串 + 本地 SQLite，符合「本地优先、无云依赖」的产品目标。

## 影响

- **UI 交付形态待复核**：GPUI 的表达能力够用，但样式素材、第三方库与人力供给少于 Web/TS 生态；且 GPUI 无法做 TUI。当前建议「先 Ratatui TUI 验证 Core，二期再定 GUI」→ 见 `30-decisions/0003-ui-delivery-form.md` 与 `40-research/ui-delivery-options.md`。
- 目录结构按 `40-research/rust-backend-stack.md` §3 的 cargo workspace 落地。
- GPUI 相关调用收敛在 `agent-ui` 单一 crate 内，`agent-core` 不感知 UI。
- 需自建 Anthropic 等原生协议适配层（无官方 Rust SDK）。
- 需要处理 GPUI pre-1.0 的破坏性升级风险（固定版本 + 隔离层）。
- 若选 A/B 方案的讨论作废：契约的**表达形式**可能变化，但 `docs/20-spec/provider-contract.md` 的语义不变。

## 未决项

产物体积上限、是否需要代码签名与公证 → 统一登记在 [`S2`](../10-now/open-questions.md) Q6。

## 相关

- 调研：`40-research/gpui-frontend-stack.md`、`40-research/rust-backend-stack.md`
- 目标：`20-spec/goals-and-routes.md`
