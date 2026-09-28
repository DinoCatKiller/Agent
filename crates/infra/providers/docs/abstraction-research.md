# X3 · 多模型统一抽象路线调研

| | |
|---|---|
| ID | `X3` |
| 类型 | 调研（一次性消费，会过时） |
| 状态 | ✅ 完成 |
| 更新 | 2026-09-27 |
| 何时读 | 质疑「要不要自研抽象 / 引入 rig」时 |
| 规模 | ~2.3k token |

## 目的

回答一个问题：**多供应商统一抽象，是自研还是复用现成 crate**。结论直接决定 ADR-002。

## 1. Rust 生态候选（2026-09 数据）

| 方案 | 版本 | 下载量 | 许可证 | 抽象层次 | 协议覆盖 |
|------|------|--------|--------|---------|---------|
| `rig-core` | **0.42.0**（2026-08-17） | 304 万 | MIT | 语义级（`CompletionModel` trait，归一化请求/响应/流） | 26+ provider，含 Anthropic 原生 |
| `genai` | 稳定 **0.6.5**（2026-06-06）／beta 0.7.0-beta.24 | 41.8 万 | MIT OR Apache-2.0 | 高层统一 API + 按 provider 适配器 | OpenAI / Anthropic / Gemini / Ollama / Groq / DeepSeek / Vertex 等 |
| `async-openai` | 0.41.3 | 780 万 | MIT | **传输级**（`Config` trait 只抽象端点/鉴权/header） | 仅 OpenAI 协议族（含兼容端点） |
| `langchain-rust` | 4.6.0 | 15.3 万 | MIT | `Chain` / `LLM` trait 组合 | OpenAI / Anthropic 等 |

补充事实：

- Anthropic、DeepSeek **都没有官方 Rust SDK**。Anthropic 侧社区 crate 均为 0.0.x–0.3.x 早期状态；DeepSeek 因自身同时兼容 OpenAI 与 Anthropic 两种线上协议，生态默认用 OpenAI 协议客户端对接。
- `rig` 0.42.0 的 features 变动值得注意：0.40.0 中存在 `rmcp`（MCP）feature，0.42.0 已移除 → **不能指望它替你解决 MCP**。
- `rig` 两年发布 62 个版本，pre-1.0，破坏性变更与 feature 增删属于常态。
- `genai` 两年发布 142 个版本，稳定版 0.6.5 仍在演进，0.7 长期处于 beta。

## 2. 三条成熟路线的本质区别

| 路线 | 代表 | 抽象的是 | 换协议能力 | 适合 |
|------|------|---------|-----------|------|
| 传输抽象 | `async-openai` | 「同一个 OpenAI 协议打到不同端点」 | ❌ 换到 Anthropic 协议无解 | 只绑 OpenAI 协议族 |
| 语义抽象（归一化） | `rig-core` | 把各家 wire 协议归一成统一请求/响应/流/工具类型 | ✅ 最完整 | 要多 provider + 工具 + 结构化输出 |
| 适配器抽象 | `genai` | 统一高层 API + 每 provider 一个 adapter（优先用原生协议） | ✅ 但建模较浅 | 要轻量多 provider 聊天 |

## 3. 各家协议差异（归一化必须处理的地方）

### 3.1 工具调用

| 维度 | OpenAI / DeepSeek(Chat) | Anthropic |
|------|------------------------|-----------|
| 工具定义 | `tools[].function.{name, description, parameters}` | `tools[].{name, description, input_schema}` |
| 选择策略 | `auto` / `required` / `none` / 指定函数 | `auto` / `any` / `tool` / `none` |
| 返回形式 | `tool_calls[].function.arguments`（**JSON 字符串**，可能非法） | `tool_use` 内容块，`input` 是**结构化 JSON** |
| 结果回传 | `role: "tool"` + `tool_call_id` | user 消息里的 `tool_result` + `tool_use_id` |

→ 统一建模：`ToolDefinition { name, description, parameters(JSON Schema) }` + `ToolCall { id, name, arguments }`，在 provider 边界完成「字符串 ↔ 结构化」的转换，并**保留原发 id** 用于回传。

### 3.2 结构化输出（分歧最大）

| 供应商 | 支持情况 |
|--------|---------|
| OpenAI | `response_format: {type: json_schema, strict}` / Responses 的 `text.format`；strict 是**受限 schema 子集**（根必须 object、所有字段 required、`additionalProperties:false`） |
| Anthropic | 独立参数 `output_config.format: {type: json_schema, schema}` |
| DeepSeek | Chat Completions **只有** `json_object`（无 schema 校验）；严格结构化只能走 strict 工具调用（Beta）或 Responses API |

→ 结论：结构化输出必须建模为**可降级能力**，不能假设人人都有。

### 3.3 流式

| 端点 | 形态 | 文本 | 工具参数 | 结束信号 |
|------|------|------|---------|---------|
| OpenAI / DeepSeek Chat | data-only SSE | `choices[].delta.content` | `delta.tool_calls[].function.arguments`（分片） | `data: [DONE]` |
| OpenAI / DeepSeek Responses | 语义事件流 | `response.output_text.delta` | `response.function_call_arguments.delta` | `response.completed` |
| Anthropic Messages | 命名事件流 | `content_block_delta`(text_delta) | `content_block_delta`(input_json_delta) | `message_stop` |

→ 三套都要各自解析；工具参数分片需按 id 拼接；OpenAI 系需 `include_usage` 才有 usage。

## 4. 复用现成 crate 的真实成本

| 成本项 | 说明 |
|--------|------|
| 版本绑定 | rig / genai 均为 pre-1.0 且高频发布，升级即可能返工；feature 会被增删（rig 已移除 `rmcp`） |
| 契约不匹配 | 我们需要的「UI 可直接消费的流式事件」（含节流、中断、截断语义）与库的流类型语义不完全对齐，仍要写一层适配 |
| 抽象泄漏 | rig 的 `ModelHandle`/类型擦除、`providerOptions` 透传等仍需理解其内部模型 |
| 覆盖面 vs 真实需求 | 首批只需 2–3 家；26+ provider 的「免费覆盖」在早期是负债（编译体积、依赖面、升级面） |
| 复用收益 | 归一化设计、Anthropic 协议细节、schema 清洗经验 —— **这些应以「参考实现」的形式吃下来，而不是以「依赖」的形式背上** |

## 5. 结论（供 ADR-002 采信）

采用**方案 C：契约自研 + 实现可换**。

1. `agent-common` 定义内部消息/事件/错误；`agent-providers` 定义 `Provider` trait（对齐 `A2`）。**契约所有权必须在自己手上**——这是 G-1/G-2 的前提。
2. 首批 adapter：
   - **OpenAI 兼容族**自研（`reqwest` + SSE），成本低且覆盖 DeepSeek / 通义 / Kimi / GLM / Ollama / OpenRouter 等一大票供应商。
   - **Anthropic 原生**协议自研（或短期以 `genai`/自有实现对照验证），因为它没有官方 Rust SDK，且协议差异集中在 content block 与 `input_json_delta`，可控。
3. 归一化设计**参考而非依赖** `rig-core`：`ToolDefinition`/`ToolCall`/`ToolChoice`/`FinishReason` 的归一化方式、`sanitize_schema` 的 schema 清洗思路、流式的「错误/坏帧/截断」三分语义，这些都值得照抄思路。
4. 若后期要一次性铺开 10+ 家供应商，**可以在 trait 背后挂一个 `rig` 驱动的 adapter**（`impl Provider for RigProvider`），此时它只是实现细节，随时可撤——这是保留 `genai`/`rig` 作为**可选实现**而非**架构依赖**的关键。

## 相关

- 上游：`A2`（A2）
- 决策：`D2`
