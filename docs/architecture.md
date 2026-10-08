# A1 · 架构总览

| | |
|---|---|
| ID | `A1` |
| 类型 | 规格（跨 crate） |
| 状态 | ✅ 已定稿（随里程碑增补） |
| 更新 | 2026-09-28 |
| 何时读 | 新建模块、改目录结构、讨论分层边界时 |
| 规模 | ~1.6k token |

## 目的

给出分层与边界，让「接入第 N 家模型」变成加一个适配器，而不是改核心。

## 适用范围

- 包含：分层职责、关键对象、一次调用的数据流、目录结构草案。
- 不包含：具体接口签名（见 `A2`）、厂商差异（见 `P*`）、路由策略（见 `R2`，待创建）。

## 1. 目标 / 非目标

目标：

- 统一调用面：核心只认一套内部消息与事件。
- 供应商可插拔：新增厂商不改核心。
- 能力可协商：不是所有模型都支持 vision / tools / json / reasoning。
- 可观测、可降级、可计费。

非目标：

- 不做训练 / 微调。
- 不内置向量库（交给外部 Retriever）。
- 不做前端 UI（独立文档）。

## 2. 分层

```
App / CLI
   ↓
Agent Core            循环、规划、记忆、工具编排
   ↓
Capability & Policy   能力校验、路由、重试/降级、预算控制
   ↓
Provider Adapter      每家一个，实现统一契约（A2）
   ↓
Transport             HTTP / SSE / WS、超时、限流、重试
   ↕
Observability         trace / 日志 / 指标 / 成本
```

依赖规则：

- 依赖只能向下，禁止反向依赖。
- Provider 之间禁止互相引用。
- Core 禁止 import 任何 provider 私有类型。

## 3. 关键对象

| 对象 | 职责 |
|------|------|
| `InternalMessage` / `ContentPart` | 归一化的输入（文本 / 图片 / 文件 / 工具结果） |
| `ModelRequest` / `ModelResponse` / `StreamEvent` | 一次调用的输入输出 |
| `ModelProvider` | 供应商适配器契约 |
| `ModelRegistry` | 模型元信息：上下文窗口、能力、价格、上下线状态 |
| `Router` | 按能力 / 成本 / 可用性选择 provider + model |

## 4. 一次调用的数据流

1. 对话切片（`crates/features/chat`）组装 `InternalMessage[]`。
2. Registry 校验能力（是否需要 vision / tools / json）。
3. Router 选定 provider + model（含兜底链）。
4. Adapter 转换为厂商格式 → Transport 发送。
5. 流式回传 → 归一化为 `StreamEvent`。
6. Observability 记录 token / latency / cost；失败时交 Policy 决定重试或降级。

## 5. 目录结构（已定稿）

按 `D5` + `D6`：**代码按功能组织（垂直切片）**，目录本身表达它属于哪一类。

```
crates/
  common/                 契约与纯类型（无 IO，包名 agent-common）
  kernel/                 机制内核：跨功能复用的技术能力，**不许出现业务名词**（`D6` §1）
    transport/            HTTP / SSE / 超时 / 取消 / 重试
    providers/            供应商适配器 + 注册表 + 模型清单
    routing/              模型注册表查询、能力协商、路由与兜底
    store/                SQLite 连接 / 迁移 / 事务（**不是**业务查询）
    config/               配置文件与钥匙串读写
    telemetry/            tracing 初始化 / token 成本
  features/               语义层：一个功能一个文件夹（垂直切片）
    chat/                 service.rs + repo.rs + ui.rs + 领域类型 + tests/
    sessions/             同上
    settings/             同上
  app/                    二进制入口（包名 agent-app，组装 kernel + features + UI）
```

**依赖方向（硬规则）**：`app → features/* → kernel/* → common`，只能向下。

- `common` 不依赖任何人（契约纯净度靠这条保证）。
- feature 之间可以互相依赖（Django 里 app 也互相 import），但**不可能成环**——crate 依赖图天生无环，这是 `D5` 选"每功能一个 crate"的核心理由。
- `kernel/*` 不得依赖 `features/*`；UI 依赖（`gpui` 等）只允许出现在 `features/*` 与 `app`。
- **落点判据（`D6` §1）**：这是「怎么发 HTTP / 怎么连库 / 怎么读钥匙串」（机制）→ `kernel/*`；这是「对话怎么进行、会话怎么命名」（语义）→ `features/<功能>/`。
- feature 内部文件布局见 `D6` §3；**按需创建，不预建空文件**（`ui.rs` 等 `D3`，`repo.rs` 等真需要持久化）。
- 规格文档归属映射见 `D5`。

**文档归属**：crate 相关的规格放在该 crate 的 `README.md` / `docs/` 下，跨 crate 的放在根 `docs/`；判定规则见 [`AGENTS.md`](../AGENTS.md) §1。阶段状态不靠搬文件表达（见 `S1`）。

## 6. 约束

- 供应商差异不得泄漏到 Core（见 [`AGENTS.md`](../AGENTS.md) §4）。
- 每个 adapter 必须可脱离网络单测（用 fixtures + mock transport）。
- 所有对外调用必须支持取消（`tokio_util::sync::CancellationToken`）。
- 上下文超限属于可恢复错误，应先裁剪/摘要再重试一次。
- UI 线程禁止阻塞；UI 依赖只允许出现在 `features/*` 与 `app`（见 `D5`）。

## 7. 未决项

是否内置 MCP 客户端、记忆存储方案、首批适配器范围 → 统一登记在 `S2`。
