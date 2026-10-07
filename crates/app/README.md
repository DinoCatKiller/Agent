# agent-app · 二进制入口

| | |
|---|---|
| 状态 | ✅ M4：契约冒烟 + 对话 REPL（单轮 / 多轮工具循环 / Ctrl-C 取消） |
| 边界 | 组装依赖、承载 CLI 命令。**不含**业务逻辑（都在 `features/*` 里） |
| 上游 | `agent-common` / `agent-providers` / `agent-transport` / `agent-chat` |
| 下游 | 无（最终产物） |
| 何时读 | 想跑起来看一眼、或新增 CLI 命令时 |

## 现在能做什么

```powershell
# 契约层自检（7 段，断网）
cargo run -p agent-app -- self-check

# 对话 REPL（M4）
cargo run -p agent-app -- chat --provider openai-compatible --model gpt-4o
cargo run -p agent-app -- chat --provider anthropic --model claude-sonnet-4-5-20250929
# 可选：--base-url（vLLM / 本地服务，传入后不做模型清单拦截）
#       --api-key（缺省读 OPENAI_API_KEY / ANTHROPIC_API_KEY 环境变量）
```

REPL 内：直接输入对话；`/quit` 退出；**Ctrl-C 取消当前生成**（第一次中断本轮、丢弃未提交输出，不退出程序）。
内置两个 demo 工具（`current_time` / `echo`）证明工具循环端到端可用。

## 它不能做什么

- ❌ 没有会话持久化（M5 `kernel/store`）、没有路由兜底（M6）、没有 UI（`D3` 未决）
- ❌ 密钥经 `R1` 管理前，只走参数 / 环境变量，且不入库、不打日志

## 布局

```
src/main.rs     命令分发 + 参数解析（self-check 保留在 main 内）
src/repl.rs     chat 命令：装配 ChatService、消费 LoopEvent 打印（未来 UI 的参考实现）
tests/e2e.rs    端到端：真实适配器 + reqwest 栈 → wiremock，工具循环两轮收口（Q1 §1）
```

## 相关

- `S1` 当前阶段 ｜ `A1` 架构 ｜ `A5` 工具调用 ｜ `Q1` 测试策略
