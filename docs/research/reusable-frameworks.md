# X6 · 可复用框架评估（哪些自研可以换成现成库）

| | |
|---|---|
| ID | `X6` |
| 类型 | 调研（一次性消费，会过时） |
| 状态 | ✅ 完成 |
| 更新 | 2026-10-11 |
| 何时读 | 想减少自研、评估引入新依赖；`A6` 落地 / `M7` 开工前 |
| 规模 | ~2k token |

## TL;DR

- 前提纠正：**不是「全都自研」**。`reqwest` / `tokio` / `serde` / `rusqlite` / `ratatui` / `tracing` 等基础设施都已在用；真正自研的只有 **SSE 解析、Provider 契约、业务语义**三类 —— 这是 `G1` §3「自研边界」的刻意选择，不是没找到轮子。
- **建议引入**（高收益、低成本，全在**装配层**）：`clap` 换手写 CLI 解析；`rustyline` 换裸 stdin REPL；`backoff` 换尚未实现的退避（`A6` ⬜）。
- **值得评估**：`M7`（MCP）开工前 spike `rmcp`，复查 `D8` —— 此时改主意成本为零。
- **不建议更换**：SSE 解析、Provider 契约、上下文裁剪、路由兜底 —— 自研各有明确理由（§4）。
- 结论不触动任何 `kernel` / `features` 契约：建议项只改 `app` 装配层与一处未实现模块。

## 1. 现状盘点

| 层 | 现状 | 判定 |
|----|------|------|
| HTTP / TLS | `reqwest` + rustls | ✅ 已用框架 |
| 异步 / 取消 / 超时 | `tokio` + `tokio-util` | ✅ 已用框架 |
| 序列化 / 错误 | `serde` / `serde_json` / `thiserror` | ✅ 已用框架 |
| 流式装配 | `futures` + `async-stream` | ✅ 已用框架 |
| 持久化 | `rusqlite`（bundled） | ✅ 已用框架 |
| TUI | `ratatui` + `crossterm` | ✅ 已用框架 |
| 可观测 | `tracing` | ✅ 已用框架 |
| SSE 解析 | 自研约 200 行（`kernel/transport`） | ⛔ 不换（§4） |
| Provider 契约 + 适配器 | 自研（`D2`） | ⛔ 不换（§4） |
| 上下文裁剪 / 路由 / 工具循环 | 自研（业务语义） | ⛔ 不换（§4） |
| **CLI 参数解析** | 手写循环（`crates/app`） | 🔧 建议换 `clap` |
| **REPL 行编辑** | 裸 stdin 读行（`crates/app`） | 🔧 建议换 `rustyline` |
| **重试 / 退避** | ⬜ 未实现（`A6`） | 🔧 建议用 `backoff` |
| **MCP 客户端** | ⬜ `M7` 未开工（`D8`） | 🔍 建议 spike `rmcp` |
| SQLite 迁移 | 手写约 60 行（`kernel/store`） | ⚪ 可换 `rusqlite_migration`，收益低 |
| 工具 JSON Schema | 手写 `json!` 字面量 | ⚪ 可换 `schemars`，工具多再做 |
| token 计数 | 估算 ≈4 字符/token | ⚪ 保持估算（`tiktoken-rs` 增体积） |

## 2. 建议引入（高收益、低成本）

| # | 候选 | 现状与位置 | 收益 | 代价 |
|---|------|-----------|------|------|
| A1 | **`clap`（derive）** | `crates/app` 手写 `while` 循环解析（约 40 行）+ 两处硬编码 usage | 声明式声明参数；**自动 `--help` / 版本 / 彩色错误提示**；支持 `--flag=value`、参数乱序；加子命令（如 MCP server 清单）近零成本；代码量降到约 15 行 | 新增一个依赖（CLI 事实标准） |
| A2 | **`rustyline`** | `crates/app/src/repl.rs` 用 `stdin.read_line` 裸读 | 方向键历史、行内编辑、Ctrl-R 搜索 —— 现在按方向键会打印 `^[[A`；REPL 是 M4 后每天用的入口 | 新增一个依赖；但**同步 API 不变**，当前「blocking 线程 + channel」架构原样保留，只换循环体 |
| A3 | **`backoff`** | `kernel/transport` 的 `retry`（指数退避 + 抖动）**尚未实现**（`A6` ⬜） | 退避 + 抖动是纯算法，`ExponentialBackoff` 直接可用；省约 100 行与边界 bug | 新增一个依赖；语义若不贴合（如要按 `ErrorCategory` 分级），仍可与 `A6` 策略层组合，不冲突 |

排序：**A1 → A2 → A3**。A1 / A2 都只改 `crates/app` 装配层，不触碰任何契约，风险最低、可独立验证。

## 3. 值得评估：`rmcp`（`M7` 开工前）

`D8` 决策「自研最小 MCP 客户端（stdio + tools only）」，理由与 `D2` 一脉相承：**需求面（三个方法一问一答）与依赖面（整棵 SDK 依赖树）严重不对称**。此结论无需推翻，但有两点使其值得**复查**：

1. `D8` 落于 **2026-10-07**，并把 `rmcp` 明确列为「逃生通道」； `M7` 至今**未开工**（见 `S1`），此刻改主意成本为零。
2. 建议动作：`M7` 动手前做一次半天 spike，把 `rmcp`（client + stdio + tools）的**实测依赖树大小 / 编译时间**与「自研 300–500 行」并列对比后拍板。

判定口径（与 `G1` §3 一致）：若 `rmcp` 已足够稳定，则自研的唯一收益退化为「零重依赖」；若否则维持 `D8`。**结论若与 `D8` 相悖，须另写一条新 ADR（旧的不改）并在 `S2` 登记。**

## 4. 不建议更换（自研有理）

| 自研点 | 位置 | 理由 |
|--------|------|------|
| SSE 帧解析 | `kernel/transport` | 自研理由写在模块头：`eventsource-stream` 自带浏览器 `EventSource` 的自动重连语义、与本层「纯解析 + 自收尾」不贴合；`sse-rs` 版本太新仍要包装。自研约 200 行且测试完备，关键是保留了 `A2` §4 要的「**EOF 截断不当成功**」精确语义 |
| Provider 契约 + 适配器 | `kernel/providers` | `D2` 核心决策：契约所有权在自己、不受上游版本节奏绑架；`rig` / `genai` 仅作参考实现 |
| 上下文裁剪 | `features/chat` | 依赖「工具调用与结果配对同生共死」这一项目特有业务不变量，无库可表达 |
| 路由与兜底 | `kernel/routing` | 同上，能力协商 + 降级链是业务语义 |
| GPUI ↔ tokio 桥 | 待建（`X2`） | 第三方均 0.1–0.3 版，自研约 50 行更稳（`X2` 已结论） |

## 落地清单

- [ ] A1：`crates/app` 引入 `clap`，替换 `parse_session_args` 与硬编码 usage（`chat` / `tui` / `self-check` 三个子命令）。
- [ ] A2：`crates/app/src/repl.rs` 用 `rustyline` 替换裸 stdin 读行，保留阻塞线程 + channel 结构。
- [ ] A3：`A6` 落地时，`kernel/transport` 的重试直接采用 `backoff`，不自行实现退避。
- [ ] B1：`M7` 开工前 spike `rmcp`，与自研方案对比后决定是否复查 `D8`。
- [ ] B2（可选）：工具数量增多后，用 `schemars` 从 Rust 类型派生工具 JSON Schema。
- [ ] B3（可选）：如需 down 迁移 / 迁移文件组织，评估 `rusqlite_migration`。

## 相关

- 自研边界判据 → `G1` §3
- 已定技术路线（栈 / SSE / 存储 / 密钥） → `X1`；UI / GPUI → `X2` / `X4`；样式 → `X5`
- 契约与适配器决策 → `D2`；MCP 决策 → `D8`
- 重试 / 退避落地归属 → `A6`；MCP 里程碑 → `RM` M7
- 未决项 → `S2`
