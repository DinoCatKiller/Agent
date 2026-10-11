# agent-telemetry · 可观测性与成本

| | |
|---|---|
| 状态 | ✅ `R3` 骨架落地（2026-10-11）：tracing 初始化 + token 成本计算 |
| 边界 | **机制**：日志怎么初始化、成本怎么算。**不含**「在哪儿打点」（散在各自 crate）与远程上报 |
| 上游 | `agent-common`（`Usage` / `Pricing` 契约类型） |
| 下游 | `app`（启动时初始化）；各 `kernel/*` 打点 |
| 何时读 | 加日志、排查「这轮花了多少」，或想接 OTel 时 |

## 这个 crate 是什么

可观测性有三件事，本 crate 只做其中「机制」的两件：

| 线 | 谁做 |
|----|------|
| 日志初始化 | 本 crate（`init_from_env`） |
| 打点（span / event） | **各 crate 自己**（`kernel/*` / `features/*`），本 crate 只给约定 |
| 成本计算 | 本 crate（`cost_of` 纯函数） |

## 核心内容

| 项 | 说明 |
|----|------|
| `init_from_env()` | 装 `tracing-subscriber`：级别读 `RUST_LOG`（缺省 `info`），输出 **stderr**；幂等 |
| `cost_of(usage, pricing)` | 纯函数：`输入 tokens × 单价 + 输出 tokens × 单价`；**没有定价返回 `None`，不猜** |

## 三个必须知道的约束

1. **密钥永不进日志**（`R1` §4 / 硬约束 5）。
2. **不打印 prompt 全文**（可能含用户隐私）—— 只记长度 / 条数这类统计。
3. **本地优先**：不自动上传、不接 OTel（`G1` §2 留作按需），日志也不写 stdout（会污染程序自身输出）。

## 相关

- 规格 → `R3` ｜ 契约类型 → `A2`（`Usage` / `Pricing`）｜ 分层判据 → `D6` §1
- 使用方：`crates/app`（启动初始化）｜ 地图 → [`AGENTS.md`](../../../AGENTS.md) §2
