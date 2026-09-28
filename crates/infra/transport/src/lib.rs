//! 传输层：把「一次模型调用」真正发出去并流式收回来（对应 `A4`）。
//!
//! 职责边界：
//! - 只向下依赖 `agent-common`（契约类型），不认识任何具体供应商；
//!   供应商差异（鉴权头、字段名）留在 `agent-providers` 的适配器里（见 `A1` §2）。
//! - 输入输出都是归一化类型；传输层不产生供应商特有的结构。
//!
//! 模块（随 M2 逐步落地，落地前不建空文件）：
//! - [`http`] reqwest client 封装：连接 / 读超时、**流式空闲超时**、代理、rustls ✅
//! - `sse`    `bytes_stream` → 事件流；兼容三套事件语义（见 `X1` §2）
//! - `retry`  指数退避 + 抖动，只包 `ProviderError::retryable == true`
//! - `cancel` 把 `CallContext.cancel`（`CancellationToken`）接进请求与流
//!
//! 收尾三分语义（`A2` §4，硬约束，绝不允许静默降级）：
//! 1. **传输错误** → 发 `Error` 后停止；
//! 2. **可恢复坏帧** → 跳过该帧继续，不中断整条流；
//! 3. **EOF 无终止记录（截断）** → 补 `End { finish_reason: Truncated }` 或 `Error`，
//!    **禁止当成功**。

pub mod http;

pub use http::{HttpClient, HttpConfig, IdleTimeout, TransportError};
