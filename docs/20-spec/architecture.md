# A1 · 架构总览

> 索引 ID：`A1` ｜ 状态：🚧 草案 ｜ 最后更新：2026-09-27

## 目的

给出分层与边界，让「接入第 N 家模型」变成加一个适配器，而不是改核心。

## 适用范围

- 包含：分层职责、关键对象、一次调用的数据流、目录结构草案。
- 不包含：具体接口签名（见 `20-spec/provider-contract.md`）、厂商差异（见 `20-spec/providers/`）、路由策略（见 `20-spec/model-registry.md`，待创建）。

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

1. Core 组装 `InternalMessage[]`。
2. Registry 校验能力（是否需要 vision / tools / json）。
3. Router 选定 provider + model（含兜底链）。
4. Adapter 转换为厂商格式 → Transport 发送。
5. 流式回传 → 归一化为 `StreamEvent`。
6. Observability 记录 token / latency / cost；失败时交 Policy 决定重试或降级。

## 5. 目录结构（已定稿）

技术栈已定：Rust edition 2024 + tokio + GPUI（见 `30-decisions/0001-tech-stack.md`），cargo workspace 布局如下（完整说明见 `40-research/rust-backend-stack.md` §3）：

```
agent/
  crates/
    agent-schema/        内部消息 / 事件 / 错误类型（契约唯一来源）
    agent-core/          agent 循环、工具编排（不依赖任何 provider 与 UI）
    agent-providers/     Provider trait + 各厂商 adapter
    agent-routing/       模型注册表、能力协商、路由与兜底
    agent-transport/     reqwest 封装、SSE 解析、重试、超时
    agent-store/         SQLite 会话/消息持久化
    agent-config/        配置 + keyring 密钥
    agent-observability/ tracing、token/成本统计
    agent-ui/            GPUI 视图层（**唯一**接触 GPUI 的 crate）
    agent-app/           二进制入口，组装依赖
  tests/                 契约测试 + fixtures
```

依赖只能向下；`agent-core`、`agent-schema` 不得依赖具体实现 crate。

## 6. 约束

- 供应商差异不得泄漏到 Core（见 `AGENTS.md` §3）。
- 每个 adapter 必须可脱离网络单测（用 fixtures + mock transport）。
- 所有对外调用必须支持取消（`tokio_util::sync::CancellationToken`）。
- 上下文超限属于可恢复错误，应先裁剪/摘要再重试一次。
- UI 线程禁止阻塞；GPUI API 只允许出现在 `agent-ui`。

## 7. 未决项

是否内置 MCP 客户端、记忆存储方案、首批适配器范围 → 统一登记在 [`S2`](../10-now/open-questions.md)。
