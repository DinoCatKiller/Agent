# S1 · 当前阶段（概览）

| | |
|---|---|
| ID | `S1` |
| 类型 | 状态 · **概览** |
| 更新 | 2026-10-07 |
| 何时读 | **每次开工第一眼**，5 秒确认「现在做什么」 |
| 规模 | ~0.5k token |

> **本文件只写当前阶段的任务。** 任务完成 → 清空并重写；历史交给 git，长期排期见 `RM`。
> 任务跨多个 crate 时：**这里只给概览**，细节写在相关 crate 的 `status.md` 里，并从下表链过去。

## 当前任务

**M6 · 工具调用 + 路由兜底** ｜ 状态：🚧 待开工

一句话：工具执行从顺序改并行（`A5` 的 M6 部分）；新建 `kernel/routing`——模型注册表查询、能力协商、失败降级链（`R2`）。

| crate | 本任务产出 | 细节 |
|-------|-----------|------|
| `crates/features/chat` | 多工具并行执行 + 编排层路由接入 | crate [`README.md`](crates/features/chat/README.md) |
| `crates/kernel/routing` | ⬜ 新建 crate（记得同步建 `README.md` 并回填 `AGENTS.md` §2 地图，硬约束 7） | — |

## 阻塞

无。

## 上一阶段

- **M5 完成（2026-10-07）**：UI 层 + 会话持久化——`kernel/store`（连接/迁移/事务）+ `features/chat/repo.rs`（`Chat` 原样 JSON 落盘）+ 一期 TUI（`agent-app tui`：会话侧栏 / 流式转录 / Ctrl-C 取消 / 轮末落盘，配色出自 `design/tokens.json`）。断网全绿（134 例，含 worker 协议端到端 3 例），`clippy -D warnings` 干净；**验收演示已通过（2026-10-07，真实 API 对话验证）**，落盘与重启恢复另经 winpty 假服务器冒烟确认。

## 下一阶段

- 择期：`S2` 待确认项（Q4 MCP 客户端、Q6 签名、Q7 正式名称）。

## 相关

- 里程碑与排期定义 → `RM` ｜ 未决问题 → `S2` ｜ 地图 → [`AGENTS.md`](AGENTS.md) §2
