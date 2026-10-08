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

**M7 · MCP 客户端 + 定名落地** ｜ 状态：🚧 待开工

一句话：按 `D8` 自研最小 MCP 客户端（client / stdio / tools only）——`kernel/mcp` 协议机制 + `app` 装配适配进 `ToolSet`，端到端能列出并调用一个 stdio MCP server 的工具。

| 事项 | 产出 | 细节 |
|------|------|------|
| `crates/kernel/mcp` | ⬜ 新建 crate：JSON-RPC over stdio + `initialize` / `tools/list` / `tools/call`（记得同步建 README 并回填 `AGENTS.md` §2，硬约束 7） | 决策 `D8` |
| `crates/app` | MCP 工具适配器（`McpTool → Tool`）+ server 装配入口（`R1` 前先用 CLI 参数/默认清单） | `A5` |
| 定名落地 | 产品名 **CodingRocket**（`S2` Q7）：README / 横幅已改；crate 包名（`agent-*`）是否全量改在本任务内拍板执行 | — |

## 阻塞

无 —— `S2` 未决项已全部清零（Q6 按建议闭环：本期不自签不公证、30MB 警戒线）。

## 上一阶段

- **M6 完成（2026-10-07）**：工具调用 + 路由兜底——`features/chat` 工具并行执行（`A5` §2）；新建 `kernel/routing`（`R2`）：`Router` 实现 `Provider` 契约，能力协商 fast-fail + 失败降级链，`app` 以 `--model a,b` 暴露。断网全绿，wiremock 验证 429 → 备用模型接管。

## 相关

- 里程碑与排期定义 → `RM` ｜ 未决问题 → `S2` ｜ 地图 → [`AGENTS.md`](AGENTS.md) §2
