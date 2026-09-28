# S1 · 当前阶段状态

| | |
|---|---|
| ID | `S1` |
| 类型 | 状态（高频更新） |
| 更新 | 2026-09-28 |
| 何时读 | **每次开工第一眼**，5 秒确认「现在到哪」 |
| 规模 | ~0.7k token |

## TL;DR

- **阶段**：架构与契约已定，**尚未落码**。
- **已完成**：目标与技术路线、Provider 接入契约、技术栈决策、多模型抽象决策、GPUI/TUI/样式可移植性调研。
- **卡点**：UI 交付形态未定（→ D3）＋ 样式层路线未确认（→ D4）＋ S2 的几个问题待拍板。
- **下一步**：按下面 §下一步 顺序执行。

## 里程碑

| # | 里程碑 | 状态 |
|---|--------|------|
| M0 | 文档体系与关键决策定稿 | 🚧 剩 D3/D4 + S2 待拍板 |
| M1 | cargo workspace 骨架 + `agent-schema` 类型落地 | ⬜ |
| M2 | `agent-transport`：reqwest + SSE 解析 + 超时/重试 | ⬜ |
| M3 | 首个 adapter（OpenAI 兼容族）+ 契约测试（fixtures 回放） | ⬜ |
| M4 | `agent-core`：单轮对话 + 流式贯通（无 UI，CLI 验证） | ⬜ |
| M5 | UI 层（形态由 D3 定）+ 会话持久化 | ⬜ |
| M6 | 工具调用 + 模型路由与兜底 | ⬜ |

## 下一步（按顺序）

1. 用户拍板 D3（UI 形态）、D4（样式层）与 S2 的三个问题。
2. 建 cargo workspace 骨架（`cargo new`，**不预建空目录**，骨架见 A1 §5）。
3. 把 `20-spec/provider-contract.md` 落成 `agent-schema` 的类型 + `agent-providers` 的 `Provider` trait。
4. 建 fixtures 与首个 adapter 的契约测试骨架。

## 决策速查

| ID | 结论 | 状态 |
|----|------|------|
| D1 | 技术栈 = Rust + tokio（UI 待复核） | ✅ 已接受 |
| D2 | 多模型抽象 = 契约自研 + 实现可换 | ✅ 已接受 |
| D3 | UI 交付形态 | 🚧 待决 |
| D4 | 样式层路线（token 桥 + 组件层） | 🚧 待确认 |

## 相关

- 未决问题 → [`S2 open-questions.md`](open-questions.md)
- 地图 → [`docs/00-start-here.md`](../00-start-here.md)
