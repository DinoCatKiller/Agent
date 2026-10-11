# R3 · 可观测性与成本

| | |
|---|---|
| ID | `R3` |
| 类型 | 规格（归属 `agent-telemetry`） |
| 状态 | ✅ 骨架落地（2026-10-11） |
| 更新 | 2026-10-11 |
| 何时读 | 加日志 / span / 计费；排查「这轮花了多少 token、多少钱」时 |
| 规模 | ~1.5k token |

## TL;DR

- **日志**：`tracing`；级别读 `RUST_LOG`（缺省 `info`）；输出 **stderr**；`app` 启动时调 `agent_telemetry::init_from_env()` 装 subscriber（幂等）。
- **一次模型调用 = 一个 span**（§2）。本 crate 只给「初始化 + 字段约定」，**打点在各自的 `kernel/*` / `features/*`**。
- **成本**：`cost_of(usage, pricing)` 纯函数，价格来自 `ModelSpec::pricing`（`A2` §1）；**没有定价返回 `None`（不猜）**。
- **本地优先**：不自动上传、不接 OTel、不打印 prompt 全文与密钥。

## 1. 日志

| 项 | 约定 |
|----|------|
| 库 | `tracing`（`info!` / `debug!` / `warn!` / `error!`） |
| 初始化 | `agent_telemetry::init_from_env()` —— `app` 启动时调一次；已装过则直接返回 |
| 级别 | `RUST_LOG` 环境变量，缺省 `info`（例：`RUST_LOG=agent_transport=debug,info`） |
| 输出 | **stderr** —— stdout 留给程序自身的输出（见 §4） |
| target | 统一用 crate 名（`agent_transport` / `agent_config` / `agent_routing`…），便于按模块过滤 |

## 2. 一次调用的 span

span 名 `model_call`：

| 字段 | 来源 | 何时写 |
|------|------|--------|
| `provider` | 适配器 id / `ProviderError.provider` | 进入时 |
| `model` | `ModelRequest.model` | 进入时 |
| `request_id` | `CallContext.request_id` | 进入时（可空） |
| `turn` | 编排层轮次序号 | 进入时 |
| `input_tokens` / `output_tokens` | `Usage` | 收到 `Usage` 时 |
| `latency_ms` | 单调时钟差值 | 收尾时 |
| `cost` | `cost_of(...)` 的结果 | 收尾时（无定价则省略该字段） |
| `outcome` | `ok` / `error:<category>` / `cancelled` | 收尾时 |

**打点位置**：request/response 边界只有适配器与 `kernel/transport` 知道，所以 span 落在那两层；`turn` 由 `features/chat` 补。当前实现只在关键路径留了 `debug!`，完整 span 随实际排查需要再铺（见落地清单）。

## 3. 成本

```rust
cost_of(&usage, spec.pricing.as_ref()) -> Option<f64>   // 与 pricing.currency 同币种
```

- 公式：`input_tokens / 1e6 × input_per_mtok + output_tokens / 1e6 × output_per_mtok`。
- **无定价 → `None`**：界面上显示「未知」。**不要**用 `0` 或估算值冒充 —— 那会让 UI 说谎。
- 累计口径：单轮 = 该轮各次调用之和（`features/chat` 的 `RoundEnded` 已带 `round_usage`）；会话累计由 `Chat::total_usage` 持有。

## 4. 明确不做

| 不做 | 原因 |
|------|------|
| OpenTelemetry | `G1` §2 列为「后续按需」；先把 `tracing` 一条线用顺 |
| 远程上报 / 遥测回传 | 本地优先（`G1` §1.1）——用户数据不出机器 |
| 打印 prompt 全文 / 密钥 | 隐私与安全（`R1` §4、硬约束 5）；只记长度、条数这类统计 |
| 往 **stdout** 写日志 | 会污染 `self-check` 等程序输出，也会打乱管道用法 |

## 落地清单

- [x] `kernel/telemetry` 骨架：`init_from_env()` + `cost_of()`（4 个用例：双向计价 / 不足百万 / 无定价为 `None` / 零用量）。
- [x] `app` 启动时初始化日志。
- [ ] `model_call` span 的完整打点（字段见 §2）—— 先保持 `debug!`，随排查需要铺开。
- [ ] 会话级成本汇总进 TUI 状态栏（`features/chat` + `app`）。

## 相关

- 选型（`tracing` + `tracing-subscriber`） → `X1` §1 ｜ 契约类型（`Usage` / `Pricing`） → `A2`
- 密钥与日志红线 → `R1` §4 ｜ 门面 → `crates/kernel/telemetry` 的 `README.md` ｜ 未决项 → `S2`
