//! 契约唯一来源（对应 `docs/20-spec/provider-contract.md`，ID `A2`）。
//!
//! 规则：
//! 1. 改这里的类型前，先改 `docs/20-spec/` 下的对应规格文档。
//! 2. 本 crate 只依赖 `serde` / `serde_json` / `thiserror`，
//!    **不得**依赖 HTTP、UI 或任何供应商实现。

pub mod completion;
pub mod error;
pub mod message;
pub mod model;
pub mod stream;

pub use completion::{
    FinishReason, ModelRequest, ModelResponse, ResponseFormat, ToolCall, ToolChoice, ToolDefinition,
    Usage,
};
pub use error::{ErrorCategory, ProviderError};
pub use message::{ContentPart, Message, Role};
pub use model::{Capability, ModelSpec, Pricing};
pub use stream::{DeltaKind, StreamEvent};
