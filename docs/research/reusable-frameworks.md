# X6 · 可复用框架评估（哪些自研可以换成现成库）

| | |
|---|---|
| ID | `X6` |
| 类型 | 调研（一次性消费，会过时） |
| 状态 | ✅ 完成（A1–A3 已落地；B1 `rmcp` spike 已完成，2026-10-11） |
| 更新 | 2026-10-11 |
| 何时读 | 想减少自研、评估引入新依赖；`A6` 落地 / `M7` 开工前 |
| 规模 | ~2k token |

## TL;DR

- 前提纠正：**不是「全都自研」**。`reqwest` / `tokio` / `serde` / `rusqlite` / `ratatui` / `tracing` 等基础设施都已在用；真正自研的只有 **SSE 解析、Provider 契约、业务语义**三类 —— 这是 `G1` §3「自研边界」的刻意选择，不是没找到轮子。
- **已落地**（2026-10-11）：`clap` 替换手写 CLI 解析；`rustyline` 替换裸 stdin REPL（非 TTY 回退保留）；`backoff` 落地 `kernel/transport` 的 `retry` 模块（可取消的指数退避 + 抖动）。三项都在**装配层 / 机制层**，未触动任何契约。
- **已评估（2026-10-11）**：`rmcp` spike 已完成（档 1，见 §3）—— 它已是 **3.5.1**、最小配置净新增约 20 个包，`D8` 的两条主要论据（pre-1.0 / 依赖重）已不成立。**`M7` 暂缓**（MCP 使用频率不高，排到后面）；是否改判 `D8` 留到开工时决定。
- **不建议更换**：SSE 解析、Provider 契约、上下文裁剪、路由兜底 —— 自研各有明确理由（§4）。
- 落地清单见文末。

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
| **CLI 参数解析** | `clap`（`crates/app`） | ✅ 已换（A1） |
| **REPL 行编辑** | `rustyline`（`crates/app`） | ✅ 已换（A2） |
| **重试 / 退避** | `kernel/transport::retry`（`backoff`） | ✅ 已换（A3）；流式重试 / 降级随 `A6` |
| **MCP 客户端** | ⬜ `M7` 未开工（`D8`） | 🔍 建议 spike `rmcp` |
| SQLite 迁移 | 手写约 60 行（`kernel/store`） | ⚪ 可换 `rusqlite_migration`，收益低 |
| 工具 JSON Schema | 手写 `json!` 字面量 | ⚪ 可换 `schemars`，工具多再做 |
| token 计数 | 估算 ≈4 字符/token | ⚪ 保持估算（`tiktoken-rs` 增体积） |

## 2. 已落地（原「建议引入」，2026-10-11 实施）

| # | 候选 | 原落点与现状 | 收益 | 代价 |
|---|------|-------------|------|------|
| A1 | **`clap`（derive）** | `crates/app` 原手写 `while` 循环解析（约 40 行）+ 两处硬编码 usage | 声明式参数；**自动 `--help` / 版本 / 错误提示**；支持 `--flag=value`、参数乱序；加子命令近零成本 | 新增一个依赖（CLI 事实标准） |
| A2 | **`rustyline`** | `crates/app/src/repl.rs` 原用 `stdin.read_line` 裸读 | 方向键历史、行内编辑、Ctrl-R 搜索 —— 原实现按方向键会打印 `^[[A` | 新增一个依赖；**同步 API 不变**，原「blocking 线程 + channel」结构原样保留 |
| A3 | **`backoff`** | `kernel/transport` 的 `retry`（指数退避 + 抖动）原为 ⬜ 未实现 | 退避 + 抖动是纯算法，`ExponentialBackoff` 直接可用；省约 100 行与边界 bug | 新增一个依赖；控制流自持（接取消、按 `retryable` 判定），与 `A6` 策略层不冲突 |

实施要点：

- A1：`SessionArgs`（`chat` / `tui` 共用）+ `TuiCliArgs`（flatten + `--db`），`--model` 用 `value_delimiter` 承接逗号降级链。
- A2：TTY 用 `rustyline`，**非 TTY 回退裸 `read_line`**，脚本 / 管道输入不受影响；Ctrl-C 经共享取消槽取消当前轮（与 `tui` 同一套思路）。
- A3：`RetryPolicy` + `retry(policy, cancel, op)`；只重试 `retryable == true`，退避期间可取消。

## 3. `rmcp` 实测结论（spike 已完成，2026-10-11）

本节原为「`M7` 开工前待评估」；**档 1 spike 已做完**（临时工程 `tmp/rmcp-spike`，量完即删，主项目零改动）。实测：

| 指标 | baseline（自研依赖面 = `tokio` + `serde_json`） | `rmcp` 最小配置（`--no-default-features --features client,transport-child-process`） |
|------|--------------------------------------------|---------------------------------------------------------------------------------|
| 包总数 | 17（16 外部 + 根） | 56 |
| 相对基线 | — | 独有 38 个外部包 |
| 编译 | 18.8s（含首次下载） | **+25.9s 增量** |

三个发现：

1. **`rmcp` 已是 v3.5.1** —— `D8` 的「pre-1.0」论据**失效**（已跨过 1.0 到 3.x）。
2. **`tower` 在默认关闭的 feature 里**，最小配置不含它 —— `D8` 的「依赖树重（tower 等）」**不成立**。但默认 features 会白带 `server`，必须用 `--no-default-features` 收窄。
3. **净新增约 20 个包**（不是 38）—— 独有包里 19 个本项目已在用（`futures` 全家、`tracing`、`thiserror`、`serde`、`syn v2`、`tokio-util`）；真正新增的是 `chrono` / `process-wrap` / `indexmap` / `hashbrown` / `tokio-stream` / `slab` / `once_cell` / `equivalent` / `num-traits` + **`windows` 系列 10 个**（Win32 绑定，仅 Windows 真编译）。

API 表面（读 `rmcp-3.5.1` 源码）：`serve_client` + `TokioChildProcess` + `list_tools` + `call_tool`，正好覆盖 `D8` 要的三个方法与一种 transport；`rmcp::Tool` / `CallToolResult` 可映射到 `A5` 的 `ToolDefinition` / `ToolOutput`。

**处置**：`M7` 按决定**暂缓**（MCP 使用频率不高，排到后面）；`D8` **未改判** —— 真要开工时，以上数据即是否写新 ADR 推翻 `D8` 的依据（旧 ADR 不改）。

## 4. 不建议更换（自研有理）

| 自研点 | 位置 | 理由 |
|--------|------|------|
| SSE 帧解析 | `kernel/transport` | 自研理由写在模块头：`eventsource-stream` 自带浏览器 `EventSource` 的自动重连语义、与本层「纯解析 + 自收尾」不贴合；`sse-rs` 版本太新仍要包装。自研约 200 行且测试完备，关键是保留了 `A2` §4 要的「**EOF 截断不当成功**」精确语义 |
| Provider 契约 + 适配器 | `kernel/providers` | `D2` 核心决策：契约所有权在自己、不受上游版本节奏绑架；`rig` / `genai` 仅作参考实现 |
| 上下文裁剪 | `features/chat` | 依赖「工具调用与结果配对同生共死」这一项目特有业务不变量，无库可表达 |
| 路由与兜底 | `kernel/routing` | 同上，能力协商 + 降级链是业务语义 |
| GPUI ↔ tokio 桥 | 待建（`X2`） | 第三方均 0.1–0.3 版，自研约 50 行更稳（`X2` 已结论） |

## 落地清单

- [x] A1：`crates/app` 引入 `clap`（子命令 + `SessionArgs` / `TuiCliArgs`），取代 `parse_session_args` 与硬编码 usage。
- [x] A2：`crates/app/src/repl.rs` 用 `rustyline` 读入（历史 / 行内编辑 / Ctrl-R）；保留阻塞线程 + channel；**非 TTY 回退裸 `read_line`**。
- [x] A3：`kernel/transport` 落 `retry` 模块（`RetryPolicy` + `retry`，退避取自 `backoff`，可取消）；**流式重试与降级策略随 `A6`**。
- [x] B1（2026-10-11）：`rmcp` spike 已完成（档 1），结论见 §3 —— `D8` 的两条主要论据已不成立；`M7` 暂缓，是否改判留到开工时决定。
- [ ] B2（可选）：工具数量增多后，用 `schemars` 从 Rust 类型派生工具 JSON Schema。
- [ ] B3（可选）：如需 down 迁移 / 迁移文件组织，评估 `rusqlite_migration`。

## 相关

- 自研边界判据 → `G1` §3
- 已定技术路线（栈 / SSE / 存储 / 密钥） → `X1`；UI / GPUI → `X2` / `X4`；样式 → `X5`
- 契约与适配器决策 → `D2`；MCP 决策 → `D8`
- 重试 / 退避落地归属 → `A6`；MCP 里程碑 → `RM` M7
- app 门面（CLI / REPL 用法） → `crates/app` 的 `README.md`
- 未决项 → `S2`
