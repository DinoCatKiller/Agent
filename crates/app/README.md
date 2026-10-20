# agent-app · 二进制入口

> **包名 vs 可执行文件**：crate 包名是 `agent-app`（内部技术名，`S2` Q7），编译出的可执行文件是
> **`codingrocket`**（`cargo build -p agent-app` → `codingrocket.exe`）。下面命令里的 `-p agent-app`
> 接的是**包名**，不受影响。

| | |
|---|---|
| 状态 | ✅ M4 契约冒烟 + 对话 REPL ｜ ✅ M5 `tui` 一期终端界面（会话 + 流式 + 键鼠）｜ ✅ M6 请求经 `Router` 出（降级链）｜ ✅ `X6` A1/A2 CLI `clap` + REPL `rustyline` ｜ ✅ `A6` 候选内重试 ｜ ✅ `R1` 配置文件 + 系统钥匙串 ｜ ✅ `R3` 日志初始化 + 成本计算 ｜ ✅ 二期 `gui` 子命令（GPUI 窗口，`gui` feature，2026-10-12）｜ ✅ `gui` 内含设置页（`features/settings`，写配置 + 存钥匙串） |
| 边界 | 组装依赖、承载 CLI 命令、终端 / 窗口生命周期。**不含**业务逻辑（都在 `features/*` 里） |
| 上游 | `agent-common` / `agent-providers` / `agent-transport` / `agent-routing` / `agent-config` / `agent-telemetry` / `agent-chat` / `agent-sessions` / `agent-store`；`agent-ui-kit` / `agent-settings`（**仅 `gui` feature**） |
| 下游 | 无（最终产物） |
| 何时读 | 想跑起来看一眼、或新增 CLI 命令时 |

## 现在能做什么

```powershell
# 帮助（clap 生成，含每个子命令的选项说明）
cargo run -p agent-app -- --help

# 契约层自检（7 段，断网）
cargo run -p agent-app -- self-check

# 对话 REPL（M4）。`--provider` / `--model` 都可省 —— 省了从配置文件取（R1）
cargo run -p agent-app -- chat --provider openai-compatible --model gpt-4o
cargo run -p agent-app -- chat          # 完全靠配置文件里的默认值也能起来（见下「解析顺序」）
# 可选：--base-url（vLLM / 本地服务，传入后不做模型清单拦截）
#       --api-key（缺省按「环境变量 → 系统钥匙串」解析，R1）
#       --config PATH（配置文件；缺省走平台标准目录，R1）
#       --model a,b（逗号分隔 = 失败降级链；A6：同一候选先退避重试 3 次，用尽才换下一家）

# 一期 TUI（M5，`D3`）
cargo run -p agent-app -- tui --provider openai-compatible --model gpt-4o
# 可选：--db PATH（会话库路径，缺省 ./agent-sessions.db）

# 二期 GUI（`D3` 方案 A）：**注意 `--features gui`**，默认构建不含 GPUI（详见下方「为什么 GUI 在 feature 后面」）
cargo run -p agent-app --features gui -- gui --provider openai-compatible --model gpt-4o
# 同 tui 的 --db / --config / --api-key / --base-url；窗口 1100x760，标题 CodingRocket
# 窗口内两个标签页：**对话**（`features/chat` 的 ChatView）与**设置**（`features/settings`）。
# 对话页底部有**模型选择器**：内置清单的 chip（`openai·gpt-4o` 这样）+ 手输模型名
#   （自定义端点用）。点 chip = 切 (provider, model)，**生成中不可切**，切换即时生效。
# 设置页每行是**一个供应商的 API Key**（钥匙串按 `<provider>:api_key` 存）；
#   配置项（默认供应商 / 模型 / 端点 / 代理 / 超时）按 `A` 语义**重启后生效**。
# 设置页的「连接提供商」：从内置目录挑厂商 → 写 `[providers.<id>]` + 存钥匙串
#   → **立刻**出现在模型选择器里（不用重启；落点是 `features/settings`，映射在 `setup::catalog_rows`）。
# 联网刷新模型清单（C2）：设置页「管理模型」里每个条目一个「刷新」按钮 →
#   宿主注入的 `on_refresh_models` 回调用 `Cmd::RefreshModels` 打给 worker →
#   经适配器的 `RefreshableModels` 拉 `GET /models` → 成功写回 `providers.<id>.models`；
#   宿主在事件桥里收到 `ModelsRefreshed` 后重算候选（推给对话页 `set_candidates` + 回灌设置页），
#   失败走 `ModelsRefreshFailed`，**不清空**现有清单（`anthropic` 暂无该能力）。
```

> **终端要求（Windows）**：crossterm 的 raw mode 走 Win32 控制台 API，请在 **Windows Terminal / PowerShell / cmd** 里运行。
> Git Bash 的 MinTTY 不是 Windows 控制台（stdio 是管道），直接跑会「终端初始化失败」——加 `winpty` 前缀即可：`winpty cargo run -p agent-app -- tui …`。

TUI 内：左栏会话列表（Tab 切焦点，↑↓ 选择，Enter 载入，`n` 新建），右侧流式对话
（↑↓/PageUp/PageDown 滚动）；Ctrl-C 取消当前生成（生成中）或退出（空闲）；Ctrl-Q 退出；
状态栏显示 provider / model / 会话标题 / 累计 tokens。**每轮结束自动落盘**——重启后
会话列表与完整历史都在。配色来自 [`design/tokens.json`](../../design/tokens.json)（`D4`）。

鼠标：左键点侧栏条目 = 选中并载入该会话，点转录 / 输入区 = 焦点回到输入；滚轮在侧栏
上下移动选择、在其余区域滚动转录。鼠标捕获会**接管**终端原生文本选择，要复制文本请按住
Shift 框选（Windows Terminal / ConPTY 下可用）。

REPL 内（`rustyline`，`X6` A2）：直接输入对话；↑↓ 翻历史、←→ 行内编辑、Ctrl-R 搜索；
`/quit` 退出；**Ctrl-C 取消当前生成**（第一次中断本轮、丢弃未提交输出，不退出程序）。
非交互场景（如 `echo "hi" | …`）自动回退到裸 `read_line`，管道输入照常可用。
内置两个 demo 工具（`current_time` / `echo`）证明工具循环端到端可用。

## 密钥与配置（`R1`）

- **provider / model 的解析顺序**（`setup::resolve`，唯一入口）：
  - provider：`--provider` > 配置文件 `default_provider` > **报错退出**（提示可用值）
  - model：`--model` 首个 > `providers.<provider>.model` > `default_model` > **报错退出**
  - 两边都没有就报错而不是猜 —— 模型选错是真花钱的事（`A2` §2 的精神）。降级链只由
    `--model a,b` 表达（`R2`），配置默认值不带降级链。
- 优先级：`--api-key` > 环境变量（`OPENAI_API_KEY` / `ANTHROPIC_API_KEY`）> **系统钥匙串**。
- 配置文件（TOML）**只放非敏感项**；路径用 `--config` 指定，或走平台标准目录
  （Windows `%APPDATA%\CodingRocket\config\config.toml` —— 注意 `config` 目录 + `config.toml` 文件名）。
- 配置读不到 / 解不开 → 提示一行后**降级为默认配置继续跑**，不挡启动。
- 细节、安全约定与 `keyring` 的坑 → `R1`。

## 为什么 GUI 在 feature 后面

`gui` 默认关闭：GPUI 的依赖树很大（debug 约 4GB，可执行文件 +50MB 量级），
一期 TUI 产物不该为尚未完成的图形界面买单（`S2` Q6 的 ≤30MB 警戒线）。

代价只有一条：**门禁必须带 `--all-features`**，否则 `gui` 那条分支不会被编译检查（`Q1` §5）。
等图形界面成为主形态，把 `gui` 加进 `default` 即可，其余代码不用动。

## 它不能做什么

- ❌ 设置**保存后立即生效**：`A` 语义 = 重启生效（`S1` 切片 5），B 方案的热切换未做。
  （唯一例外：「连接提供商」只**新增**条目、不动生效中的那对 `(provider, model)`，故宿主会立刻刷新选择器。）
- ❌ 无钥匙串服务时的加密文件降级（`R1` 已识别该降级路径，未实现）。
- ❌ 密钥永不入库、不打日志（硬约束 5）。

## 布局

```
src/main.rs     clap 命令定义 + 分发（self-check 保留在 main 内）
src/setup.rs    装配共用：配置 / 密钥 / 供应商构造 / demo 工具（所有命令共用）
                —— `build_provider` 返回 `BuiltProvider`（`ErasedProvider` + 可选 `RefreshableModels` 刷新句柄，同一 `Arc`；C2）
src/worker.rs   会话宿主：唯一拥有 `Chat` + `SessionRepo`（TUI 与 GUI **共用**，语义只有一份）
                —— 处理 `Cmd::SelectModel`（切 (provider, model)）与 `Cmd::RefreshModels`（联网刷新清单，C2）
src/repl.rs     chat 命令：rustyline 输入 + 消费 LoopEvent 打印（事件消费的参考实现）
src/tui.rs      tui 命令：终端生命周期 + 事件泵（键鼠）
src/gui.rs      gui 命令（`gui` feature）：窗口 + 后台 tokio runtime + 事件桥
tests/e2e.rs    端到端：真实适配器 + reqwest 栈 → wiremock，工具循环两轮收口（Q1 §1）
```

## 相关

- `S1` 当前阶段 ｜ `A1` 架构 ｜ `A5` 工具调用 ｜ `A6` 错误 / 重试 / 降级 ｜ `R1` 配置与密钥 ｜ `Q1` 测试策略
- UI 形态：`D3`、样式：`D4` ｜ CLI / REPL 选型 → `X6` A1 / A2
