# agent-app · 二进制入口

| | |
|---|---|
| 状态 | ✅ M4：契约冒烟 + 对话 REPL；✅ M5：`tui` 一期终端界面（侧栏会话 + 流式转录 + 轮末落盘）；✅ M6：请求统一经 `Router` 出（`--model a,b` 失败降级链） |
| 边界 | 组装依赖、承载 CLI 命令、终端生命周期。**不含**业务逻辑（都在 `features/*` 里） |
| 上游 | `agent-common` / `agent-providers` / `agent-transport` / `agent-chat` / `agent-sessions` / `agent-store` |
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
#       --model a,b（逗号分隔 = 失败降级链：a 遇 429/5xx/超时/断网/Auth 自动落到 b，R2）

# 一期 TUI（M5，`D3`）
cargo run -p agent-app -- tui --provider openai-compatible --model gpt-4o
# 可选：--db PATH（会话库路径，缺省 ./agent-sessions.db）；--model 同样支持 a,b 降级链
```

> **终端要求（Windows）**：crossterm 的 raw mode 走 Win32 控制台 API，请在 **Windows Terminal / PowerShell / cmd** 里运行。
> Git Bash 的 MinTTY 不是 Windows 控制台（stdio 是管道），直接跑会「终端初始化失败」——加 `winpty` 前缀即可：`winpty cargo run -p agent-app -- tui …`。

TUI 内：左栏会话列表（Tab 切焦点，↑↓ 选择，Enter 载入，`n` 新建），右侧流式对话
（↑↓/PageUp/PageDown 滚动）；Ctrl-C 取消当前生成（生成中）或退出（空闲）；Ctrl-Q 退出；
状态栏显示 provider / model / 会话标题 / 累计 tokens。**每轮结束自动落盘**——重启后
会话列表与完整历史都在。配色来自 [`design/tokens.json`](../../design/tokens.json)（`D4`）。

REPL 内：直接输入对话；`/quit` 退出；**Ctrl-C 取消当前生成**（第一次中断本轮、丢弃未提交输出，不退出程序）。
内置两个 demo 工具（`current_time` / `echo`）证明工具循环端到端可用。

## 它不能做什么

- ❌ 重试退避（`A6` 待建）、密钥管理（`R1` 待建）
- ❌ 密钥经 `R1` 管理前，只走参数 / 环境变量，且不入库、不打日志

## 布局

```
src/main.rs     命令分发 + 参数解析（self-check 保留在 main 内）
src/setup.rs    装配共用：供应商构造 / 密钥来源 / demo 工具（chat 与 tui 共用）
src/repl.rs     chat 命令：消费 LoopEvent 打印（事件消费的参考实现）
src/tui.rs      tui 命令：终端生命周期 + 事件泵 + worker（唯一拥有 Chat 与 SessionRepo）
tests/e2e.rs    端到端：真实适配器 + reqwest 栈 → wiremock，工具循环两轮收口（Q1 §1）
```

## 相关

- `S1` 当前阶段 ｜ `A1` 架构 ｜ `A5` 工具调用 ｜ `Q1` 测试策略 ｜ UI 形态：`D3`、样式：`D4`
