# D5 · 代码按功能组织（Django 式垂直切片）

| | |
|---|---|
| ID | `D5` |
| 类型 | 决策（**只增不改**） |
| 状态 | **已接受**（2026-09-28 确认方案 C；迁移已完成） |
| 更新 | 2026-09-28 |
| 何时读 | 新建 crate / 新增功能、移动目录结构时 |
| 规模 | ~1.6k token |

> **已被 `D6` 部分修订**（2026-09-28）：命名 `infra/*` → `kernel/*`，并补上「什么算功能 / 什么算机制」的判据与 feature 内部文件布局。
> 本文**原文不改**（ADR 只增不改）；两处冲突时以 `D6` 为准。

## 背景

原先（`A1` §5）按**架构层**切 crate：`schema / providers / transport / core / routing / store / config / observability / ui / app`。

后果：**实现一个功能要横跨多个 crate 文件夹**，每个 crate 还各有一份 `Cargo.toml` + `README.md`。负担来自"按层切"，不是来自 crate 本身。

诉求：像 Django app 那样——`models / service / views / tests` 同目录，**做一件事只看一个文件夹**。

## 备选方案

| 方案 | 说明 | 优点 | 缺点 |
|------|------|------|------|
| A 保持按层 | 每层一个 crate | 依赖方向天然清晰；基础设施集中 | 功能改动跨多个文件夹（痛点） |
| B 单 crate + 功能模块 | `src/features/<f>/{service,ui,model,tests}.rs` | 最接近 Django；无每功能 `Cargo.toml` 开销 | 编译单元单一；模块间可成环，边界靠纪律 |
| **C 每功能一个 crate** | `crates/features/<f>/` 内含 `service / ui / tests / fixtures` | 一个功能一个文件夹；**crate 依赖不能成环（编译期硬约束）**；测试就地；增量编译好 | 每个功能一个 `Cargo.toml`；跨 feature 调用要设计 pub API |

## 决定

采纳 **方案 C**。三层目录语义，`crate 目录本身表达"它属于哪一类"`：

```
crates/
  common/                 # 契约与纯类型（无 IO）—— 原 agent-schema，包名 agent-common
    src/{message,model,completion,stream,error}.rs
    docs/message-protocol.md
  kernel/                  # 基础设施：被 features 复用
    transport/            #   HTTP / SSE / 超时 / 取消 / 重试
    providers/            #   供应商适配器 + 注册表 + 模型清单
    routing/              #   模型注册表查询、能力协商、路由与兜底
    store/                #   SQLite 会话与消息持久化
    config/               #   配置 + 密钥（keyring）
    telemetry/            #   tracing / token / 成本
  features/               # 用户可见功能：**一个功能 = 一个文件夹**（Django app）
    chat/                 #   一次对话：编排 + 状态 + ui + tests
    sessions/             #   会话列表与管理
    settings/             #   设置项与密钥录入
  app/                    # 二进制入口：组装 kernel + features + UI（包名 agent-app）
```

**依赖方向（硬规则）**

```
app  →  features/*  →  kernel/*  →  common        （只能向下，不能反向）
```

- `common` 不依赖任何人（契约纯净度靠这条保证）。
- feature **之间可以互相依赖**（Django 里 app 也互相 import），但**不可能成环**——crate 依赖图天生无环。这是选 C 而非 B 的核心理由。
- `kernel/*` 不得依赖 `features/*`；UI 技术（`gpui` 等）只允许出现在 `features/*` 与 `app`。

**已确认的首批 feature**（2026-09-28）：`chat` / `sessions` / `settings`，三者目前都是**骨架**（只有边界说明与 README 待办，不预写实现）。

## 「一个功能一个文件夹」到什么程度

| 典型改动 | 落在哪 | 跨几个文件夹 |
|---------|--------|-------------|
| 加一个功能 | `crates/features/<新功能>/` | **1** |
| 在聊天里加图片输入 | `features/chat/`（契约要加能力位时才带上 `common/`） | 1（偶发 2） |
| 加一家 OpenAI 兼容供应商 | `kernel/providers/`（模型清单 + 一行注册） | **1** |
| 接一个异构协议（Anthropic 原生） | `kernel/providers/` | **1** |
| 改存储格式 | `kernel/store/` | **1** |
| 改 UI 主题 | `features/*/ui` + `common` 的主题 token | 2 |

> **原则**：跨文件夹是**真实耦合的信号**。目标不是消灭它，而是让**常见改动只落在一处**。硬凑"永不跨目录"只会把耦合藏起来。

## 规格文档归属映射（按本决策）

| 规格 | 新位置 |
|------|--------|
| `A1` 架构与分层 | 根 `docs/architecture.md`（跨 crate） |
| `A2` Provider 契约 | `crates/kernel/providers/docs/contract.md` |
| `A3` 消息协议 | `crates/common/docs/message-protocol.md` |
| `A4` 流式 / 取消 / 超时 | `crates/kernel/transport/docs/streaming.md` |
| `A5` 工具调用 | `crates/features/chat/docs/tool-calling.md` |
| `A6` 错误 / 重试 / 降级 | `crates/kernel/transport/docs/errors-and-fallback.md` |
| `A7` 上下文与 Token | `crates/features/chat/docs/context-and-tokens.md` |
| `R1` 配置与密钥 | `crates/kernel/config/docs/config-and-secrets.md` |
| `R2` 模型注册表与路由 | `crates/kernel/routing/docs/model-registry.md` |
| `R3` 可观测性与成本 | `crates/kernel/telemetry/docs/observability.md` |
| `R4` 缓存 / 限流 / 并发 | `crates/kernel/transport/docs/cache-and-ratelimit.md` |
| `P*` 各供应商差异 | `crates/kernel/providers/docs/providers/*.md` |
| `Q1`–`Q3` 质量 | 根 `docs/`（跨 crate） |

## 与 UI 形态的关系（不阻塞 `D3`）

每个 feature 自带 `ui.rs`——**界面代码跟着功能走**，这是本次诉求的核心。

- `D3` 选 **TUI**：`ui.rs` 是 Ratatui 视图。
- 选 **GPUI**：`ui.rs` 是 GPUI 视图（`gpui` 只被 `features/*` 与 `app` 依赖）。
- 选 **Tauri**：feature 边界即前后端接口边界（feature 暴露 command，Web 前端只调它）。

## 影响

- **迁移已完成**：`agent-schema` → `crates/common`（包名 `agent-common`）；`agent-providers` → `crates/kernel/providers`；`agent-transport` → `crates/kernel/transport`；`agent-app` → `crates/app`。均用 `git mv` 保留历史。
- 新建三个 feature crate 骨架：`features/{chat,sessions,settings}`。
- 每个 feature 的 `README.md` = "这个功能的门面"（边界 / 依赖 / 待办）。
- **不预建空目录**：`kernel/store`、`kernel/config`、`kernel/routing`、`kernel/telemetry` 只在该功能开工时创建。

## 未决项

- `features/chat` 与 `sessions` 的边界细节：会话标题自动生成属于谁？（建议：生成逻辑在 `sessions`，触发在 `chat` 落盘时）
- 是否把"模型管理页"独立成 feature（当前：模型清单属 `kernel/providers`，选择动作在 `chat` 的 UI）
