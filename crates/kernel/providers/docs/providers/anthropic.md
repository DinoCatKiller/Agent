# P2 · Anthropic（供应商差异记录）

| | |
|---|---|
| ID | `P2` |
| 类型 | 规格（供应商差异） |
| 状态 | ✅ 稳定（M3 第二适配器落地，12 条契约用例全绿） |
| 更新 | 2026-09-30 |
| 何时读 | 接入或修改 Claude 系模型 / Anthropic 兼容端点时 |
| 规模 | ~1.6k token |

## TL;DR

Anthropic Messages 协议：`x-api-key` 鉴权、SSE **命名事件**流（`event: message_stop` 收尾）、
工具参数是结构化 `input`（非字符串）。**最容易踩的坑**：`max_tokens` 必填、`system` 是顶层
参数、消息 role 只有 user/assistant 且**必须交替**（连续 Tool 结果要合并进同一条 user 消息）。

## 1. 接入信息

| 项 | 值 |
|----|-----|
| 协议 | Anthropic Messages（`/v1/messages`，适配器负责拼路径） |
| Base URL | `https://api.anthropic.com`（占位符，**真实值由配置层注入，不入库**，见 `R1`） |
| 鉴权 | 头 `x-api-key: <api_key>` + `anthropic-version: 2023-06-01`；`api_key = None` 时不带 `x-api-key`（部分本地代理） |
| 流式 | SSE **命名事件**（`event: message_start` / `content_block_delta` / …），`message_stop` 为终止记录，另有 `ping` 心跳 |
| 配额与限流 | 429 按 ITPM/OTPM 计，可能带 `Retry-After` 头；529 `overloaded_error`（非标准状态码）→ `A6` 退避 |

## 2. 模型清单（对应代码里的 `anthropic_default_models()`）

| 模型 id | 上下文 | 最大输出 | 能力 |
|---------|--------|---------|------|
| `claude-opus-4-1-20250805` | 200k | 32k | Text Vision Tools Reasoning |
| `claude-sonnet-4-5-20250929` | 200k | 64k | 同上 |
| `claude-sonnet-4-20250514` | 200k | 64k | 同上 |
| `claude-3-7-sonnet-20250219` | 200k | 64k | 同上 |
| `claude-3-5-sonnet-20241022` | 200k | 8k | Text Vision Tools |
| `claude-3-5-haiku-20241022` | 200k | 8k | Text Vision Tools |

价格字段留空（波动大，由配置层覆盖）。清单是**接入时快照**：带日期的 id 不含糊，新模型由配置层注入自己的 `ModelSpec`。能力清单刻意**不含** `JsonMode` / `JsonSchema`（协议无 `response_format`，见 §3）。

## 3. 字段映射差异

| 我们（A2） | Anthropic | 备注 |
|------------|-----------|------|
| `Message(role=System)` | 顶层 `system` 参数 | 多条按序以 `\n\n` 连接；**不允许**出现在 messages 里 |
| 消息 role 交替 | 强制 user/assistant 交替 | 连续同角色 400；适配器**合并**：连续 User 合并、连续 Tool 结果并入同一条 user 消息的多个 `tool_result` 块 |
| `Role::Tool` | user 消息里的 `{"type":"tool_result","tool_use_id",…}` 块 | 工具结果回填（`Q1` 用例 4） |
| `ToolCall.arguments` | **结构化 `input`**（非字符串） | 双向都是 JSON 对象，无需解析容错（对比 `P1` 的字符串分片） |
| 流式工具参数 | `content_block_start(tool_use)` → `input_json_delta` 分片 → `content_block_stop` | 按 `index` 拼装；start 自带完整 `input` 且无分片时直接用 |
| `ToolDefinition.parameters` | `input_schema` | 原样透传 JSON Schema |
| `ToolChoice::Required` | `{"type":"any"}` | `Specific(names)` 只取第一个 → `{"type":"tool","name":…}` |
| `ResponseFormat::*` | 无对应 | 靠能力协商拦截（清单模型缺 `JsonMode`/`JsonSchema` 即 fast-fail）；未登记清单的兼容实现会**静默忽略** |
| `max_tokens` | **必填** | 调用方未给时适配器兜底 `DEFAULT_MAX_TOKENS = 4096` |
| `FinishReason` | `end_turn`/`stop_sequence`→Stop、`max_tokens`/`model_context_window_exceeded`→Length、`tool_use`→ToolCalls、`refusal`→ContentFilter | 未知值 → `Other` 并 `tracing::warn` |
| `Usage` | `input_tokens` / `output_tokens` | 流式分两处：`message_start` 给 input，`message_delta` 给累计 output，适配器在终止处合并（`A2` §4：`End` 前必须给一次） |
| 推理内容 | `thinking_delta` → `DeltaKind::Thinking` | `signature_delta` 忽略（校验签名，非用户可见） |
| `Image{url}` | `data:` URI 拆为 base64 source；其余走 URL source | URL source 要求模型支持 |
| 非图片 `File` | 不支持，丢弃并 `tracing::warn` | PDF `document` 块属二期 |

## 4. 错误映射表（A2 §5 全覆盖）

| HTTP / 错误体 | `ErrorCategory` | 可重试 | 备注 |
|---------------|-----------------|--------|------|
| 401（authentication_error）/ 403（permission_error） | `Auth` | 否 | |
| 429（rate_limit_error） | `RateLimit` | 是 | 可能带 `Retry-After` |
| 400 + 文案含 "prompt is too long" 等 | `ContextOverflow` | 否（上层裁剪后重发） | 按子串兜底（兼容实现文案不一） |
| 400 / 404 / 413 / 422 | `InvalidRequest` | 否 | 含模型不存在（not_found_error）、request_too_large |
| 408 / timeout_error | `Timeout` | 是 | 另：传输层 `Timeout`/`IdleTimeout` 也归此类 |
| 5xx（api_error）与 **529**（overloaded_error） | `Server` | 是 | 529 不在标准 5xx 段内，单独映射；`request_id` 在错误体**顶层** |
| 流中 `error` 事件 | 按 `error.type` 映射（同上） | — | 如 overloaded → `Server`；产出 `StreamEvent::Error` 后停止（`A2` §4） |
| 流提前结束（EOF 无 `message_stop`） | `End{Truncated}` 事件 | 由上层决定 | `FramePolicy::on_truncated` |
| 其它 / 网络错误 | `Unknown`（网络错误覆盖为可重试） | — | `raw` 保留原始错误体 |

## 5. Fixtures 清单

| 用例 | 目录 |
|------|------|
| 非流式基础 | `tests/fixtures/anthropic/chat_basic.json` |
| 流式文本 / 工具分片 / 截断 / 坏帧 / 无 usage | `…/stream_text_basic` `tool_call_split` `truncated` `bad_frame` `no_usage` |
| 错误体 429/401/500/400上下文 | `…/errors/*.json` |
| 工具结果回填 | `…/request_tool_result/request.json` |

与 `openai-compatible/` **同构**（`Q1` §2），12 条用例一一对应——这是「可替换」的验收方式。

## 6. 已知坑

- **`max_tokens` 必填**：不传直接 400；适配器兜底 4096，正式值应由调用方/配置层给出。
- **role 必须交替**：连续同角色 400。适配器已做合并（连续 User、连续 Tool 结果），但「assistant 是首条消息」仍会被拒——Core 组装历史时保证 user 起头。
- **usage 分两处**：`message_start` 只给 input，`message_delta` 只给累计 output；缺任一处按 0 计，两处都缺则补 `Usage::default()`（`Q1` 用例 11）。
- **流中可能出现 `error` 事件**（典型 overloaded）：适配器映射为 `StreamEvent::Error` 并终止，之后不再发任何事件。
- **无 `response_format`**：JSON 输出只能靠 prompt 或工具约束；清单模型已在能力协商中排除 `JsonMode`/`JsonSchema`。
- **`count_tokens` 有真实端点**（POST `/v1/messages/count_tokens`）但 `Provider::count_tokens` 是同步签名，一期用本地估算（≈4 字符/token）；是否接异步计数随 `A2` §7 未决项。
- **thinking 模型不接受 `temperature`**（开启扩展思考时）；`thinking` / `top_k` 等厂商参数走 `additional_params` 透传。
- **兼容代理的鉴权头可能不同**（部分要 `Authorization: Bearer`）；当前只发 `x-api-key`，需要时加配置项。
- **消息体里的空文本块会被 400**（如 `assistant("")` 占位）：适配器跳过空文本块、丢弃空消息段。

## 相关

- 契约：`A2`、消息映射：`A3` ｜ 测试要求：`Q1` ｜ 传输管线：`A4` ｜ 抽象路线决策：`D2`
