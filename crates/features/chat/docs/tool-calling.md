# A5 · 工具调用

| | |
|---|---|
| ID | `A5` |
| 类型 | 规格（归属 `agent-chat`） |
| 状态 | ✅ 已落地（M4：trait + 顺序循环 + 错误喂回；并行化随 M6） |
| 更新 | 2026-09-30 |
| 何时读 | 加工具执行方式、改循环策略、做并行/降级（M6）前 |
| 规模 | ~1.3k token |

## TL;DR

- `chat` 只认 [`Tool`](../../src/tools.rs) trait（定义 + 执行），实现由宿主提供（`app` demo 工具 / 测试假工具 / 将来的 MCP，见 `S2` Q4）。
- 工具**执行错误不中断循环**：作为 `tool_result` 文本喂回模型，模型自救；反复失败由 `max_model_turns`（默认 8）兜底。
- M4 顺序执行；M6 并行化 / 失败降级只改 `ToolSet` 调度，**不改 trait**。
- 工具参数的归一化（分片拼装、字符串→结构化）**已在适配器边界完成**（`A2` §3），本层只见结构化 JSON。

## 1. Tool trait 契约

```rust
pub trait Tool: Send + Sync {
    fn def(&self) -> ToolDefinition;                       // name / description / JSON Schema
    fn run(&self, arguments: Value) -> BoxFuture<'static, ToolOutput>;
}
pub type ToolOutput = Result<Value, String>;               // Err = 错误文案（喂回模型）
```

- 手写 `BoxFuture` 与 `Provider` trait 风格一致，不引 `async_trait`。
- `run` 应当是**纯输入输出**：无会话状态、无 UI 依赖。取消由调用方（service）在 future 外层 `select`，工具自身不必感知。
- `ToolSet`：注册表。`definitions()` 供模型选择，`get(name)` 供调度；同注册表内工具名不可重复（后注册者被忽略——由测试约束）。

## 2. 执行语义（M4）

| 情形 | 行为 |
|------|------|
| 模型请求工具 `finish_reason = ToolCalls` | 循环继续：逐个执行 → 回填 `role=Tool` 消息 → 再次调用模型 |
| 工具执行 `Err` | 内容 = `error: <文案>`，`ToolFinished { ok: false }`，照常回填 |
| 未知工具名 | 同上：`error: unknown tool: <name>` 喂回（模型可改道或道歉） |
| 取消（工具执行中） | 立即 `Cancelled` 收轮；**已执行完的结果不回填、不再次调用模型** |
| 达到 `max_model_turns` | `RoundEnded { MaxTurns }`；**最后一轮的工具调用不执行**（执行了也没人消费结果） |

顺序执行是刻意的 M4 简化：并行执行要处理「部分失败时的聚合策略」与事件交错，随 M6 一并设计。

## 3. 事件映射（`LoopEvent`）

编排层对 UI 暴露的是 [`LoopEvent`](../../src/service.rs)，不是原始 `StreamEvent`：

| `StreamEvent`（A2 §4） | `LoopEvent` | 备注 |
|---|---|---|
| `Delta(Text)` | `Delta { Text }` | 原样透传 |
| `Delta(Thinking)` | `Delta { Thinking }` | 只显示不进历史（`A3` 无 thinking 变体） |
| `ToolCall` | （无） | 分片已在适配器拼装；入 `TurnDraft`，`content_block` 收口后由循环调度 |
| `Usage` / `End` | `ModelTurnEnded { finish_reason, usage }` | **只在此时入史** |
| `Error` | `Error` | 之后不再有事件；未提交增量自动回滚 |
| （循环产生） | `ToolStarted` / `ToolFinished` / `RoundEnded` / `Cancelled` | UI 据此渲染工具面板与轮次统计 |

## 4. 与契约的关系

- `ToolDefinition.parameters` = JSON Schema，由适配器翻译为各厂商形态（`P1` / `P2` §3）。
- 回填消息 = `Message::tool_result(call_id, 内容)`（`A3` §1），一条结果对应一次调用。
- `ToolCall.id` 用适配器归一化后的句柄；`provider_id`（厂商原发 id）本层不使用。

## 落地清单

- [x] `Tool` trait + `ToolSet` 注册表（M4）
- [x] 顺序执行 + 错误喂回 + 未知工具容错（M4）
- [x] `max_model_turns` 兜底（M4）
- [ ] 并行执行与结果聚合（M6）
- [ ] 失败降级链（工具失败 → 换工具/换模型，M6，`R2`）
- [ ] MCP 客户端接入与否 → `S2` Q4
- [ ] 工具调用审批（human-in-the-loop）→ GUI 期

## 相关

- 契约：`A2` §3（工具类型）、`A3` §1（回填消息）｜ 编排：`features/chat` README ｜ 未决项 → `S2`
