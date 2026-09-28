# P? · <供应商名>（供应商差异记录）

| | |
|---|---|
| ID | `P?` |
| 类型 | 规格（供应商差异） |
| 状态 | 草案 / 稳定 |
| 更新 | YYYY-MM-DD |
| 何时读 | 接入或修改这一家供应商时 |
| 规模 | ~Xk token |

## TL;DR

一句话说清：协议类型（原生 / OpenAI 兼容）、鉴权方式、最容易踩的一个坑。

## 1. 接入信息

| 项 | 值 |
|----|-----|
| 协议 | 原生 / OpenAI 兼容 / Anthropic 兼容 |
| Base URL | `https://…`（占位符，**真实值不入库**） |
| 鉴权 | `Authorization: Bearer` / header `x-api-key` / … |
| 流式 | SSE：data-only（`[DONE]`）/ 语义事件 / 命名事件 |
| 配额与限流 | 已知的 RPM / TPM / 并发限制 |

## 2. 模型清单（对应代码里的 `ModelSpec`）

| 模型 id | 上下文 | 最大输出 | 能力 | 价格（入/出，每 M token） |
|---------|--------|---------|------|--------------------------|
| | | | | |

## 3. 字段映射差异

| 我们（A2） | 该供应商 | 备注 |
|------------|---------|------|
| `ToolDefinition.parameters` | | |
| `ToolCall.arguments` | | 字符串还是结构化 |
| `ToolChoice::Required` | | |
| `ResponseFormat::JsonSchema` | | 是否支持、schema 子集限制 |
| `FinishReason` | | 取值表与归一化 |
| `Usage` | | 字段名、是否需要开关才有 |

## 4. 错误映射表（A2 §5 全覆盖）

| HTTP / 错误体 | `ErrorCategory` | 可重试 | 备注 |
|---------------|-----------------|--------|------|
| 401 / 403 | `Auth` | 否 | |
| 429 | `RateLimit` | 是 | 是否带 `Retry-After` |
| 400（参数 / 模型不支持） | `InvalidRequest` | 否 | |
| 5xx | `Server` | 是 | |
| 上下文超限的错误码 | `ContextOverflow` | 一次 | |

## 5. Fixtures 清单

| 用例 | 文件 | 录制日期 |
|------|------|---------|
| 流式文本 | `tests/fixtures/<provider>/stream_text_basic/` | |

## 6. 已知坑

- （如：某些模型不发 `[DONE]`；`stream_options.include_usage` 必须显式打开；…）

## 相关

- 契约：`A2`、消息映射：`A3`
- 测试要求：`Q1`
- 抽象路线决策：`D2`
