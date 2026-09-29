# A4 · 流式、取消与超时

| | |
|---|---|
| ID | `A4` |
| 类型 | 规格（归属 `agent-transport`） |
| 状态 | ✅ 已落地（HTTP / SSE / 收尾 / 取消） |
| 更新 | 2026-09-29 |
| 何时读 | 动 SSE / 中断 / 超时 / 装配一次流式调用时 |
| 规模 | ~2k token |

## TL;DR

- 一次流式调用是一条**五段管线**，每段只做一件事，可单独替换与测试：

  ```text
  bytes_stream → guard_idle → parse_sse → finalize(policy) → guard(token)
  ```

- 传输层**只做机制**（`D6` §1）：不认识厂商字段，也不知道哪一帧算终止记录。
  这些由适配器实现 `FramePolicy` 提供（`A2` §2）。
- 三分收尾语义（`A2` §4）全部收口在 `finish` 一个模块里，见 §2。
- **取消**分两阶段：请求用 `cancellable` / `HttpClient::send`，流用 `guard`。见 §3。
- **超时**三种，其中流式请求**不设**整体超时，靠**空闲超时**兜底。见 §4。
- 断网可测（`G-3`）：原始字节 fixtures + `tokio::time::pause()`。

## 1. 管线

| 段 | 类型 | 输入 → 输出 | 职责 |
|----|------|-------------|------|
| `guard_idle` | `IdleTimeout<S>` | `Result<T, TransportError>` 流 → 同类型 | 相邻两帧间隔超过上限就产出 `IdleTimeout` |
| `parse_sse` | `SseStream<S>` | `Result<Bytes, _>` 流 → `Result<SseEvent, _>` 流 | 字节 → 通用 SSE 帧；**不认识厂商** |
| `finalize` | `FinalizedStream<S, F, P>` | `Result<SseEvent, _>` 流 → `Result<P::Output, _>` 流 | 执行三分收尾（§2） |
| `guard` | `CancelGuard<S>` | 任意 `Result<T, _>` 流 → 同类型 | 取消即收尾（§3） |

两条硬规则：

1. **错误先归一化，再进管线**。`IdleTimeout` / `SseStream` / `FinalizedStream` /
   `CancelGuard` 的输入输出都是 `Result<T, TransportError>`，不做二次包装——
   所以 `bytes_stream()` 的 `reqwest::Error` 必须在入口处就 `map(TransportError::from)`，
   否则会得到 `Result<Result<_, _>, _>` 这种没法往下接的类型。
2. **`TransportError` 不等于 `ProviderError`**。前者只描述「发/收」出了什么事；
   「HTTP 状态 → `ErrorCategory`」是适配器的映射表（`A2` §5）。

## 2. 三分收尾语义（`A2` §4 的落点）

契约在 `A2` §4，**机制**在这里。`FramePolicy` 是唯一需要适配器实现的部分：

| 情形 | 机制行为 | 适配器怎么写 |
|------|---------|-------------|
| 传输错误 | 上抛 `TransportError` 并**停止**；不补任何收尾事件 | 不用管 |
| 可恢复坏帧 | 跳过该帧、流继续（记一条 debug 日志） | `Frame::Skip` |
| 正常帧 | 产出 0..n 个事件 | `Frame::one(..)` / `Frame::many(..)` |
| 终止记录 | 产出 0..n 个末帧后流**正常结束**（不再读上游） | `Frame::Terminal(vec![..])` / `Frame::end()` |
| **EOF 无终止记录** | 调 `on_truncated()`，把返回值发给下游 | `on_truncated()` 必须给出表达**截断**的事件 |

两个容易踩的点：

- **一帧可以产出多个事件**，终止记录也一样。这不是花哨：契约要求 `End` 之前必须有
  一次 `Usage`（`A2` §4），而供应商可能把 usage 与终止原因放在**同一条**记录里
  （OpenAI Responses 的 `response.completed`）；供应商干脆不给 usage 时，适配器也要
  在收尾处补 `Usage::default()`（`Q1` 用例 11）。一帧只能产一个事件的话，这两件事
  无法同时做到。
- **`[DONE]` 之类只是帧内容，不是机制的一部分**。判定权在策略里，机制从不解析
  `data` 的语义。

## 3. 取消

`CallContext` 定义在 `agent-providers`（`A2` §2），而 transport 不能反向依赖它
（`A1` §2），所以本层只认 `tokio_util::sync::CancellationToken`；把 `ctx.cancel`
传进来是适配器的一行代码。

| 阶段 | 入口 | 语义 |
|------|------|------|
| 请求 | `cancellable(&token, fut)`、`HttpClient::send(req, &token)` | `biased` 先看取消：**已取消时绝不发起**，挂起时立刻放弃等待 |
| 流 | `guard(stream, token)` | 取消后产出一次 `Cancelled` 并结束；**取消优先于**尚未吐出的帧 |

实现要点（改这里之前务必知道）：tokio-util 的 `cancelled()` 内部是 `Notified`，
**Drop 即注销**注册。所以 `CancelGuard` 持久保存一个 `'static` 的
`WaitForCancellationFutureOwned`（`token.clone().cancelled_owned()`）——每次临时构造
再丢弃的话，取消时**没人唤醒**我们，流会一直挂到上游下次产出。

## 4. 超时

| 超时 | 位置 | 默认 | 说明 |
|------|------|------|------|
| 连接（含 TLS 握手） | `HttpConfig::connect_timeout` | 10s | 建连阶段 |
| 整体请求 | `HttpConfig::request_timeout` | **`None`** | **流式请求不要设**：长回答会被整体超时误杀 |
| 流式空闲 | `HttpConfig::stream_idle_timeout` | 60s | 判据是**相邻两帧的间隔**，不是总耗时 |

为什么必须有空闲超时：部分供应商**不发结束记录**（`X1` §2）。没有它，一条断掉的
连接会让调用方永久挂起——这正是 `A2` §4「截断不得静默当成功」在时间维度上的对应物。

## 5. 装配示例

```rust
// 1) 请求：URL / 鉴权头 / 请求体由适配器构造（A2 §2）
let response = client.send(request, &ctx.cancel).await?;

// 2) 字节：先归一化错误，再套空闲超时
let bytes = client.guard_idle(response.bytes_stream().map(TransportError::from));

// 3) 字节 → 通用帧；4) 帧 → 归一化事件（三分收尾在其中）；
// 5) 最外层接取消（也是让连接真正被关掉的那一层）
let events = finalize(parse_sse(bytes), MyPolicy::new(model, response_id));
let stream = guard(events, ctx.cancel.clone());
```

`MyPolicy` 只需实现两个方法；它不碰网络，纯函数式翻译 + 判定。

## 6. 与 `Q1` 12 条用例的关系

| `Q1` 用例 | 归属 | 状态 |
|-----------|------|------|
| 2 流式文本（顺序 + 拼接） | **传输层**（机制）+ 适配器（字段） | ✅ 传输层已覆盖 |
| 5 截断（EOF 无终止记录） | **传输层**（机制）+ 适配器（补什么） | ✅ 传输层已覆盖 |
| 6 坏帧 | **传输层**（机制）+ 适配器（怎么判坏） | ✅ 传输层已覆盖 |
| 11 无 usage | **传输层**（一帧可产多事件）+ 适配器（补默认值） | ✅ 传输层已覆盖 |
| 1 / 3 / 4 / 7–10 / 12 | 适配器（`M3`） | ⬜ |

传输层用 `tests/fixtures/*.sse` 的原始字节 + `tests/pipeline.rs` 跑**完整管线**
（含逐字节分片），所以「分片边界不影响语义」也被固定住了。

## 落地清单

- [x] `HttpClient`：rustls、连接 / 整体超时、代理、`send`（接取消）
- [x] `IdleTimeout`：空闲超时（扁平化，可嵌管线任意位置）
- [x] `SseDecoder` / `SseStream`：字节层切行、BOM、CRLF、`retry`、未知字段
- [x] `Frame` / `FramePolicy` / `FinalizedStream`：三分收尾，帧类型泛型（不绑 SSE）
- [x] `CancelGuard` / `cancellable`：请求与流两阶段取消
- [x] 原始字节 fixtures + 端到端管线测试（正常 / 截断 / 坏帧 / 无 usage / 取消）
- [ ] 退避重试 → **不在本规格**：随 `A6`（错误、重试与降级）落地。只包 `retryable == true`，
  **不做流中重试**（已吐出的增量撤不回）

## 相关

- 上游：`A2` §4 / §5（收尾语义与错误分类）、`A1` §2 / §6（分层与「调用必须可取消」）
- 测试：`Q1` §2 fixtures 约定 / §3 用例清单 / §4 `time::pause()`
- 选型依据：`X1` §2（生产商终止记录差异、SSE 库评估）
- 重试与降级 → `A6` ｜ 未决项 → `S2`
