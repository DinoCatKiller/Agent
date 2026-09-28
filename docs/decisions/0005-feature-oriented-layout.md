# D5 · 代码按功能组织（Django 式垂直切片）

| | |
|---|---|
| ID | `D5` |
| 类型 | 决策（**只增不改**） |
| 状态 | **提案（待确认）** |
| 更新 | 2026-09-28 |
| 何时读 | 新建 crate / 新增功能、移动目录结构时 |
| 规模 | ~1.5k token |

## 背景

现状（`A1` §5）按**架构层**切 crate：`schema / providers / transport / core / routing / store / config / observability / ui / app`。

后果：**实现一个功能要横跨多个 crate 文件夹**，每个 crate 还各有一份 `Cargo.toml` + `README.md`。心智负担来自"按层切"，不是来自 crate 本身。

诉求：像 Django app 那样——`models / service / views / tests` 在同一个目录里，**做一件事只看一个文件夹**。

## 备选方案

| 方案 | 说明 | 优点 | 缺点 |
|------|------|------|------|
| A 保持按层（现状） | 每层一个 crate | 依赖方向天然清晰；基础设施集中好找 | 功能改动跨多个文件夹（当前痛点） |
| B 单 crate + 功能模块 | `src/features/<f>/{service,ui,model,tests}.rs` | 最接近 Django；无每功能 `Cargo.toml` 开销 | 编译单元只有一个（增量编译随代码量变差）；模块之间可成环，边界只能靠纪律 |
| **C 每功能一个 crate** | `crates/features/<f>/` 内含 `service / ui / tests / fixtures` | 一个功能一个文件夹；**crate 依赖不能成环（编译期硬约束）**；测试与 fixtures 就地；增量编译好 | 每个功能一个 `Cargo.toml`；跨 feature 调用要设计好 pub API |

## 决定（建议采纳 C）

引入三层目录语义，**crate 目录本身表达"它是哪一类"**：

```
crates/
  common/                 # 契约与纯类型（无 IO）—— 原 agent-schema 升级为共享基础
    src/{message,model,completion,stream,error}.rs
    docs/message-protocol.md
  infra/                  # 基础设施：被 features 复用；features 之间不互相依赖
    transport/            #   HTTP / SSE / 超时 / 取消 / 重试
    providers/            #   供应商适配器 + 注册表 + 模型清单
    store/                #   SQLite 会话与消息持久化
    config/               #   配置 + 密钥（keyring）
    telemetry/            #   tracing / token / 成本
  features/               # 用户可见功能：**一个功能 = 一个文件夹**（Django app）
    chat/                 #   src/{service,ui,tests}.rs + tests/fixtures/ + README.md
    sessions/             #   会话列表与管理
    settings/             #   设置界面（密钥、代理、主题）
  app/                    # 二进制入口：组装 infra + features + UI
```

**依赖方向（硬规则）**

```
app  →  features/*  →  infra/*  →  common        （只能向下，不能反向）
```

- `common` 不依赖任何人（契约的纯净度靠这条保证）。
- feature **之间可以互相依赖**（Django 里 app 也互相 import），但**不可能成环**——crate 依赖图天生无环，这是选 C 而非 B 的核心理由。
- `infra/*` 不得依赖 `features/*`。

## 「一个功能一个文件夹」到什么程度

| 典型改动 | 落在哪 | 跨几个文件夹 |
|---------|--------|-------------|
| 加一个功能 | `crates/features/<新功能>/` | **1** |
| 在聊天里加图片输入 | `features/chat/`（契约要加能力位时才会带上 `common/`） | 1（偶发 2） |
| 加一家 OpenAI 兼容供应商 | `infra/providers/`（模型清单 + 一行注册） | **1** |
| 接一个异构协议（如 Anthropic 原生） | `infra/providers/` | **1** |
| 改存储格式 | `infra/store/` | **1** |
| 改 UI 主题 | `features/*/ui` + `common` 的主题 token | 2 |

> **原则**：跨文件夹是**真实耦合的信号**。目标不是消灭它，而是让**常见改动只落在一处**。硬凑"永不跨目录"只会把耦合藏起来。

## 与 UI 形态的关系（不阻塞 `D3`）

每个 feature 自带 `ui.rs`——**界面代码跟着功能走**，这正是本次诉求的核心。

- 若 `D3` 选 **TUI**：`ui.rs` 就是 Ratatui 视图。
- 若选 **GPUI**：`ui.rs` 是 GPUI 视图（`gpui` 只被 `features/*` 与 `app` 依赖，`common`/`infra` 永远不碰 UI）。
- 若选 **Tauri**：feature 边界同时就是前后端接口边界（feature 暴露 command，Web 前端只调它）。

## 影响

- **迁移**（用 `git mv` 保历史）：`agent-schema` → `crates/common`；`agent-providers` → `crates/infra/providers`；`agent-app` → `crates/app`。包名相应为 `agent-common` / `agent-providers` / `agent-app`（目录名按功能，包名保持 `agent-*` 前缀避免与依赖撞名）。
- `AGENTS.md` §2 地图的路径要更新**一次**；因为其它文档只写 ID，**不需要改**。
- 每个 feature 的 `README.md` 成为"这个功能的门面"（对外接口 / 依赖 / 怎么单测）。
- **不预建空目录**：`sessions/`、`settings/`、`store/` 等只在该功能开工时才创建（沿用现有规则）。

## 待确认（→ `S2` Q8）

1. 是否采纳方案 C（每功能一个 crate），而不是 B（单 crate + 模块）？
2. `features` 的首批划分：`chat` 单独即可，还是同时切出 `sessions` / `settings`？
3. 模型选择行为归属：建议**模型清单**放 `infra/providers`，**选择动作**放 `features/chat` 的 UI，暂不单列 `models` feature（等有模型管理页再说）。
