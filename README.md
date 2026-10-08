# CodingRocket · 多模型接入客户端

> 人类入口，1 分钟。**文档地图在 [`AGENTS.md`](AGENTS.md) §2** —— 人和 agent 共用那一张表。
> 定名见 `S2` Q7（2026-10-07）：crate 包名（`agent-*`）是内部技术名，产品名以本标题为准。

## 这是什么

跨平台桌面客户端，统一接入多家大模型（OpenAI 兼容族 / Anthropic / 本地模型），支持流式对话、多会话、模型热切换与工具调用。本地优先：密钥进系统钥匙串，历史进本地 SQLite。

## 现在到哪

状态**只在一处维护**，本文件不重复（重复必然腐烂）：

| 想知道 | 去哪 |
|--------|------|
| 现在做什么、下一步 | [`STATUS.md`](STATUS.md)（一屏内） |
| 等你拍板的事 | [`OPEN-QUESTIONS.md`](OPEN-QUESTIONS.md) |
| 整体排期与「完成」的定义 | [`docs/roadmap.md`](docs/roadmap.md) |

## 文档放哪（约定）

**文档跟着代码走**，不集中堆在一个 `docs/` 里：

| 位置 | 放什么 |
|------|--------|
| [`STATUS.md`](STATUS.md)（根） | 阶段**概览**：当前任务是什么、跨哪些 crate。**只写当前任务，完成即重写** |
| `crates/<类别>/<crate>/status.md` | 该 crate 在当前任务里的**细节**（按需建立，随任务生灭） |
| [`OPEN-QUESTIONS.md`](OPEN-QUESTIONS.md)（根） | 未决问题，固定路径、就地更新 |
| [`docs/roadmap.md`](docs/roadmap.md) | 里程碑与排期定义（**不含状态**） |
| `crates/features/<功能>/` | **语义**（`D6`）：一个功能一个文件夹 —— README + `src/{service,repo,ui,领域类型}.rs` + `tests/` 同目录 |
| `crates/kernel/<crate>/` | **机制**（`D6`）：README.md（门面）+ `docs/*.md`（规格细节）。不许出现业务名词 |
| `docs/*.md`（根） | 跨 crate 的规格：架构、测试、目标 |
| `docs/decisions/` | ADR，只增不改 |
| `docs/plans/` | 尚无归属 crate 的长期计划 |
| `docs/research/` | 跨领域调研（其余调研放在它影响的那个 crate 里） |
| [`docs/_template.md`](docs/_template.md) | 新文档模板 |

**引用规则**：文档之间只写 ID（如 `A2`、`S1`），**不写路径**。路径只在 [`AGENTS.md`](AGENTS.md) §2 的地图里出现一次 —— 这样搬文件不用改别的文档。

## 目录树

```
README.md                 人类入口（本文件）
AGENTS.md                 agent 入口 + 唯一地图
STATUS.md                 当前阶段与下一步
OPEN-QUESTIONS.md         未决问题
docs/
  _template.md            文档模板
  goals.md                目标、技术路线、自研边界
  architecture.md         架构与分层
  testing.md              测试策略
  decisions/              ADR（0001 技术栈 / 0002 抽象路线 / 0003 UI 形态 / 0004 样式层）
  plans/gui-phase2.md     二期 GUI + L3 样式引擎
  research/               跨领域调研（Rust 后端栈、GPUI、UI 形态、样式可移植性）
crates/
  common/                 契约与纯类型（无 IO）：消息 / 事件 / 错误（+ docs/message-protocol.md）
  kernel/                 机制内核：跨功能复用的技术能力，**不许出现业务名词**（D6）
    transport/            HTTP / SSE / 超时 / 取消 / 重试（M2 在建）
    providers/            Provider trait 与适配器（+ docs/contract.md、docs/providers/）
    （routing / store / config / telemetry 按需创建）
  features/               语义层：一个功能一个文件夹（垂直切片，D5 + D6）
    chat/                 一次对话：service + repo + ui + 领域类型 + tests
    sessions/             会话：列表 / 切换 / 命名
    settings/             设置项与密钥录入
  app/                    二进制入口与冒烟自检
```

## 快速验证

```powershell
cargo test --workspace                            # 4 个单测
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p agent-app -- self-check              # 契约层冒烟
```
