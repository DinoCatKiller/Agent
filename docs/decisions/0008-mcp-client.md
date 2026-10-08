# ADR-008：MCP 客户端——自研最小接入

| | |
|---|---|
| ID | `D8` |
| 类型 | 决策（**只增不改**） |
| 状态 | **已接受** |
| 更新 | 2026-10-07 |
| 何时读 | 接 MCP、扩工具生态、质疑要不要引 SDK 时 |
| 规模 | ~1k token |

## 背景

`S2` Q4：是否支持 MCP（Model Context Protocol——让模型调用外部工具的开放协议，JSON-RPC 2.0 over stdio/HTTP）。接入后 `ToolSet` 可挂接 MCP 生态的现成 server（文件、搜索、数据库……），工具面不再局限于手写 Rust 工具。

现状约束：Rust 侧没有稳定依托——`rig` 0.42 已移除 `rmcp`（官方 SDK）依赖；而本项目的工具边界（`A5`：`Tool` trait + 宿主提供实现）在 M4 就已就绪，接入点干净。需要拍板：接不接、自研还是引 SDK、范围多大。

## 备选方案

| 方案 | 说明 | 优点 | 缺点 |
|------|------|------|------|
| A：依赖 `rmcp`（官方 Rust SDK） | 传输 / 握手 / 类型全现成 | 协议覆盖全（resources / sampling…） | pre-1.0、依赖树重（tower 等）；契约所有权在外部；本项目只用到客户端 + stdio + 3 个方法 |
| B：**自研最小 MCP 客户端（stdio）** | 自己实现 JSON-RPC + `initialize` / `tools/list` / `tools/call` | 需求面极小（约 300–500 行）；零重依赖（serde_json + tokio 进程 IO）；对齐 `D2`（契约自研、实现可换） | 传输层自己养；HTTP/SSE transport 将来自己加 |
| C：不做 MCP（现状） | 宿主直接注册 Rust 工具 | 零成本 | 放弃 MCP 工具生态，每个工具手写 |

## 决定

采用**方案 B**，范围（本期）刻意收窄：

1. **仅 client**（不做 server）；**仅 stdio transport**（本地 MCP server 子进程）；**只要 tools**（`tools/list` / `tools/call`），不做 resources / sampling / roots。
2. MCP server 的工具适配成 `Tool` trait 实现（定义 → `ToolDefinition`，调用 → `tools/call`），插入 `ToolSet`——**「宿主提供实现」的边界不动**（`A5`），MCP 只是新的实现来源。
3. 落点按 `D6`：协议与传输是机制 → 新建 `kernel/mcp`；`McpTool → Tool` 的适配与装配在 `app`（kernel 不依赖 features）。
4. `rmcp` 留作**逃生通道**：将来需要 HTTP/SSE transport 或更宽能力面时再评估引入，引法同 `D2`（包在 trait 背后作实现细节）。

最关键的理由：**需求面与依赖面严重不对称**——我们要的只是三个方法的一问一答，引一个 SDK 换来的是整棵 pre-1.0 依赖树。

## 影响

- 正面：工具生态打开；`ToolSet` 的契约与测试（`Q1`）零改动。
- 代价 / 后续动作：新建 `kernel/mcp`（README + 规格随 M7 落地）；MCP server 清单的配置来源 → 随 `R1`；server 可信度与安全边界 → `Q3`（安全文档，⬜）。
- 排期：进 `RM` 为 **M7**（完成标准：REPL/TUI 能列出并调用一个 stdio MCP server 的工具，端到端）。

## 未决项

server 清单格式与安全边界 → 随 `R1` / `Q3`；此处不另列。
