# Agent · 多模型接入客户端

> **人类入口**。1 分钟读完。
> 完整知识地图（全量文档表）在 [`docs/00-start-here.md`](docs/00-start-here.md)。

## 这是什么

一个跨平台桌面客户端，统一接入多家大模型（OpenAI 兼容族 / Anthropic / 本地模型），支持流式对话、多会话、模型热切换与工具调用。本地优先：密钥进系统钥匙串，历史进本地 SQLite。

## 现在到哪

**阶段：架构与契约已定，尚未落码。**

- ✅ 目标与技术路线、自研边界已明确
- ✅ Provider 接入契约（Rust trait + 类型）已成文
- ✅ 技术栈、多模型抽象路线已决策
- 🚧 UI 交付形态待决（GPUI / TUI / Tauri）
- ⬜ 代码骨架尚未创建

详细状态与下一步见 [`docs/10-now/status.md`](docs/10-now/status.md)。

## 文档怎么读

| 你是 | 从哪里进 |
|------|---------|
| 人（第一次看） | `docs/00-start-here.md` → 它给三条阅读路线 |
| Agent | `AGENTS.md` → 再按路由表加载命中文档 |

**不想读全部？** 只读 `docs/10-now/status.md`（现在做什么）和 `docs/10-now/open-questions.md`（等你拍板的事）就够跟上进度。

## 目录速览

```
AGENTS.md        agent 入口（读取协议 + 路由 + 硬约束）
docs/
  00-start-here.md   唯一知识地图（从这里开始）
  10-now/            当前状态、未决问题      <- 先看这个
  20-spec/           规格与契约（改了影响实现）
  30-decisions/      ADR：为什么这么定（只增不改）
  40-research/       调研证据（一次性消费）
  50-plans/          后期计划（二期 GUI / L3 样式引擎）
  90-templates/      文档模板
```
