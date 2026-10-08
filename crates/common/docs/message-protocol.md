# A3 · 消息与多模态协议

| | |
|---|---|
| ID | `A3` |
| 类型 | 规格 |
| 状态 | ✅ 已落地（`crates/common/src/message.rs`） |
| 更新 | 2026-09-28 |
| 何时读 | 涉及文本/图片/文件输入、role 语义、工具消息回填 |
| 规模 | ~1.5k token |

## TL;DR

- 内部只有 **4 个 role**：`System` / `User` / `Assistant` / `Tool`。
- 内容统一为 `Vec<ContentPart>`，一期实现 `Text` / `Image` / `File`（音频留位）。
- 工具结果回填 = `role: Tool` + **必填** `tool_call_id`；`Assistant` 发起调用时 `tool_calls` 非空、`content` 可为空。
- 所有厂商差异在**适配器边界**消化，Core 只认这里定义的类型。
- 图片/文件一律用 URL 或 `data:` URI 传递，**不做本地路径透传**（避免适配器各自读盘）。

## 1. Role 语义

| Role | 谁产生 | 约束 |
|------|--------|------|
| `System` | 应用 | 只允许出现在最前部（Anthropic 要求 system 独立成参数，由适配器提取） |
| `User` | 用户 / 应用 | 可携带图片、文件 |
| `Assistant` | 模型 | 若发起工具调用，`tool_calls` 非空；`content` 允许为空 |
| `Tool` | 应用回填 | **必须**带 `tool_call_id`，一条结果对应一次调用 |

## 2. ContentPart

| 变体 | 字段 | 备注 |
|------|------|------|
| `Text` | `text` | 最常用 |
| `Image` | `url`（URL 或 `data:` URI）、`mime_type?` | 需要 `Vision` 能力 |
| `File` | `name`、`mime_type?`、`data?`、`url?` | 需要 `File` 能力；一期只透传不解析 |

音频（`Audio`）留位，等有明确场景再加，避免过早复杂化。

## 3. 各厂商映射（适配器职责）

| 内部形态 | OpenAI / 兼容族 | Anthropic |
|----------|----------------|-----------|
| `System` 消息 | `messages[0].role = "system"` | 顶层 `system` 参数（不在 messages 里） |
| `Text` part | `{"type":"text","text":…}` | `{"type":"text","text":…}` |
| `Image` part | `{"type":"image_url","image_url":{"url":…}}` | `{"type":"image","source":{…}}` |
| 助手工具调用 | `tool_calls[].function.arguments`（**JSON 字符串**） | `tool_use` 内容块，`input` 为**结构化 JSON** |
| 工具结果 | `role:"tool"` + `tool_call_id` | user 消息内的 `tool_result` + `tool_use_id` |
| 多个 tool_result | 多条独立 `role:"tool"` 消息 | 同一 user 消息里的多个 `tool_result` 块 |

> 内部模型取**并集**语义，不偏向任何一家；映射细节写在各 `providers/*.md` 里。

## 4. 序列化约定

- 所有类型都要 `Serialize + Deserialize`：它们是 fixtures 回放与跨进程传递的格式。
- 枚举用 `#[serde(tag = ...)]` 带内部标签，便于日志里肉眼识别。
- **空集合不序列化**（`skip_serializing_if`）：避免把 `"tools": []` 发给不支持的供应商。
- `Message::text()` 只用于日志/预览，**不得**用于重新序列化（会丢结构）。

## 5. 落地清单

- [x] `Role` / `ContentPart` / `Message` 类型（`agent-common`）
- [x] `Message::system/user/assistant/tool_result` 构造器
- [x] `tool_call_id` 必填约束（构造器形态）
- [x] 运行期校验：`role = Tool` 而缺 `tool_call_id` 时返回 `InvalidRequest`（M4 已落：`Chat::request`，见 `crates/features/chat/src/chat.rs`）
- [ ] 多模态 fixtures（图片 / 文件输入的解析用例，随首个多模态任务建）

## 相关

- 上游：`A2`
- 下游：流式 → `A4`（待创建）、工具调用 → `A5`（待创建）
- 未决项 → `S2`
