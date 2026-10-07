# agent-chat · 功能切片：对话

| | |
|---|---|
| ID | —（功能切片，非索引文档） |
| 状态 | ✅ M4 落地：状态机 + 轮次编排（事件流）+ 工具边界 + 上下文裁剪；`ui.rs` 等 `D3` |
| 边界 | **语义**：一次对话怎么进行、状态怎么变、界面长什么样。含工具调用编排与上下文管理 |
| 依赖 | `agent-common`（契约）｜ `agent-providers`（`Provider` / `CallContext`） |
| 不允许 | 依赖 `app`；被 `kernel/*` 依赖；**自己实现 HTTP / SSE 帧解析 / 连接池** —— 那是机制，属 `kernel`（`D6` §1） |
| 何时读 | 改对话逻辑、加对话能力、写对话界面时 |

## 内部布局（`D6` §3）

```
src/
  lib.rs        模块声明 + 对外导出
  chat.rs       状态机：Chat（历史 + 累计用量）+ TurnDraft（流式累积，只在 End 后入史）
  service.rs    轮次编排：run_round 事件流（Delta/Tool/ModelTurnEnded/RoundEnded/Cancelled/Error）
  tools.rs      工具执行边界：Tool trait + ToolSet 注册表（实现由宿主提供）
  context.rs    上下文裁剪：交换块整块丢弃，保 tool 配对与当前轮（A7）
  repo.rs       本功能的持久化 —— M5 随 kernel/store 落地（S2 Q8 已闭环）
  ui.rs         本功能的界面 —— 等 D3 定了再建
tests/round.rs  编排集成测试（假 provider，9 例，断网）
docs/           规格：A5（工具调用）、A7（上下文与 Token）
```

## 核心用法（CLI 消费者见 `crates/app/src/repl.rs`）

```rust
let service = ChatService::new(provider_arc, toolset);
let mut round = service.run_round(&mut chat, "用户输入", cancel_token)?;
while let Some(event) = round.next().await { /* 渲染 LoopEvent */ }
```

两条铁律（改这里之前必读）：

1. **只在 `End` 后入史**：增量攒在 `TurnDraft`，取消 / 出错丢弃即回滚（截断是 `End{Truncated}`，照常入史——那是用户看到的输出）。
2. **工具错误喂回模型**：`tool_result` 内容 = `error: …`；不做流中重试（重试属 `A6`），死循环由 `max_model_turns` 兜底。

## 相关

- 契约：`A2`、`A3` ｜ **轮次循环图 → `A5-V`** ｜ 规格：`A5`（工具调用）、`A7`（上下文与 Token）｜ 传输：`crates/kernel/transport`
- 组织决策：`D5`、`D6` ｜ UI 形态：`D3` ｜ 持久化：M5 `kernel/store`（`S2` Q8）
