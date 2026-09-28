# S1 · 当前阶段与下一步

| | |
|---|---|
| ID | `S1` |
| 类型 | 状态 |
| 更新 | 2026-09-28 |
| 何时读 | **每次开工第一眼**，5 秒确认「现在到哪」 |
| 规模 | ~1k token |

> **固定路径，就地更新，永不搬家。** 阶段变化只改本文件内容，不移动任何文件。

## TL;DR

- **阶段**：M1 完成 —— workspace 建好，契约层（`agent-schema` + `Provider` trait）已落地，4 个单测 + clippy 零告警。
- **下一步**：**M2 = `agent-transport`**（reqwest + SSE 解析 + 超时/取消 + 退避重试）。
- **卡点**：不影响 M2–M4；**M5（UI）前必须拍板 `D3`**，**M3 前需定 `S2` Q3（先接哪家）**。

## 里程碑

| # | 里程碑 | 状态 |
|---|--------|------|
| M0 | 文档体系与关键决策定稿 | ✅ 文档体系完成；`D3` / `D4` 待拍板 |
| M1 | cargo workspace + 契约层落地 | ✅ 3 个 crate、4 个单测、clippy 零告警、`self-check` 通过 |
| M2 | `agent-transport`：reqwest + SSE 解析 + 超时/取消 + 退避 | ⬜ **下一步** |
| M3 | 首个 adapter（OpenAI 兼容族）+ 契约测试 fixtures | ⬜ |
| M4 | `agent-core`：单轮对话 + 流式贯通（CLI 验证） | ⬜ |
| M5 | UI 层（形态由 `D3` 定）+ 会话持久化 | ⬜ 被 `D3` 阻塞 |
| M6 | 工具调用 + 模型路由与兜底 | ⬜ |

## 已完成的具体产物

| 产物 | 位置 |
|------|------|
| cargo workspace（3 crate） | `Cargo.toml`、`crates/*` |
| 契约类型（消息/请求/响应/事件/错误/能力） | `crates/agent-schema/src/*`（门面：`crates/agent-schema/README.md`） |
| `Provider` trait + 类型擦除 + 注册表 + 能力推导 | `crates/agent-providers/src/lib.rs`（门面：`crates/agent-providers/README.md`） |
| 契约冒烟入口 | `cargo run -p agent-app -- self-check` |
| 契约规格 / 消息协议 / 测试策略 / 供应商模板 | `A2` / `A3` / `Q1` / `PT` |

## 下一步（M2 待办清单）

1. 建 `crates/agent-transport`，同步建它的 `README.md` 并在 `AGENTS.md` §2 登记。
2. `reqwest` client 封装：连接/读超时、流式空闲超时、代理、rustls。
3. SSE 解析：`bytes_stream` → 事件；先评估 `eventsource-stream` / `sse-rs`，不满足则自研（200–400 行，有停止条件）。
4. 实现三分收尾语义（`A2` §4）：**传输错误 / 可恢复坏帧 / EOF 截断**；截断绝不允许静默当成功。
5. 取消：把 `CallContext.cancel` 接进请求与流。
6. 退避重试：只包 `ProviderError::retryable == true` 的情况，指数退避 + 抖动。
7. 按 `Q1` 的 12 条用例补齐 fixtures（重点是 5 截断、6 坏帧、11 无 usage）。

## 决策速查

| ID | 结论 | 状态 |
|----|------|------|
| `D1` | 技术栈 = Rust + tokio（UI 待复核） | ✅ 已接受 |
| `D2` | 多模型抽象 = 契约自研 + 实现可换 | ✅ 已接受 |
| `D3` | UI 交付形态 | 🚧 待决（阻塞 M5） |
| `D4` | 样式层路线（token 桥 + 组件层） | 🚧 待确认（阻塞 M5） |

## 相关

- 未决问题 → `S2`
- 地图 → `AGENTS.md` §2
