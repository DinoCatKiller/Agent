# agent-chat · 功能切片：对话

| | |
|---|---|
| ID | —（功能切片，非索引文档） |
| 状态 | ✅ M4 落地：状态机 + 轮次编排（事件流）+ 工具边界 + 上下文裁剪；✅ M5 落地 `repo.rs` 会话持久化 + `ui.rs` 界面（一期 TUI 的对话侧）；✅ M6 落地工具并行执行（保序回填）；✅ 二期 GPUI 渲染端 `gpui.rs` + 通道契约 `channel.rs`（`gui` feature，2026-10-12）；✅ **Ely 组件重构**（2026-10-21：`Sidebar` / `MessageList` / `MessageBubble` / `PromptInput`，顺带解决 `G-3`） |
| 边界 | **语义**：一次对话怎么进行、状态怎么变、界面长什么样。含工具调用编排、上下文管理、业务表结构与界面状态机 |
| 依赖 | `agent-common`（契约）｜ `agent-providers`（`Provider` / `CallContext`）｜ `agent-store`（连接 / 迁移机制）｜ `ratatui`（仅 `ui.rs` 渲染端）｜ `agent-ui-kit`（仅 `gui` feature，`gpui.rs` 渲染端 + Ely 组件 / 令牌主题）｜ `jiff`（仅 `gui` feature，会话条目的时间戳） |
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
  channel.rs    UI 无关的 Cmd / UiEvent 契约（两个渲染端共用，D3）
  gpui.rs       二期的界面 —— ChatView（GPUI 渲染端，复用同一个 ChatUi；仅 gui feature）
                组件全部来自 Ely：Sidebar（会话栏）+ MessageList（虚拟列表）+ MessageBubble
                + StreamingMarkdown + PromptInput + Popover（模型选择器）
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

**二期 GPUI（`gui` feature）**：`ChatView` 消费同一套 `Cmd` / `UiEvent`（`channel.rs`），
内部仍是同一个 `ChatUi`——两个渲染端的回滚、提交边界、工具状态因此完全一致。
运行：`cargo run -p agent-app --features gui -- gui --provider … --model …`。

渲染端**不自造组件**（Ely 覆盖了这里需要的全部形态），要点三条：

- **会话栏**：`Sidebar` 只接受子元素（不实现 `Styled`），内层用普通 `div` 排布；
  `ConversationList` 的分组标题硬编码英文，所以这里只用 `ConversationItem`、自己按
  今天 / 昨天 / 最近 7 天 / 更早分组。`Conversation.at` 收 `jiff::Timestamp`。
- **转录**：`MessageList` 是**虚拟列表**（`G-3` 的解），条目数给一次、闭包按索引渲染；
  正文走 `StreamingMarkdown`，角色映射到 `Role::{User, Assistant, Error}`（Ely 没有 Tool）。
- **输入区**：`PromptInput` 自带 Enter 提交与 busy→stop 的按钮语义；模型选择器用
  `Popover::with_opener` 自持开关（不再手写 `model_menu_open` 标志位）。

字号：`Theme::text_size(TextSize)` 返回 `Rems`，一帧取一次（`gpui.rs` 的 `Type`）再传给元素，
不要在每个元素里现算。

**运行中切模型**（`channel.rs` 的 `Cmd::SelectModel` / `UiEvent::ModelChanged`）：
按 **(provider, model) 组合**切，不是只切模型——密钥按 provider 分别存（`R1`），
换 provider 必须连带重建 provider 实例。两条约定：

- **生成中不许切**（`busy` 时回一条 `Notice`）：切在「轮边界」上，与 `A` 方案的语义一致；
- **界面不做乐观更新**：状态栏与选择器等 worker 回播 `ModelChanged` 才变，
  这样「缺密钥导致切不过去」时不会先显示成成功。

候选清单由宿主给（[`ModelChoice`]）：`features/*` 不认识供应商名单，那是装配层的知识（`D6`）。

**联网刷新模型清单**（`channel.rs` 的 `Cmd::RefreshModels` / `UiEvent::ModelsRefreshed`，`C2`）：
宿主 worker 拿到 `RefreshableModels` 句柄去拉 `GET /models`，成功后把新的候选重算并推给对话页。
失败走单独的 `UiEvent::ModelsRefreshFailed`（`C2d`）——与泛化的 `Notice` 分开，好让**发起刷新的设置页**
能就地报错；**失败不清空现有清单**。
本模块只负责**报提示**（`Notice` / 状态栏文案）——「刷新怎么拉、写回哪里、候选怎么算」都是装配层的
知识（`D6`：跨功能的线与供应商细节不进 `features/*`）。

**设置保存后立即生效**（`channel.rs` 的 `Cmd::ApplySettings` / `UiEvent::SettingsApplied`，B 方案，
2026-10-25）：设置页保存成功后宿主发 `ApplySettings`，worker 用**当前组合**重建 provider
（重读磁盘的密钥 / 端点 / 代理 / 超时）。本模块只提供契约与兜底提示——「保存」与「发起」在
`features/settings`，重建在 `app` 的 worker。**生成中该命令排队**（worker 轮内不处理命令，
轮末自动生效）；`SettingsApplyFailed` 时旧连接原样保留。

两条铁律（改这里之前必读）：

1. **只在 `End` 后入史**：增量攒在 `TurnDraft`，取消 / 出错丢弃即回滚（截断是 `End{Truncated}`，照常入史——那是用户看到的输出）。`ui.rs` 的显示回滚与它对齐：`ModelTurnEnded` 是提交边界。
2. **工具错误喂回模型**：`tool_result` 内容 = `error: …`；不做流中重试（重试属 `A6`），死循环由 `max_model_turns` 兜底。

## 相关

- 契约：`A2`、`A3` ｜ **轮次循环图 → `A5-V`** ｜ 规格：`A5`（工具调用）、`A7`（上下文与 Token）｜ 传输：`crates/kernel/transport`
- 组织决策：`D5`、`D6` ｜ UI 形态：`D3`（一期 TUI）、`D4`（令牌）｜ 持久化机制：`crates/kernel/store`（表结构在 [`repo`](src/repo.rs)）
