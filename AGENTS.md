# AGENTS.md — agent 入口 · 唯一地图

> 常驻上下文，必须精简。这里只有：读取协议、地图、硬约束。
> **引用文档只写 ID（如 `A2`），不写路径** —— 路径只出现在 §2 这张表里。搬文件不用改其它文档。

## 0. 读取协议

1. 读本文件 → 在 §2 地图里定位与任务相关的条目（相关 1 条就读 1 条，相关 6 条就读 6 条，**不设数量上限**）。
2. 只读这些条目指向的文档。crate 的 `README.md` 是它的门面，**先读 README 再进它的 `docs/`**；单篇先看 `TL;DR`，需要细节再往下读。
3. **按需读取，宁多勿错**。唯一的判断标准是：**「读它能改变我的做法、或让我少犯错吗？」** 能 → 读，多读几篇没问题；不能 → 不读。
   读得多是因为**相关**，不是因为"额度还剩"；读无关上下文才是错误。
   - **明确不读**（除非任务就是它们）：`X*` 调研（一次性消费、会过时，只在要重新评估选型或找依据时读）；`D*` ADR 中与本任务无关的部分；`⬜` 条目（文件不存在）；与本任务无关的 crate 的 `docs/`。
4. `⬜` = 文件不存在。复制 `docs/_template.md` 建骨架，写完回填本表状态。**不预建空文件。**
5. **状态就地更新，从不搬家**（不要用「把文件挪进某个目录」表达阶段）。状态是**分层**的：
   - 根 `STATUS.md`（`S1`）= **概览**：当前阶段的任务是什么、横跨哪些 crate。**一屏内，只写当前任务。**
   - `crates/<crate>/status.md` = **细节**：某个 crate 在这个任务里要做什么。一个任务可横跨多个 crate，各自建各自的。
   - **任务完成 → 清空或重写**，不留历史（历史交给 git）。长期排期写在 `RM`，**永远不写进 status**。
   - 未决问题仍在根 `OPEN-QUESTIONS.md`（`S2`）。

## 1. 文档归属规则（新增文档放哪）

| 影响范围 | 放哪 |
|---------|------|
| 只影响一个 crate（基础设施） | `crates/infra/<crate>/README.md`（门面）或 `crates/infra/<crate>/docs/*.md`（细节） |
| 只影响一个**功能切片** | `crates/features/<功能>/`：README.md（门面）+ `src/` + `tests/` + 可选 `docs/` |
| 影响多个 crate / 全项目 | 根 `docs/*.md` |
| 决策（只增不改） | `docs/decisions/NNNN-<slug>.md` |
| 长期计划（尚无归属 crate） | `docs/plans/*.md` |
| 当前阶段状态 · **概览** | 根 `STATUS.md` |
| 当前阶段状态 · **某 crate 的细节** | `crates/<crate>/status.md`（按需建立，随任务生灭） |
| 未决问题 | 根 `OPEN-QUESTIONS.md` |
| 里程碑与排期（**不含状态**） | `docs/roadmap.md` |
| 模板 | `docs/_template.md`、`docs/decisions/_template.md`、`crates/infra/providers/docs/providers/_template.md` |

## 2. 地图

规模为估算 token（≈KB × 350，中文），用于判断读取成本。

**ID 前缀含义**（看 ID 就知道它大概是什么、属于谁）：

| 前缀 | 含义 | 前缀 | 含义 |
|------|------|------|------|
| `S1` / `S2` | 状态概览 / 未决问题 | `P1–P6` / `PT` | 供应商文档 / 供应商模板 |
| `G1` | 目标与技术路线 | `X1–X5` | 调研（一次性消费，会过时） |
| `A1–A7` | 架构与契约（跨 crate 或归属某 crate） | `D0–D5` | 决策 ADR（只增不改） |
| `R1–R4` | 运行时（配置 / 路由 / 观测 / 限流） | `RM` | 里程碑排期（**不含状态**） |
| `Q1–Q3` | 质量（测试 / 评测 / 安全） | `GUI` | 二期 GUI 与 L3 计划 |

### 状态与排期

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| S1 | 当前阶段**概览**（只写当前任务） | `STATUS.md` | **每次开工第一眼** | ~0.5k | ✅ |
| S2 | 未决问题（全项目唯一来源） | `OPEN-QUESTIONS.md` | 需要拍板时 | ~1k | ✅ |
| RM | 里程碑与排期定义（**不含状态**） | `docs/roadmap.md` | 想知道整体排期与完成标准 | ~0.7k | ✅ |

**crate 级任务细节** = `crates/<crate>/status.md`，**按需建立、随任务生灭**，因此**不逐条登记进本表**
（否则每换一个任务就要改地图，又变成搬文件）。当前存在的几个，在 `S1` 的「当前任务」表里已链接。

### 跨 crate 规格（根 `docs/`）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| G1 | 目标、技术路线、自研边界 | `docs/goals.md` | 争论「要不要自研 / 先做哪个」 | ~2k | ✅ |
| A1 | 架构与分层 | `docs/architecture.md` | 新建模块、改目录结构 | ~1.6k | ✅ |
| A1-V | 架构图（`A1`/`RM`/`S1` 的**可视化**，离线 HTML） | `docs/architecture-map.html` | 想一眼看懂分层 / 依赖 / 进度 | 1 张图 | ✅ |
| Q1 | 测试策略 | `docs/testing.md` | 写单测、契约测试、加适配器前 | ~1.4k | ✅ |
| Q2 | 效果评测与回归 | `docs/eval.md` | 建评测集、prompt 回归 | — | ⬜ |
| Q3 | 安全与合规 | `docs/security.md` | 密钥、审计、数据合规 | — | ⬜ |
| T0 | 文档模板 | `docs/_template.md` | 新建任何文档 | ~0.3k | ✅ |

### `crates/common`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | crate 门面 | `crates/common/README.md` | 改契约类型前 | ~0.7k | ✅ |
| A3 | 消息与多模态协议 | `crates/common/docs/message-protocol.md` | 改消息 / 内容块 / tool_call_id | ~1.5k | ✅ |

### `crates/infra/providers`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | crate 门面 | `crates/infra/providers/README.md` | 加供应商、改 trait | ~0.8k | ✅ |
| A2 | Provider 接入契约 | `crates/infra/providers/docs/contract.md` | 改调用接口、新增供应商 | ~2.6k | ✅ |
| X3 | 多模型抽象路线调研 | `crates/infra/providers/docs/abstraction-research.md` | 质疑要不要自研抽象 | ~2.3k | ✅ |
| PT | 供应商文档模板 | `crates/infra/providers/docs/providers/_template.md` | 新建某家供应商文档 | ~0.4k | ✅ |
| P1–P6 | 各供应商差异 | `crates/infra/providers/docs/providers/*.md` | 接某一家模型时 | — | ⬜ |

### `crates/infra/transport`（M2 正在建）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | crate 门面 | `crates/infra/transport/README.md` | 动传输层 / SSE 前 | ~0.6k | ✅ |
| A4 | 流式、取消与超时 | `crates/infra/transport/docs/streaming.md` | 动 SSE / 中断 / 超时 | — | ⬜ |
| R4 | 缓存、限流与并发 | `crates/infra/transport/docs/cache-and-ratelimit.md` | 重复请求、QPS 控制 | — | ⬜ |

### `crates/features/*`（功能切片：一个功能 = 一个文件夹，见 `D5`）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | 对话功能门面 | `crates/features/chat/README.md` | 改对话逻辑 / 界面时 | ~0.6k | ✅ 骨架 |
| — | 会话功能门面 | `crates/features/sessions/README.md` | 改会话列表 / 切换时 | ~0.5k | ✅ 骨架 |
| — | 设置功能门面 | `crates/features/settings/README.md` | 加设置项 / 改密钥录入时 | ~0.5k | ✅ 骨架 |
| A5 | 工具调用 | `crates/features/chat/docs/tool-calling.md` | 工具定义、并行调用、结果回填 | — | ⬜ |
| A7 | 上下文与 Token | `crates/features/chat/docs/context-and-tokens.md` | 裁剪、摘要、计费 | — | ⬜ |

### `crates/infra/*`（其他基础设施，按需创建）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| A6 | 错误、重试与降级 | `crates/infra/transport/docs/errors-and-fallback.md` | 限流 / 超时 / 审核 / 不可用 | — | ⬜ |
| R1 | 配置与密钥 | `crates/infra/config/docs/config-and-secrets.md` | 加配置项、接密钥 | — | ⬜ |
| R2 | 模型注册表与路由 | `crates/infra/routing/docs/model-registry.md` | 模型清单、路由与兜底 | — | ⬜ |
| R3 | 可观测性与成本 | `crates/infra/telemetry/docs/observability.md` | 日志 / trace / 计费 | — | ⬜ |

### `crates/app`

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| — | 二进制入口门面 | `crates/app/README.md` | 想跑起来看一眼、新增 CLI 命令 | ~0.5k | ✅ |

### 决策记录（只增不改）

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| D0 | ADR 模板 | `docs/decisions/_template.md` | 写新 ADR | ~0.2k | ✅ |
| D1 | 技术栈：Rust + GPUI | `docs/decisions/0001-tech-stack.md` | 质疑语言 / 框架选型 | ~0.9k | ✅ 已接受（UI 待复核） |
| D2 | 多模型抽象：契约自研 | `docs/decisions/0002-llm-abstraction.md` | 是否引入 rig / genai | ~1k | ✅ 已接受 |
| D3 | UI 交付形态 | `docs/decisions/0003-ui-delivery-form.md` | 定 UI 技术、讨论分阶段 | ~0.9k | 🚧 待决 |
| D4 | 样式层建设路线 | `docs/decisions/0004-styling-layer.md` | 样式怎么组织、能否复用 TS 生态 | ~0.6k | 🚧 待确认 |
| D5 | 代码按功能组织（Django 式） | `docs/decisions/0005-feature-oriented-layout.md` | 新建 crate / 功能、移动目录时 | ~1.6k | ✅ 已接受 |

### 计划与跨领域调研

| ID | 标题 | 路径 | 何时读 | 规模 | 状态 |
|----|------|------|--------|------|------|
| GUI | 二期 GUI + L3 样式引擎 | `docs/plans/gui-phase2.md` | 排后期计划、讨论 L3 何时做 | ~1.5k | ✅ |
| X1 | Rust 后端技术路线 | `docs/research/rust-backend-stack.md` | 定 crate 选型、SSE / 存储 / 密钥 / 打包 | ~1.9k | ✅ |
| X2 | GPUI 前端技术路线 | `docs/research/gpui-frontend-stack.md` | 动 UI、升 GPUI、平台 / IME / 渲染问题 | ~2k | ✅ |
| X4 | UI 交付形态对比 | `docs/research/ui-delivery-options.md` | 讨论 TUI / GUI / WebView 怎么选 | ~2.5k | ✅ |
| X5 | TS 样式资产移植可行性 | `docs/research/styling-portability.md` | 想复用 Web 样式 / 设计系统 | ~2.5k | ✅ |

> **迁移预告**：`X2` / `X4` / `X5` 与 `D3` / `D4` / `GUI` 都属于 UI 领域；`crates/agent-ui` 落地后整体迁入该 crate。因为引用只写 ID，迁移不需要改其它文档。

## 3. 路由（任务 → 加载清单）

| 任务 | 加载 |
|------|------|
| 现在做什么 / 下一步 | `S1` → 命中任务后进它所链接的 `crates/<crate>/status.md` |
| 整体排期 / 完成标准 | `RM` |
| 一眼看懂整体架构 / 当前进度 | `A1-V`（图）；事实仍以 `A1` + `RM` + `S1` 为准 |
| 等用户拍板的事 | `S2` |
| 新增 / 修改供应商 | `A2` → `PT` → 对应 `P*` → `X3` |
| 写传输 / SSE / 重试 | `A4`（⬜ 先建）→ `Q1` → `A2` |
| 改契约类型 | 对应 crate 的 README → `A2` / `A3` |
| 新增功能 / 新建 feature | `D5` → 该功能的 `crates/features/<功能>/README.md` |
| 新建模块 / 改目录结构 | `A1` §5 → `D5` → `G1` |
| 决定 UI 技术 | `D3` → `X4`（GPUI 细节 → `X2`） |
| 样式怎么组织 | `D4` → `X5` |
| 「自研还是找现成轮子」 | `G1` §3 → `X1`/`X2`/`X3` |
| 长期排期 / L3 | `RM` → `GUI` |
| 推翻某个决定 | 对应 `D*` → 写**新** ADR（旧的不改） |

## 4. 硬约束（始终生效）

1. **技术栈已定**：Rust（edition 2024）+ tokio 后端。UI 形态未定（D3）前**不写 UI 代码**，不引入 Electron / Tauri 等替代方案。
2. 供应商差异**不得泄漏**到 `agent-core`；核心只依赖 `A2` 的契约。
3. **一个结论只有一个家**：其它文档只许用 ID 引用，不许复制内容。
4. 未决问题只写在 `S2`；ADR 只增不改。
5. 密钥 / 真实 endpoint 不入库；对外接口流式优先；UI 线程禁止阻塞。
6. **文档跟代码走**：改某个 crate 的行为，同批次更新它的 `README.md` / `docs/`。
7. 新建 crate 时同步建它的 `README.md`，并回填 §2 地图。
