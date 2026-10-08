//! 工具执行边界（规格：`A5`）。
//!
//! `chat` 只认 [`Tool`] 接口，实现由宿主提供（`app` 的 demo 工具、测试的假工具）。
//! M4 顺序执行；M6 并行化 / 失败降级只改 [`ToolSet`] 的调度，不改本接口（`A5`）。

use std::sync::Arc;

use agent_common::ToolDefinition;
use futures::future::BoxFuture;
use serde_json::Value;

/// 工具执行结果：`Ok` = 成功值；`Err` = 错误文案。
///
/// 错误**不中断循环**：作为 `tool_result` 喂回模型，让它自救或改道（`A5` 错误语义）；
/// 模型反复失败由 `max_model_turns` 兜底（见 [`crate::service`])。
pub type ToolOutput = Result<Value, String>;

/// 一个可被模型调用的工具。
///
/// `run` 返回 owned future（手写 `BoxFuture`，与 `Provider` trait 的风格一致，不引 `async_trait`）；
/// 取消由调用方（service）在 future 外层用 `select` 处理，工具自身不必感知。
pub trait Tool: Send + Sync {
    /// 暴露给模型的定义（name / description / JSON Schema）。
    fn def(&self) -> ToolDefinition;

    /// 执行一次调用。应当是纯输入输出：无会话状态、无 UI 依赖。
    fn run(&self, arguments: Value) -> BoxFuture<'static, ToolOutput>;
}

/// 工具注册表：模型看到 `definitions()`，调用走 `get()`。
#[derive(Default, Clone)]
pub struct ToolSet {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个工具（builder 风格）。
    pub fn with(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|tool| tool.def()).collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .iter()
            .find(|tool| tool.def().name == name)
            .cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Echo;

    impl Tool for Echo {
        fn def(&self) -> ToolDefinition {
            ToolDefinition {
                name: "echo".into(),
                description: "原样返回参数".into(),
                parameters: json!({"type": "object"}),
            }
        }

        fn run(&self, arguments: Value) -> BoxFuture<'static, ToolOutput> {
            Box::pin(async move { Ok(arguments) })
        }
    }

    #[tokio::test]
    async fn registry_lookup_and_run() {
        let set = ToolSet::new().with(Arc::new(Echo));
        assert_eq!(set.len(), 1);
        assert_eq!(set.definitions()[0].name, "echo");
        let tool = set.get("echo").expect("registered tool");
        let out = tool.run(json!({"x": 1})).await.unwrap();
        assert_eq!(out, json!({"x": 1}));
        assert!(set.get("nope").is_none());
    }
}
