# ADR-002：多模型统一抽象路线

> 索引 ID：`D2` ｜ 状态：**已接受** ｜ 日期：2026-09-27

## 背景

需要「接入各种模型」，且要求 Core 不依赖任何供应商私有类型（G-1），契约编译期固化（G-2）。Rust 生态已有多个成熟度不同的统一抽象 crate，需决定复用还是自研。

## 备选方案

| 方案 | 说明 | 优点 | 缺点 |
|------|------|------|------|
| A：直接依赖 `rig-core` | 用其 `CompletionModel` / agent / 工具编排 | 26+ provider 开箱可用；归一化最完整 | pre-1.0、两年 62 个版本、feature 有增删（0.42 移除 `rmcp`）；契约所有权在外部；其 agent 抽象与我们的 UI 流式语义不完全对齐 |
| B：直接依赖 `genai` | 用其统一高层 API | 轻、原生协议适配好 | 建模较浅（工具/结构化输出）；0.7 长期 beta；契约同样在外部 |
| C：**契约自研 + 实现可换** | 自己定义 schema 与 `Provider` trait；adapter 实现可自研、也可在 trait 背后包 rig/genai | 契约所有权在自己；Core 零供应商依赖；可渐进替换；不被上游版本节奏绑架 | 首批 adapter 需自己写（OpenAI 兼容 + Anthropic 协议细节） |

## 决定

采用 **方案 C**。具体边界：

1. `agent-schema` 定义内部消息 / 流式事件 / 错误类型；`agent-providers` 定义 `Provider` trait，严格对齐 `A2`。
2. 首批 adapter 自研：
   - **OpenAI 兼容族**（`reqwest` + SSE）——一份实现覆盖 OpenAI / DeepSeek / 通义 / Kimi / GLM / Ollama / OpenRouter 等。
   - **Anthropic 原生**（content block + `input_json_delta`）——无官方 Rust SDK，协议差异可控。
3. 归一化设计**参考 `rig-core` 而非依赖它**：`ToolDefinition`/`ToolCall`/`ToolChoice`/`FinishReason` 的归一化方式、`sanitize_schema` 的 strict 子集清洗、流式的「传输错误 / 可恢复坏帧 / EOF 截断」三分语义。
4. 保留逃生通道：若后续要一次铺开 10+ 家供应商，实现 `impl Provider for RigBackedProvider`，把 rig 降级为**实现细节**，可随时撤除。

## 影响

- 正面：满足 G-1/G-2；升级不受上游破坏性变更影响；流式事件可直接被 GPUI 消费。
- 代价：首批适配器的协议细节（SSE 三套语义、工具分片拼接、schema 降级）需自行实现并配 fixture 测试；供应商 API 漂移由自己跟进。
- 后续动作：`A2` 从伪代码落成 Rust trait；建立 fixtures 契约测试（见 `Q1`）。

## 未决项

首批是否加入 Gemini 原生、是否本期支持 MCP → 统一登记在 `S2` Q3 / Q4。
