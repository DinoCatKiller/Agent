# S1 · 当前阶段（概览）

| | |
|---|---|
| ID | `S1` |
| 类型 | 状态 · **概览** |
| 更新 | 2026-09-30 |
| 何时读 | **每次开工第一眼**，5 秒确认「现在做什么」 |
| 规模 | ~0.5k token |

> **本文件只写当前阶段的任务。** 任务完成 → 清空并重写；历史交给 git，长期排期见 `RM`。
> 任务跨多个 crate 时：**这里只给概览**，细节写在相关 crate 的 `status.md` 里，并从下表链过去。

## 当前任务

**M5 · UI 层 + 会话持久化** ｜ 状态：🚧 进行中（2026-10-06 开工）

一句话：会话可存可取——`kernel/store`（rusqlite 连接 / 迁移 / 事务）落地，`features/chat/repo.rs` 提供会话 CRUD；一期 TUI（Ratatui）验证 Core（`D3`）。

| crate | 本任务产出 | 细节 |
|-------|-----------|------|
| `crates/kernel/store` | SQLite 连接管理、迁移、事务（rusqlite bundled） | crate [`README.md`](crates/kernel/store/README.md) |
| `crates/features/chat` | `repo.rs`：会话 CRUD（依赖 `agent-store`） | — |
| `crates/features/*` | 一期 TUI（Ratatui）`ui.rs`（`D3` 分期） | — |

## 阻塞

无 —— `S2` Q1（UI 形态）与 Q2（样式层）已拍板，`D3` / `D4` 已接受。

## 上一阶段

- **M4 完成**：CLI 单轮与多轮（含工具结果回填）对话——`agent-chat` 事件流编排 + `agent-app` REPL 与端到端，断网全绿。

## 下一阶段

- **M6 · 工具调用 + 路由兜底**：多工具并行、失败降级链生效（`features/chat`、`kernel/routing`）。

## 相关

- 里程碑与排期定义 → `RM` ｜ 未决问题 → `S2` ｜ 地图 → [`AGENTS.md`](AGENTS.md) §2
