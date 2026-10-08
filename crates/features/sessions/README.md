# agent-sessions · 功能切片：会话管理

| | |
|---|---|
| ID | —（功能切片，非索引文档） |
| 状态 | ✅ M5 落地：`session.rs` 标题生成 + `ui.rs` 会话侧栏（一期 TUI 的左栏） |
| 边界 | **语义**：会话的语义与生命周期 —— 创建、列举、重命名、切换、删除 |
| 依赖 | `agent-common` ｜ `agent-chat`（`SessionMeta` / `Theme`；feature 间依赖，`D5`）｜ `ratatui`（仅 `ui.rs`） |
| 不允许 | 建连接池或事务机制；依赖 `app`；被 `kernel/*` 依赖；**直接持有数据库连接**（会话表的实现在 `features/chat` 的 `repo.rs`，`S2` Q8 闭环） |
| 何时读 | 改会话列表、会话切换、会话元信息时 |

## 内部布局（`D6` §3）

```
src/
  lib.rs        模块声明 + 对外导出
  session.rs    会话语义规则：标题自动生成 derive_title（chat 落盘时调用）
  ui.rs         本功能的界面：SessionPane 会话侧栏（列表 + 选择，纯数据状态 + ratatui 渲染）
tests/          单测（导航边界、刷新保选中、标题规则）—— 纯状态，断网可跑（Q1）
```

## 与 `chat` 切片的分工

- **持久化**：`SessionRepo`（表结构、SQL）住在 `features/chat/repo.rs`；本切片不碰 SQL。
- **数据流**：app 的 worker 每次轮末 `save` 后把 `SessionMeta` 列表喂给 `SessionPane::set_sessions`；
  切换会话由 worker `load` 完成后，把 `Chat` 交给 `agent_chat::ui::ChatUi::load_chat` 回放。
- **标题**：首次落盘时由 worker 调 [`session::derive_title`](src/session.rs) 生成（取首行、截 24 字）。

## 尚未落地（随需要再做）

- [ ] 重命名 / 删除的界面入口（`SessionRepo::rename` / `delete` 已备）
- [ ] 与 `kernel/store` 的端口 trait（当前列表数据由宿主注入，无直接存储依赖，暂不需要）

## 相关

- 组织决策：`D5`、`D6` ｜ UI 形态：`D3`（一期 TUI）、`D4`（令牌）｜ 契约：`A3`
- 会话持久化：`crates/features/chat` 的 [`repo.rs`](../chat/src/repo.rs) ｜ 存储机制：`crates/kernel/store`
