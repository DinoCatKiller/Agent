# agent-transport · 传输层

| | |
|---|---|
| 状态 | 🚧 HTTP 封装已落地（M2 进行中） |
| 边界 | HTTP 客户端封装、SSE 解析、超时 / 取消 / 退避重试。**不含**供应商字段映射（`agent-providers`）、路由策略（`kernel/routing`） |
| 上游 | `agent-common`（契约类型） |
| 下游 | `agent-providers` 的适配器 |
| 何时读 | 动 SSE / 中断 / 超时 / 重试前 |

## 这个 crate 是什么

负责把「一次模型调用」**真正发出去并流式收回来**：一次真实请求能流式收完，
且坏帧 / 截断 / 超时都有明确语义与用例覆盖。

## 核心内容（规划，随 M2 落地）

| 项 | 说明 |
|----|------|
| HTTP 封装 ✅ | `reqwest`（rustls）、连接 / 读超时、**流式空闲超时**（`IdleTimeout`）、代理 |
| SSE 解析 | `bytes_stream` → 事件流；兼容三套事件语义（见 `X1` §2） |
| 收尾语义 | 传输错误 / 可恢复坏帧 / **EOF 截断** 三分（`A2` §4） |
| 取消 | `CallContext.cancel`（`CancellationToken`）接进请求与流 |
| 重试 | 指数退避 + 抖动，只包 `retryable == true` |

## 三个必须知道的约束

1. **只依赖 `agent-common`**：不 import 任何供应商实现，依赖只能向下（`A1` §2）。
2. **截断不得静默当成功**：EOF 无终止记录必须补 `End { Truncated }` 或 `Error`。
3. **断网可测**（G-3）：fixtures 存原始字节，用 `wiremock` 回放；退避 / 超时用 `tokio::time::pause()`。

## 相关

- 传输规格 → `A4`（M2 落地时创建）｜ 契约 → `A2` ｜ 测试 → `Q1` ｜ 选型 → `X1` §2
- 当前任务细节 → `crates/kernel/transport/status.md` ｜ 概览 → `S1`
