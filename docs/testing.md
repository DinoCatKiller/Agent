# Q1 · 测试策略

| | |
|---|---|
| ID | `Q1` |
| 类型 | 规格 |
| 状态 | 🚧 草案（骨架已可用，fixtures 待 M3 建立） |
| 更新 | 2026-09-28 |
| 何时读 | 写单测、契约测试、mock 供应商、加新适配器前 |
| 规模 | ~1.4k token |

## TL;DR

- 三层测试：**契约测试（fixtures 回放）** > 单元测试 > 冒烟（`agent-app self-check`）。
- 硬目标：**断网可跑全绿**（G-3）。任何测试不得依赖真实 API key 与网络。
- 每个适配器必须通过**同一套** fixtures 用例清单，这是「可替换」的验收方式。
- fixtures 存**原始响应字节**（不是转换后的结构），才能真正回归解析逻辑。
- 提交前：`cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test --workspace`。

## 1. 测试分层

| 层 | 位置 | 测什么 | 依赖 |
|----|------|--------|------|
| 契约测试 | `crates/kernel/providers/tests/` | 适配器对 fixtures 的解析与归一化、错误映射 | 无网络（mock HTTP 或直接喂字节） |
| 单元测试 | 各 crate 内 `#[cfg(test)]` | 纯函数：能力推导、schema 清洗、增量拼接 | 无 |
| 冒烟 | `cargo run -p agent-app -- self-check` | 契约类型可构造/可序列化/能力协商可拒绝 | 无 |
| 端到端 | `crates/app/tests/` | Core 的多轮工具循环（M4 起） | 无（用假 provider） |

## 2. Fixtures 约定

```
crates/kernel/providers/tests/fixtures/
  <provider>/                     # 如 openai-compatible / anthropic
    <case>/                       # 如 stream_text_basic
      request.json                # 我们发出去（归一化后的 ModelRequest）
      wire_request.json           # 期望实际发出的厂商格式（可选，用于校验映射）
      wire_response.sse           # 原始 SSE 字节（含 \n\n 分隔、[DONE]）
      expected_events.jsonl       # 期望的归一化 StreamEvent 序列（逐行）
      meta.json                   # { model, notes, recorded_at }
```

规则：

1. **原始字节优先**：`wire_response.*` 必须是真实抓包（可脱敏），不要手工编造理想格式。
2. 错误用例同样要 fixtures（429 / 401 / 5xx / 截断 / 坏帧）。
3. 每个 fixture 在 `meta.json` 里写清来源与录制日期，便于过时后重录。

## 3. 必测用例清单（每个适配器都要过）

| # | 用例 | 断言要点 |
|---|------|---------|
| 1 | 非流式基础对话 | `text`、`usage`、`finish_reason = Stop` |
| 2 | 流式文本 | 事件顺序为 `Start → Delta* → Usage → End`；拼接结果等于完整文本 |
| 3 | 流式工具参数分片 | `ToolCall.arguments` 是**可解析的完整 JSON**（跨多个 delta 拼装） |
| 4 | 工具结果回填 | 映射为正确的厂商形态（`role: tool` / `tool_result`） |
| 5 | 截断（EOF 无终止记录） | 补 `End{Truncated}` 或 `Error{Truncated}`，**不得当成功** |
| 6 | 坏帧（中间一条非法 JSON） | 跳过该帧、流继续，最终仍正常结束 |
| 7 | 429 | `ErrorCategory::RateLimit` + `retryable = true` |
| 8 | 401 / 403 | `Auth` + `retryable = false` |
| 9 | 5xx | `Server` + `retryable = true` |
| 10 | 超上下文 | `ContextOverflow`（供上层裁剪重试） |
| 11 | 无 usage | 仍必须以 `Usage::default()` 发一次，保持契约 |
| 12 | 能力不符（如 vision 打到纯文本模型） | 应在**路由阶段**拒绝，根本不产生请求 |

## 4. Mock 与工具

- HTTP 层用 `wiremock`（本地假服务器，可断言发出的请求体），比自写 mock server 省事。
- 流式断言直接比对 `Vec<StreamEvent>`，不要比对字符串输出——顺序与内容都要检验。
- 时间相关（退避、超时）用 `tokio::time::pause()` 做确定性测试，禁止 `sleep` 真等。

## 5. 门禁

| 命令 | 用途 |
|------|------|
| `cargo test --workspace` | 全部测试 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 静态检查（CI 必过） |
| `cargo fmt --check` | 格式 |
| `cargo run -p agent-app -- self-check` | 契约冒烟 |

## 相关

- 上游：`A2`、`G1`（G-3 断网可测）
- 下游：评测与回归 → `Q2`（待创建）
- 未决项 → `S2`
