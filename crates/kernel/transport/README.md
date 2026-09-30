# agent-transport · 传输层

| | |
|---|---|
| 状态 | ✅ M2 已收口：HTTP / SSE / 收尾 / 取消 + fixtures + `A4`（重试随 `A6`） |
| 边界 | HTTP 客户端封装、SSE 解析、超时 / 取消 / 退避重试。**不含**供应商字段映射（`agent-providers`）、路由策略（`kernel/routing`） |
| 上游 | `agent-common`（契约类型） |
| 下游 | `agent-providers` 的适配器 |
| 何时读 | 动 SSE / 中断 / 超时 / 重试前 |

## 这个 crate 是什么

负责把「一次模型调用」**真正发出去并流式收回来**：一次真实请求能流式收完，
且坏帧 / 截断 / 超时都有明确语义与用例覆盖。

## 核心内容（已落地；规格见 `A4`）

| 项 | 说明 |
|----|------|
| HTTP 封装 ✅ | `reqwest`（rustls）、连接 / 读超时、**流式空闲超时**（`IdleTimeout`）、代理 |
| SSE 解析 ✅ | `bytes_stream` → `SseEvent`（通用字段 `event`/`data`/`id`/`retry`，字节层切行）；厂商语义留给适配器 |
| 收尾语义 ✅ | 传输错误 / 可恢复坏帧 / **EOF 截断** 三分（`A2` §4）。机制在 `finish`：`FramePolicy<F>`（适配器判决）+ `finalize`（控制流）；帧类型泛型、与 SSE 解耦，仅 `finalize_bytes` 是 SSE 便捷入口 |
| 取消 ✅ | `CancellationToken`（即 `CallContext.cancel`）接进**请求**（`cancellable` / `HttpClient::send`）与**流**（`cancel::guard` / `CancelGuard`）；传输层只认 token，`CallContext` 桥接由适配器做 |
| 重试 | ⬜ 随 `A6` 落地：指数退避 + 抖动，只包 `retryable == true`。**不在 M2 范围** |

## 四个必须知道的约束

1. **只依赖 `agent-common`**：不 import 任何供应商实现，依赖只能向下（`A1` §2）。
2. **截断不得静默当成功**：EOF 无终止记录必须补 `End { Truncated }` 或 `Error`。
3. **断网可测**（G-3）：fixtures 存原始字节，用 `wiremock` 回放；退避 / 超时用 `tokio::time::pause()`。
4. **对外调用必须可取消**（`A1` §6）：请求与流都要接 `CancellationToken`；取消后**立刻**结束，不许挂到上游下次产出。

## 相关

- 传输规格 → `A4`（流式、取消与超时）｜ 契约 → `A2` ｜ 测试 → `Q1` ｜ 选型 → `X1` §2
- 概览 → `S1` ｜ 地图 → [`AGENTS.md`](../../../AGENTS.md) §2
