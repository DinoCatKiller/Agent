# S1 · 当前阶段（概览）

| | |
|---|---|
| ID | `S1` |
| 类型 | 状态 · **概览** |
| 更新 | 2026-09-30 |
| 何时读 | **每次开工第一眼**，5 秒确认「现在做什么」 |
| 规模 | ~0.5k token |

> **本文件只写当前阶段的任务。** 任务完成 → 清空并重写；历史交给 git，长期排期见 `RM`。
> 任务跨多个 crate 时：**这里只给概览**，细节写在相关 crate 的 `status.md` 里，并从下表链过去。

## 当前任务

**M3 · 第二适配器（Anthropic）** ｜ 状态：✅ 完成（2026-09-30）

一句话：Anthropic Messages 适配器用同一套 fixtures 结构跑通 `Q1` 的 12 条用例（14 测试含 2 个 wiremock 端到端），两个适配器契约测试各自全绿，验证契约「可替换」。

| crate | 本任务中它要做什么 | 细节 |
|-------|-----------------|------|
| `crates/kernel/providers` | `anthropic.rs`：`AnthropicCompatible`（chat/stream/count_tokens）、`AnthropicFramePolicy`（命名事件 SSE→归一化、`tool_use` 拼装、role 交替合并）、错误映射覆盖 `A2` §5；`P2` 文档 + 12 用例 fixtures；能力协商与传输错误映射提取为 crate 级共享函数 | 已收口，crate `status.md` 已随任务删除 |
| `crates/kernel/transport` | **不动**（M2 已收口） | — |
| `crates/common` | 未动（契约无需补字段） | — |

## 阻塞

- 无。

## 上一阶段

- **M3 首个适配器完成**：OpenAI 兼容族跑通 `Q1` 12 条用例（`P1` 落地），端到端兑现「一次真实请求能流式收完」。
- **M2 完成**：传输层 HTTP / SSE / 三分收尾 / 取消，规格见 `A4`；重试随 `A6` 落地。

## 下一阶段

- **M4 · 编排核心（`features/chat`）**：多轮工具循环、会话持久化决策（`S2` Q8）、上下文裁剪（`A7`）。

## 相关

- 里程碑与排期定义 → `RM` ｜ 未决问题 → `S2` ｜ 地图 → [`AGENTS.md`](AGENTS.md) §2
