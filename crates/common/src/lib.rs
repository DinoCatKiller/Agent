//! 契约唯一来源（对应 `A2`，ID `A2`）。
//!
//! 规则：
//! 1. 改这里的类型前，先改对应规格文档（`A2` / `A3`，路径见 `AGENTS.md` §2 地图）。
//! 2. 本 crate 只依赖 `serde` / `serde_json` / `thiserror`，
//!    **不得**依赖 HTTP、UI 或任何供应商实现。

pub mod completion;
pub mod error;
pub mod message;
pub mod model;
pub mod stream;

pub use completion::{
    FinishReason, ModelRequest, ModelResponse, ResponseFormat, ToolCall, ToolChoice,
    ToolDefinition, Usage,
};
pub use error::{ErrorCategory, ProviderError};
pub use message::{ContentPart, Message, Role};
pub use model::{Capability, ModelSpec, Pricing};
pub use stream::{DeltaKind, StreamEvent};
