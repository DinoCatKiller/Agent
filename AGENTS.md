# AGENTS.md — agent 入口 · 唯一地图

> 常驻上下文，必须精简。这里只有：读取协议、地图、硬约束；§1 / §3 已外移，仅留指针 → `M1`。
> **引用文档只写 ID（如 `A2`），不写路径** —— 路径只出现在 §2 这张表里。搬文件不用改其它文档。

## 0. 读取协议

1. 读本文件 → 在 §2 地图里定位与任务相关的条目（相关 1 条就读 1 条，相关 6 条就读 6 条，**不设数量上限**）。
2. 只读这些条目指向的文档。crate 的 `README.md` 是它的门面，**先读 README 再进它的 `docs/`**；单篇先看 `TL;DR`，需要细节再往下读。
3. **按需读取，宁多勿错**。唯一的判断标准是：**「读它能改变我的做法、或让我少犯错吗？」** 能 → 读，多读几篇没问题；不能 → 不读。
   读得多是因为**相关**，不是因为"额度还剩"；读无关上下文才是错误。
   - **明确不读**（除非任务就是它们）：`X*` 调研（一次性消费、会过时，只在要重新评估选型或找依据时读）；`D*` ADR 中与本任务无关的部分；`⬜` 条目（文件不存在）；与本任务无关的 crate 的 `docs/`。
4. `⬜` = 文件不存在。复制 [`docs/_template.md`](docs/_template.md) 建骨架，写完回填本表状态。**不预建空文件。**
5. **状态就地更新，从不搬家**（不要用「把文件挪进某个目录」表达阶段）。状态是**分层**的：
   - 根 [`STATUS.md`](STATUS.md)（`S1`）= **概览**：当前阶段的任务是什么、横跨哪些 crate。**一屏内，只写当前任务。**
   - `crates/<crate>/status.md` = **细节**：某个 crate 在这个任务里要做什么。一个任务可横跨多个 crate，各自建各自的。
   - **任务完成 → 清空或重写**，不留历史（历史交给 git）。长期排期写在 `RM`，**永远不写进 status**。
   - 未决问题仍在根 [`OPEN-QUESTIONS.md`](OPEN-QUESTIONS.md)（`S2`）。

## 1. 文档归属规则（按需读）
全文见 `M1`。只在**新建 / 搬移文档**时读。

## 2. 地图

规模为估算 token（≈KB × 350，中文），用于判断读取成本。
路径列中**已存在**的文档给可点击链接；状态 `⬜`（待建）的保持纯文本，避免死链。

**ID 前缀含义**（看 ID 就知道它大概是什么、属于谁）：

| 前缀 | 含义 | 前缀 | 含义 |
|------|------|------|------|
| `S1` / `S2` | 状态概览 / 未决问题 | `P1–P6` / `PT` | 供应商文档 / 供应商模板 |
| `G1` | 目标与技术路线 | `X1–X5` | 调研（一次性消费，会过时） |
| `A1–A7` | 架构与契约（跨 crate 或归属某 crate） | `D0–D6` | 决策 ADR（只增不改） |
| `R1–R4` | 运行时（配置 / 路由 / 观测 / 限流） | `RM` | 里程碑排期（**不含状态**） |
| `Q1–Q3` | 质量（测试 / 评测 / 安全） | `GUI` | 二期 GUI 与 L3 计划 |
| `M1` | 文档规范（归属 + 路由） | `T0` | 文档模板 |

### 状态与排期

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| S1 | 当前阶段**概览**（只写当前任务） | [`STATUS.md`](STATUS.md) | **每次开工第一眼** | ~0.5k | ✅ |
| S2 | 未决问题（全项目唯一来源） | [`OPEN-QUESTIONS.md`](OPEN-QUESTIONS.md) | 需要拍板时 | ~1k | ✅ |
| RM | 里程碑与排期定义（**不含状态**） | [`docs/roadmap.md`](docs/roadmap.md) | 想知道整体排期与完成标准 | ~0.7k | ✅ |

**crate 级任务细节** = `crates/<crate>/status.md`，**按需建立、随任务生灭**，因此**不逐条登记进本表**
（否则每换一个任务就要改地图，又变成搬文件）。当前存在的几个，在 `S1` 的「当前任务」表里已链接。

### 跨 crate 规格（根 `docs/`）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| G1 | 目标、技术路线、自研边界 | [`docs/goals.md`](docs/goals.md) | 争论「要不要自研 / 先做哪个」 | ~2k | ✅ |
| A1 | 架构与分层 | [`docs/architecture.md`](docs/architecture.md) | 新建模块、改目录结构 | ~1.6k | ✅ |
| A1-V | 架构图（`A1`/`RM`/`S1` 的**可视化**，离线 HTML） | [`docs/architecture-map.html`](docs/architecture-map.html) | 想一眼看懂分层 / 依赖 / 进度 | 1 张图 | ✅ |
| Q1 | 测试策略 | [`docs/testing.md`](docs/testing.md) | 写单测、契约测试、加适配器前 | ~1.4k | ✅ |
| Q2 | 效果评测与回归 | `docs/eval.md` | 建评测集、prompt 回归 | — | ⬜ |
| Q3 | 安全与合规 | `docs/security.md` | 密钥、审计、数据合规 | — | ⬜ |
| T0 | 文档模板 | [`docs/_template.md`](docs/_template.md) | 新建任何文档 | ~0.3k | ✅ |
| — | **API 参考**（脚本生成的代码字典，勿手工编辑） | [`docs/reference/`](docs/reference/) | 查某类型 / 方法签名的细节与 doc 注释；由 `scripts/generate-docs.py --all` 再生 | 79 篇 | ✅ |

### 元（文档规范）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| M1 | 文档归属规则 + 任务路由 | [`docs/index/meta.md`](docs/index/meta.md) | 新建 / 搬移文档；开新任务不知读哪些 | ~0.9k | ✅ |

### `crates/common`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | crate 门面 | [`crates/common/README.md`](crates/common/README.md) | 改契约类型前 | ~0.7k | ✅ |
| A3 | 消息与多模态协议 | [`crates/common/docs/message-protocol.md`](crates/common/docs/message-protocol.md) | 改消息 / 内容块 / tool_call_id | ~1.5k | ✅ |

### `crates/kernel/providers`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | crate 门面 | [`crates/kernel/providers/README.md`](crates/kernel/providers/README.md) | 加供应商、改 trait | ~0.8k | ✅ |
| A2 | Provider 接入契约 | [`crates/kernel/providers/docs/contract.md`](crates/kernel/providers/docs/contract.md) | 改调用接口、新增供应商 | ~2.6k | ✅ |
| X3 | 多模型抽象路线调研 | [`crates/kernel/providers/docs/abstraction-research.md`](crates/kernel/providers/docs/abstraction-research.md) | 质疑要不要自研抽象 | ~2.3k | ✅ |
| PT | 供应商文档模板 | [`crates/kernel/providers/docs/providers/_template.md`](crates/kernel/providers/docs/providers/_template.md) | 新建某家供应商文档 | ~0.4k | ✅ |
| P1 | OpenAI 兼容族差异 | [`crates/kernel/providers/docs/providers/openai.md`](crates/kernel/providers/docs/providers/openai.md) | 接 OpenAI / DeepSeek / vLLM 等兼容实现时 | ~1.5k | ✅ |
| P2 | Anthropic 差异 | [`crates/kernel/providers/docs/providers/anthropic.md`](crates/kernel/providers/docs/providers/anthropic.md) | 接 Claude / Anthropic 兼容端点时 | ~1.6k | ✅ |
| A2-V | 适配器边界图（`A2` 的可视化） | [`crates/kernel/providers/docs/adapter-map.html`](crates/kernel/providers/docs/adapter-map.html) | 想一眼看懂「插座」与两家差异 | 1 张图 | ✅ |
| P3–P6 | 其余供应商差异 | `crates/kernel/providers/docs/providers/*.md` | 接对应一家模型时 | — | ⬜ |

### `crates/kernel/transport`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | crate 门面 | [`crates/kernel/transport/README.md`](crates/kernel/transport/README.md) | 动传输层 / SSE 前 | ~0.6k | ✅ |
| A4 | 流式、取消与超时 | [`crates/kernel/transport/docs/streaming.md`](crates/kernel/transport/docs/streaming.md) | 动 SSE / 中断 / 超时 / 装配一次调用 | ~2k | ✅ |
| A4-V | 传输管线图（`A4` 的可视化） | [`crates/kernel/transport/docs/pipeline-map.html`](crates/kernel/transport/docs/pipeline-map.html) | 想一眼看懂流式调用的装配与收尾 | 1 张图 | ✅ |
| R4 | 缓存、限流与并发 | `crates/kernel/transport/docs/cache-and-ratelimit.md` | 重复请求、QPS 控制 | — | ⬜ |

### `crates/features/*`（功能切片：一个功能 = 一个文件夹，见 `D5`）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | 对话功能门面 | [`crates/features/chat/README.md`](crates/features/chat/README.md) | 改对话逻辑 / 界面时 | ~0.7k | ✅ M4–M5 |
| — | 会话功能门面 | [`crates/features/sessions/README.md`](crates/features/sessions/README.md) | 改会话列表 / 切换时 | ~0.6k | ✅ M5 |
| — | 设置功能门面 | [`crates/features/settings/README.md`](crates/features/settings/README.md) | 加设置项 / 改密钥录入时 | ~0.5k | ✅ 骨架 |
| A5 | 工具调用 | [`crates/features/chat/docs/tool-calling.md`](crates/features/chat/docs/tool-calling.md) | 工具定义、并行调用、结果回填 | ~1.3k | ✅ M4 |
| A7 | 上下文与 Token | [`crates/features/chat/docs/context-and-tokens.md`](crates/features/chat/docs/context-and-tokens.md) | 裁剪、摘要、计费 | ~1.2k | ✅ M4 |
| A5-V | 轮次循环图（`A5`/`A7` 的可视化） | [`crates/features/chat/docs/round-map.html`](crates/features/chat/docs/round-map.html) | 想一眼看懂 run_round 与工具循环 | 1 张图 | ✅ |

### `crates/kernel/*`（机制内核，按需创建）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| A6 | 错误、重试与降级 | `crates/kernel/transport/docs/errors-and-fallback.md` | 限流 / 超时 / 审核 / 不可用 | — | ⬜ |
| R1 | 配置与密钥 | `crates/kernel/config/docs/config-and-secrets.md` | 加配置项、接密钥 | — | ⬜ |
| R3 | 可观测性与成本 | `crates/kernel/telemetry/docs/observability.md` | 日志 / trace / 计费 | — | ⬜ |

### `crates/kernel/routing`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | 路由门面 | [`crates/kernel/routing/README.md`](crates/kernel/routing/README.md) | 加降级链 / 动注册表查询前 | ~0.7k | ✅ M6 |
| R2 | 模型注册表与路由 | [`crates/kernel/routing/docs/model-registry.md`](crates/kernel/routing/docs/model-registry.md) | 模型清单、路由与兜底 | ~1.2k | ✅ M6 |

### `crates/kernel/store`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | 存储门面 | [`crates/kernel/store/README.md`](crates/kernel/store/README.md) | 动持久化 / 建表 / 事务前 | ~0.6k | ✅ M5 |

### `crates/app`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | 二进制入口门面 | [`crates/app/README.md`](crates/app/README.md) | 想跑起来看一眼、新增 CLI 命令 | ~0.5k | ✅ |

### 决策记录（只增不改）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| D0 | ADR 模板 | [`docs/decisions/_template.md`](docs/decisions/_template.md) | 写新 ADR | ~0.2k | ✅ |
| D1 | 技术栈：Rust + GPUI | [`docs/decisions/0001-tech-stack.md`](docs/decisions/0001-tech-stack.md) | 质疑语言 / 框架选型 | ~0.9k | ✅ 已接受（UI 待复核） |
| D2 | 多模型抽象：契约自研 | [`docs/decisions/0002-llm-abstraction.md`](docs/decisions/0002-llm-abstraction.md) | 是否引入 rig / genai | ~1k | ✅ 已接受 |
| D3 | UI 交付形态 | [`docs/decisions/0003-ui-delivery-form.md`](docs/decisions/0003-ui-delivery-form.md) | 定 UI 技术、讨论分阶段 | ~0.9k | ✅ 已接受（一期 TUI，二期 A/C 再选） |
| D4 | 样式层建设路线 | [`docs/decisions/0004-styling-layer.md`](docs/decisions/0004-styling-layer.md) | 样式怎么组织、能否复用 TS 生态 | ~0.6k | ✅ 已接受（一期仅 tokens.json） |
| D5 | 代码按功能组织（Django 式） | [`docs/decisions/0005-feature-oriented-layout.md`](docs/decisions/0005-feature-oriented-layout.md) | 新建 crate / 功能、移动目录时 | ~1.6k | ✅ 已接受（部分被 `D6` 修订） |
| D6 | kernel（机制）与 features（语义）的判据 | [`docs/decisions/0006-kernel-vs-features.md`](docs/decisions/0006-kernel-vs-features.md) | 决定代码放哪、质疑"这算功能吗" | ~1.6k | ✅ 已接受 |
| D7 | Ollama 与本地模型定位 | [`docs/decisions/0007-ollama-local-models.md`](docs/decisions/0007-ollama-local-models.md) | 接 Ollama / 本地模型、设计 `R2` 时 | ~0.8k | ✅ 已接受 |
| D8 | MCP 客户端：自研最小接入 | [`docs/decisions/0008-mcp-client.md`](docs/decisions/0008-mcp-client.md) | 接 MCP / 扩工具生态时 | ~1k | ✅ 已接受 |

### 计划与跨领域调研

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| GUI | 二期 GUI + L3 样式引擎 | [`docs/plans/gui-phase2.md`](docs/plans/gui-phase2.md) | 排后期计划、讨论 L3 何时做 | ~1.5k | ✅ |
| X1 | Rust 后端技术路线 | [`docs/research/rust-backend-stack.md`](docs/research/rust-backend-stack.md) | 定 crate 选型、SSE / 存储 / 密钥 / 打包 | ~1.9k | ✅ |
| X2 | GPUI 前端技术路线 | [`docs/research/gpui-frontend-stack.md`](docs/research/gpui-frontend-stack.md) | 动 UI、升 GPUI、平台 / IME / 渲染问题 | ~2k | ✅ |
| X4 | UI 交付形态对比 | [`docs/research/ui-delivery-options.md`](docs/research/ui-delivery-options.md) | 讨论 TUI / GUI / WebView 怎么选 | ~2.5k | ✅ |
| X5 | TS 样式资产移植可行性 | [`docs/research/styling-portability.md`](docs/research/styling-portability.md) | 想复用 Web 样式 / 设计系统 | ~2.5k | ✅ |

> **迁移预告**：`X2` / `X4` / `X5` 与 `D3` / `D4` / `GUI` 都属于 UI 领域；按 `D5`，界面代码**跟功能走**——`features/*` 各自持有 `ui.rs`（`gpui` 仅出现在 `features/*` 与 `app`），不再有独立 `agent-ui` crate。因为引用只写 ID，迁移不需要改其它文档。

## 3. 任务路由（按需读）
全文见 `M1`。只在**开新任务、不确定该读哪些文档**时读。

## 4. 硬约束（始终生效）

1. **技术栈已定**：Rust（edition 2024）+ tokio 后端。UI 形态未定（D3）前**不写 UI 代码**，不引入 Electron / Tauri 等替代方案。
2. 供应商差异**不得泄漏**到编排核心（落在 `features/chat`）；`features/*` 只依赖 `A2` 的契约。
3. **一个结论只有一个家**：其它文档只许用 ID 引用，不许复制内容。
4. 未决问题只写在 `S2`；ADR 只增不改。
5. 密钥 / 真实 endpoint 不入库；对外接口流式优先；UI 线程禁止阻塞。
6. **文档跟代码走**：改某个 crate 的行为，同批次更新它的 `README.md` / `docs/`。
7. 新建 crate 时同步建它的 `README.md`，并回填 §2 地图。
