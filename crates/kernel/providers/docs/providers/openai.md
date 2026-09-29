# P1 · OpenAI 兼容族（供应商差异记录）

| | |
|---|---|
| ID | `P1` |
| 类型 | 规格（供应商差异） |
| 状态 | ✅ 稳定（M3 落地，12 条契约用例全绿） |
| 更新 | 2026-09-29 |
| 何时读 | 接入或修改 OpenAI / DeepSeek / vLLM / LiteLLM 等 Chat Completions 兼容实现时 |
| 规模 | ~1.5k token |

## TL;DR

OpenAI Chat Completions 协议：Bearer 鉴权、SSE data-only 流（`[DONE]` 收尾）、工具参数是 JSON **字符串**且会分片。**最容易踩的坑**：流式不回 usage，除非显式 `stream_options.include_usage = true`（适配器已默认打开）。

## 1. 接入信息

| 项 | 值 |
|----|-----|
| 协议 | OpenAI Chat Completions（`/chat/completions`） |
| Base URL | `https://api.openai.com/v1`（占位符，**真实值由配置层注入，不入库**，见 `R1`） |
| 鉴权 | `Authorization: Bearer <api_key>`；`api_key = None` 时不带头（Ollama 等本地兼容服务） |
| 流式 | SSE data-only（无 `event:` 命名），`data: [DONE]` 为终止记录 |
| 配额与限流 | 429 按 RPM/TPM 计；错误体含 `type: tokens/requests`，不带 `Retry-After` 时走 `A6` 退避 |

## 2. 模型清单（对应代码里的 `openai_default_models()`）

| 模型 id | 上下文 | 最大输出 | 能力 |
|---------|--------|---------|------|
| `gpt-4o` / `gpt-4o-mini` | 128k | 16k | Text Vision Tools JsonMode JsonSchema |
| `gpt-4.1` / `-mini` / `-nano` | 1M | 32k | 同上 |
| `o3` | 200k | 100k | Text Tools Reasoning |
| `o4-mini` | 200k | 100k | Text Vision Tools Reasoning |

价格字段留空（波动大，由配置层覆盖）。兼容实现（DeepSeek 等）**不传这份清单**，由配置层注入自己的 `ModelSpec`。

## 3. 字段映射差异

| 我们（A2） | OpenAI | 备注 |
|------------|--------|------|
| `ToolDefinition.parameters` | `function.parameters` | 原样透传 JSON Schema |
| `ToolCall.arguments` | **JSON 字符串**（可能分片/非法） | 出方向：按 `index` 拼装后 `serde_json::from_str`，非法容错为 `Null`；入方向（回填）：`to_string()` 回字符串 |
| `ToolChoice::Required` | `"required"` | `Specific(names)` 只取第一个（协议只支持指定单个函数） |
| `ResponseFormat::JsonSchema` | `{"type":"json_schema","json_schema":{name,strict:true,schema}}` | 适配器原样透传 schema；OpenAI 要求 root 为 object、`additionalProperties:false`（**不**在此清洗，见 §6） |
| `ResponseFormat::JsonObject` | `{"type":"json_object"}` | 只保证合法 JSON |
| `FinishReason` | `stop/length/tool_calls/content_filter` | 未知值 → `Other` 并 `tracing::warn` 保留原文 |
| `Usage` | `prompt_tokens` / `completion_tokens` | 流式需 `include_usage`；未回则补 `Usage::default()`（Q1 用例 11） |
| 推理内容 | `delta.reasoning_content`（DeepSeek 等扩展字段） | 归一为 `DeltaKind::Thinking` |

## 4. 错误映射表（A2 §5 全覆盖）

| HTTP / 错误体 | `ErrorCategory` | 可重试 | 备注 |
|---------------|-----------------|--------|------|
| 401 / 403 | `Auth` | 否 | `invalid_api_key` |
| 429 | `RateLimit` | 是 | TPM/RPM 超限 |
| 400 + `code=context_length_exceeded`（或文案含 "context length" 等） | `ContextOverflow` | 否（上层裁剪后重发） | 兼容实现文案不一，按子串兜底 |
| 400 / 404 / 413 / 422 | `InvalidRequest` | 否 | 含模型不存在 |
| 408 | `Timeout` | 是 | 另：传输层 `Timeout`/`IdleTimeout` 也归此类 |
| 5xx | `Server` | 是 | 错误体 `request_id` 透传 |
| 流提前结束（EOF 无 `[DONE]`） | `End{Truncated}` 事件 | 由上层决定 | `FramePolicy::on_truncated` |
| 其它 / 网络错误 | `Unknown`（网络错误覆盖为可重试） | — | `raw` 保留原始错误体 |

## 5. Fixtures 清单

| 用例 | 目录 |
|------|------|
| 非流式基础 | `tests/fixtures/openai-compatible/chat_basic.json` |
| 流式文本 / 工具分片 / 截断 / 坏帧 / 无 usage | `…/stream_text_basic` `tool_call_split` `truncated` `bad_frame` `no_usage` |
| 错误体 429/401/500/400上下文 | `…/errors/*.json` |
| 工具结果回填 | `…/request_tool_result/request.json` |

## 6. 已知坑

- **流式 usage 默认不回**：必须显式 `stream_options.include_usage = true`（适配器已内置）。
- **role-only 首帧**：流式第一帧常只有 `role` 无 `content`，不得当文本发出。
- **`content_filter` 可能在流中途出现**：当前映射为 `finish_reason` 记录，随 `[DONE]` 收尾。
- **非图片 `File` 附件**：Chat Completions 不支持，丢弃并 `tracing::warn`（Files API 属另一条链路）。
- **json_schema strict 模式**要求 schema 子集（root object、全字段 required、`additionalProperties:false`）；清洗规则未实现，超集 schema 会被 400 拒绝 → 登记为后续任务（`A5`/`R2` 待定）。
- **`o*` 推理模型**：不接受 `temperature` 等采样参数（由配置层/调用方负责不发）。

## 相关

- 契约：`A2`、消息映射：`A3` ｜ 测试要求：`Q1` ｜ 传输管线：`A4` ｜ 抽象路线决策：`D2`
