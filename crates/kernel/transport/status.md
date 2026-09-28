# agent-transport · 当前任务（M2 传输层）

| | |
|---|---|
| ID | —（crate 任务状态，随任务生灭） |
| 类型 | 状态 · **细节** |
| 更新 | 2026-09-28 |
| 何时读 | 正在做 M2 时；或想知道传输层卡在哪 |
| 规模 | ~0.6k token |

> **本文件只描述当前任务；任务完成即清空或重写。** 概览在根 `STATUS.md`（`S1`）。
> 现状：crate 骨架已建（`Cargo.toml` / `README.md` / `src/lib.rs`，已登记进 workspace 与地图）；HTTP / SSE / 重试等实现待补。

## 任务

让「一次模型调用」能真正发出去并流式收回来。

### 待办

- [x] 建 `crates/kernel/transport`：`Cargo.toml` + `README.md` + `src/lib.rs`，并在 `AGENTS.md` §2 地图登记
- [x] reqwest client 封装：连接 / 读超时、**流式空闲超时**、代理、rustls（`src/http.rs`）
- [ ] SSE 解析：`bytes_stream` → 事件流。先评估 `eventsource-stream` / `sse-rs`；不满足则自研（200–400 行，含停止条件）
- [ ] 三分收尾语义（`A2` §4）：传输错误 / 可恢复坏帧 / **EOF 截断** —— 截断绝不允许静默当成功
- [ ] 取消：把 `CallContext.cancel` 接进请求与流
- [ ] 退避重试：只包 `ProviderError::retryable == true` 的情况，指数退避 + 抖动
- [ ] 按 `Q1` 的 12 条用例补 fixtures，重点第 5（截断）、6（坏帧）、11（无 usage）
- [ ] 写 `A4`（流式、取消与超时）规格文档，并在 `AGENTS.md` §2 把状态改成 ✅

### 阻塞

- 无。`S2` 的 Q4（MCP）/ Q5（本地模型）不影响本任务。

## 相关

- 契约：`A2` §4 ｜ 测试要求：`Q1` ｜ 选型依据：`X1` §2
- 概览：`S1` ｜ 排期与完成标准：`RM` ｜ 地图：`AGENTS.md` §2
