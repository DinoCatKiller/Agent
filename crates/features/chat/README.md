# agent-chat · 功能切片：对话

| | |
|---|---|
| ID | —（功能切片，非索引文档） |
| 状态 | ✅ M4 落地：状态机 + 轮次编排（事件流）+ 工具边界 + 上下文裁剪；✅ M5 落地 `repo.rs` 会话持久化 + `ui.rs` 界面（一期 TUI 的对话侧）；✅ M6 落地工具并行执行（保序回填） |
| 边界 | **语义**：一次对话怎么进行、状态怎么变、界面长什么样。含工具调用编排、上下文管理、业务表结构与界面状态机 |
| 依赖 | `agent-common`（契约）｜ `agent-providers`（`Provider` / `CallContext`）｜ `agent-store`（连接 / 迁移机制）｜ `ratatui`（仅 `ui.rs` 渲染端） |
| 不允许 | 依赖 `app`；被 `kernel/*` 依赖；**自己打开 / 管理连接**（HTTP 客户端与 SQLite 连接是机制，属 `kernel`） |
| 何时读 | 改对话逻辑、加对话能力、写对话界面时 |

## 内部布局（`D6` §3）

```
src/
  lib.rs        模块声明 + 对外导出
  chat.rs       状态机：Chat（历史 + 累计用量）+ TurnDraft（流式累积，只在 End 后入史）
  service.rs    轮次编排：run_round 事件流（Delta/Tool/ModelTurnEnded/RoundEnded/Cancelled/Error）；
                M6 工具并行——Started 全发 → 完成即报 Finished → 全部完成后按调用序回填（A5 §2）
  tools.rs      工具执行边界：Tool trait + ToolSet 注册表（实现由宿主提供）
  context.rs    上下文裁剪：交换块整块丢弃，保 tool 配对与当前轮（A7）
  repo.rs       本功能的持久化 —— SessionRepo：会话表 + save/load/list/rename/delete（Chat 原样 JSON 落盘）
  ui.rs         本功能的界面 —— ChatUi 状态机（吃 LoopEvent，无 ratatui 可测）+ ratatui 渲染端
                + Theme（design/tokens.json 令牌，D4 一期）
tests/round.rs  编排集成测试（假 provider，9 例，断网）
docs/           规格：A5（工具调用）、A7（上下文与 Token）
```

## 核心用法（消费者见 `crates/app/src/repl.rs` 与 `crates/app/src/tui.rs`）

```rust
let service = ChatService::new(provider_arc, toolset);
let mut round = service.run_round(&mut chat, "用户输入", cancel_token)?;
while let Some(event) = round.next().await { /* 渲染 LoopEvent */ }
```

UI 的接入约定（`D3`「事件流 + 命令通道」）：`ChatUi::on_event` 消费 `LoopEvent`，
`ChatUi::load_chat` 用 `SessionRepo::load` 的结果整体替换显示；
状态机与渲染端分离，二期换 GPUI 只换渲染端。

两条铁律（改这里之前必读）：

1. **只在 `End` 后入史**：增量攒在 `TurnDraft`，取消 / 出错丢弃即回滚（截断是 `End{Truncated}`，照常入史——那是用户看到的输出）。`ui.rs` 的显示回滚与它对齐：`ModelTurnEnded` 是提交边界。
2. **工具错误喂回模型**：`tool_result` 内容 = `error: …`；不做流中重试（重试属 `A6`），死循环由 `max_model_turns` 兜底。

## 相关

- 契约：`A2`、`A3` ｜ **轮次循环图 → `A5-V`** ｜ 规格：`A5`（工具调用）、`A7`（上下文与 Token）｜ 传输：`crates/kernel/transport`
- 组织决策：`D5`、`D6` ｜ UI 形态：`D3`（一期 TUI）、`D4`（令牌）｜ 持久化机制：`crates/kernel/store`（表结构在 [`repo`](src/repo.rs)）
