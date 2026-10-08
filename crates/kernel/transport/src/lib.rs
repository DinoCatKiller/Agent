//! 传输层：把「一次模型调用」真正发出去并流式收回来（对应 `A4`）。
//!
//! 职责边界：
//! - 只向下依赖 `agent-common`（契约类型），不认识任何具体供应商；
//!   供应商差异（鉴权头、字段名）留在 `agent-providers` 的适配器里（见 `A1` §2）。
//! - 输入输出都是归一化类型；传输层不产生供应商特有的结构。
//!
//! 模块（随 M2 逐步落地，落地前不建空文件）：
//! - [`http`] reqwest client 封装：连接 / 读超时、**流式空闲超时**、代理、rustls ✅
//! - [`sse`] `bytes_stream` → 通用 SSE 事件（厂商语义留给适配器）✅
//! - [`finish`] 三分收尾语义的**机制**（传输错误 / 可恢复坏帧 / EOF 截断；
//!   帧类型泛型，不绑 SSE）✅
//! - [`cancel`] 把 `CancellationToken`（即 `CallContext.cancel`）接进**请求**与**流** ✅
//! - `retry`  指数退避 + 抖动，只包 `ProviderError::retryable == true`（⬜ 随 `A6` 落地）
//!
//! 收尾三分语义（`A2` §4，硬约束，绝不允许静默降级）：
//! 1. **传输错误** → 发 `Error` 后停止；
//! 2. **可恢复坏帧** → 跳过该帧继续，不中断整条流；
//! 3. **EOF 无终止记录（截断）** → 补 `End { finish_reason: Truncated }` 或 `Error`，
//!    **禁止当成功**。
//!
//! 前两条由 [`finish`] 的 [`FramePolicy`] 实现，第 3 条由该 trait 的
//! [`FramePolicy::on_truncated`] 兜底；具体判决（哪帧算终止 / 坏帧 / 补什么）
//! 由适配器给出，传输层只执行控制流（`D6` §1）。

pub mod cancel;
pub mod finish;
pub mod http;
pub mod sse;

pub use cancel::{CancelGuard, cancellable, guard};
pub use finish::{FinalizedStream, Frame, FramePolicy, finalize, finalize_bytes};
pub use http::{HttpClient, HttpConfig, IdleTimeout, TransportError};
pub use sse::{SseDecoder, SseEvent, SseStream, parse_sse};
