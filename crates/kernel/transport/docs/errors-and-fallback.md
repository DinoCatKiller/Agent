# A6 · 错误、重试与降级

| | |
|---|---|
| ID | `A6` |
| 类型 | 规格（归属 `agent-transport`） |
| 状态 | ✅ 已落地（重试机制 + 两级策略接线，2026-10-11） |
| 更新 | 2026-10-11 |
| 何时读 | 限流 / 超时 / 审核 / 不可用；想改重试次数或降级行为时 |
| 规模 | ~1.5k token |

## TL;DR

- **三段职责各有其家**：**错误分类** → `A2` §5（各适配器出「HTTP 状态 / 错误体 → `ErrorCategory`」映射表）；**重试** → 本节（`agent-transport`，退避算法取自 `backoff`）；**换模型降级** → `R2` §3（`agent-routing` 的 `Router`）。本节**不复制**那两处的内容，只定义「重试」本身以及**两者的组合关系**。
- **两级策略（本节的实质内容）**：先在**同一候选内退避重试**至 `max_attempts`，仍失败才**换下一候选**。
- **默认不重试**：`Router` 默认 `RetryPolicy::none()`（保持 M6 行为）；由宿主显式开启——`app` 已配 `RetryPolicy::default()`（3 次尝试、500ms 起、上限 8s、抖动 0.5）。
- **流式的重试窗口 = 首个事件之前**：`Start` 一旦发出就不再重试、不再换流（否则会重复输出）。
- 退避等待期间**取消立即生效**（`CancellationToken`，见 `A4`）。

## 1. 三段落点

| 职责 | 归属 | 由谁实现 | 说明 |
|------|------|---------|------|
| 错误分类 | `A2` §5 | 各适配器 | 「429 → `RateLimit`、5xx → `Server`…」的映射表；`retryable` 默认来自 `ErrorCategory::retryable()`，策略层可覆盖 |
| **重试** | **本节**（`agent-transport`） | 机制层提供策略与执行器 | 只管「同一目标重试几次、间隔多久」 |
| 换模型降级 | `R2` §3 | `agent-routing` 的 `Router` | 只管「换哪个候选」 |

`A1` §2 的分层里，「重试 / 降级」同属 **Capability & Policy** 层；实现上 `Router`（策略）向下使用 `agent-transport`（机制），依赖方向合规。

## 2. 重试机制

### 2.1 策略参数

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,        // 总尝试次数（含首次）；1 = 不重试
    pub initial_interval: Duration,
    pub max_interval: Duration,
    pub multiplier: f64,
    pub jitter: f64,              // 0.0–1.0；0.0 便于确定性测试
}

RetryPolicy::default()  // 3 次 / 500ms / 8s / ×2.0 / 抖动 0.5
RetryPolicy::none()     // 1 次，即不重试（Router 的默认值）
```

退避序列由 `backoff` 生成：`min(initial × multiplier^(n-1), max_interval)` 再加抖动。**整棵代码库里退避算法只有这一份**（`X6` A3 的选型结论）。

### 2.2 一问一答：`retry()`

```rust
pub async fn retry<T, F, Fut>(
    policy: &RetryPolicy,
    cancel: &CancellationToken,
    op: F,                       // F: FnMut(attempt: u32) -> Fut
) -> Result<T, ProviderError>
```

- 只重试 `error.retryable == true`，否则**立即**返回；
- 尝试次数耗尽 → 返回**最后一次**错误（不吞错）；
- 退避期间取消 → 返回「已取消」错误（`retryable == false`）。

### 2.3 自行编排：`Backoff`

流式调用无法用 `retry()` 的 Future 包装（它要先偷看首事件），因此暴露退避序列器：

```rust
let mut backoff = policy.backoff();
while let Some(delay) = backoff.next_delay() {   // None = 重试额度用尽
    tokio::time::sleep(delay).await;
}
```

`max_attempts = n` ⇒ 最多 `n - 1` 次退避。用它可保证与 `retry()` 共用同一套退避语义。

## 3. 两级策略的组合（`Router`）

`Router` 对每个候选执行：

```text
for candidate in 候选链 {
    候选内：retry(policy)  ← 429 / 5xx / 超时 / 断网 时退避重试
        ├─ 成功        → 返回
        └─ 额度用尽    → 落出到「候选间」
    候选间：should_fallback(最后一个错误) 为真 → 换下一候选
                                       为假 → 立即失败
}
```

- **流式**：上述"候选内重试"只在**首事件之前**发生（`Some(Error)` 或同步 `Err`）。首事件是正常事件后，整条流原样转发，不再重试、不再换流（`R2` §3 的窗口）。
- **`chat`（一问一答）**：由 `retry()` 完整包装，语义同上但无「首事件」概念。
- 重试失败是**对消费者透明**的：被重试/被换掉的候选不产生任何事件，只有全部耗尽才浮出最后一个错误（`R2` §3 的透明降级原则）。

## 4. 取消语义

任何一层等待都必须可取消：

| 阶段 | 取消点 |
|------|--------|
| 退避等待 | `wait_backoff` 的 `tokio::select!`（biased，取消优先） |
| 一问一答重试 | `retry()` 内部同款 select，返回「已取消」错误 |
| 候选遍历 | 每轮循环前检查 `ctx.cancel.is_cancelled()`，已取消直接结束且**不产生事件** |

取消后必须**立刻**结束，不许挂到上游下次产出（`A1` §6 / `A4`）。

## 落地清单

- [x] `kernel/transport` 落 `retry` 模块：`RetryPolicy` / `retry()` / `Backoff`（退避取自 `backoff`）。
- [x] `kernel/routing` 的 `Router` 增加 `with_retry(policy)`，候选内重试 → 候选间降级。
- [x] `app` 装配开启 `RetryPolicy::default()`。
- [x] 用例：重试成功不降级 / 额度用尽才降级 / 默认不重试 / 退避期间取消（`agent-transport`、`agent-routing`）。
- [ ] 按 `ErrorCategory` 分级策略（如 `Server` 与 `RateLimit` 用不同退避曲线）—— 需要时再加，`ProviderError::retryable` 已预留覆盖口。
- [ ] 预算控制（单轮总重试上限 / 成本上限）—— 随 `R3`（可观测性与成本）。

## 相关

- 错误分类与 `ProviderError` 契约 → `A2` §5 ｜ 降级策略（换谁） → `R2` §3 ｜ 流式与取消 → `A4`
- 重试选型（为什么用 `backoff`） → `X6` A3 ｜ 分层判据（机制 vs 策略） → `D6` §1
- 契约测试与 fixtures → `Q1` ｜ 未决项 → `S2`
