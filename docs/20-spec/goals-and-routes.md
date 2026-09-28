# G1 · 目标与技术路线

> 索引 ID：`G1` ｜ 状态：🚧 待确认 ｜ 最后更新：2026-09-27

## 1. 目标

### 1.1 产品目标（待确认）

- 跨平台桌面应用：**Rust 做后端 + GPUI 做前端**，Windows / macOS / Linux 同一份代码。
- 统一接入多家模型：OpenAI 原生协议、OpenAI 兼容协议族、Anthropic 原生、本地模型（Ollama / llama.cpp）。
- 流式对话、多会话管理、模型热切换、工具调用（function calling）。
- 本地优先：密钥进系统钥匙串，会话历史进本地 SQLite，不依赖云端服务。

### 1.2 工程目标（可验收）

| 编号 | 目标 | 验收方式 |
|------|------|---------|
| G-1 | Core 不依赖任何供应商私有类型 | `agent-core` 的 `Cargo.toml` 里不出现任何 provider crate |
| G-2 | 契约编译期固化 | 新增适配器必须 `impl Provider`，方法缺失直接编译失败 |
| G-3 | 适配器可离线测试 | 用 fixtures 回放，断网可跑全绿 |
| G-4 | UI 永不阻塞 | 网络/流式全部跑在 tokio，UI 线程只做渲染 |
| G-5 | 新增供应商成本可量化 | 只新增一个 adapter 文件 + 注册表一行，不动 Core |

### 1.3 非目标

- 不做模型训练 / 微调。
- 不内置向量库 / RAG（留接口，不实现）。
- 不做移动端与 Web 端。

## 2. 技术路线总览

| # | 路线 | 候选 | 调研结论 | 决策 | 详见 |
|---|------|------|---------|------|------|
| 1 | 语言与运行时 | Rust + tokio + serde | 事实标准，IO 密集型场景适配 | ✅ 采用 | ADR-001 |
| 2 | UI 框架 | 裸 `gpui` / `gpui-kit` | 组件库会锁定匹配的 GPUI 版本，能隔离部分破坏性变更 | ✅ `gpui-kit`（内含 `gpui-component`） | X2 |
| 3 | 供应商抽象 | 自研 trait / `rig-core` / `genai` / `async-openai` | `rig` 归一化最完整，但引入后仍要适配自有契约与流式 UI 语义 | ✅ 自研薄抽象 + 借鉴 rig 的归一化设计 | ADR-002 |
| 4 | HTTP 与流式 | `reqwest` + `eventsource-stream` / `sse-rs` | SSE 是三家共识；差异在事件语义，需自己归一 | ✅ `reqwest` + 薄封装解析（可换 `sse-rs`） | X1 |
| 5 | GPUI ↔ tokio 桥 | 自研 / `gpui-tokio-bridge` / `guic-gpui-tokio` | 第三方均 0.1–0.3 版本，过于年轻 | ✅ 自研（~50 行，复用 GPUI 自带 executor） | X2 |
| 6 | 本地存储 | `rusqlite` / `sqlx` | 单机桌面无连接池需求 | ✅ `rusqlite`（bundled SQLite） | X1 |
| 7 | 密钥存储 | `keyring-rs` / 自加密文件 | 系统钥匙串是桌面端标准做法 | ✅ `keyring`，失败时降级到加密文件 | X1 |
| 8 | Markdown / 编辑器 | `gpui-component` 内置 / 自研 | 原生 Markdown + HTML 渲染、Tree-sitter 高亮已具备 | ✅ 直接复用 | X2 |
| 9 | 打包分发 | `cargo-wix` (MSI) / MSIX / `cargo-dist` | Windows 侧 `cargo-wix` 最成熟 | ✅ `cargo-wix`（后续补签名） | X1 |
| 10 | 可观测 | `tracing` / OpenTelemetry | `tracing` 零成本且生态通用 | ✅ `tracing`，OTel 后续按需接 | X1 |

## 3. 自研边界（关于「造轮子」）

**必须自研**（现成方案解决不了或语义不对）：

1. 统一 Provider 契约与归一化消息/事件模型（`A2`）。
2. 流式增量状态机：文本增量、推理增量、**工具参数分片拼接**、截断/错误收尾语义。
3. 错误分类 + 重试/降级策略（各家 `finish_reason` / 错误体不同）。
4. Agent 循环与工具编排（多轮 tool_result 回填）。
5. 模型注册表与路由（能力、成本、可用性、兜底链）。
6. 会话 UI 状态机（GPUI `Entity` 树 + 流式增量渲染节流）。

**建议复用**（自研性价比低）：

- `tokio` / `reqwest` / `serde` + `schemars`（JSON Schema 生成）/ `keyring` / `rusqlite` / `tracing`。
- GPUI 渲染、文本系统、布局、组件库（`gpui-kit`）。
- SSE 字节→事件解析：先用成熟 crate，语义归一自己做（分界清晰）。

**谨慎自研**（成本可控但要设停止条件）：

- SSE 解析器：若 `eventsource-stream` / `sse-rs` 在中断、超时、坏帧上不满足要求，自研解析器约 200–400 行可覆盖，属于可接受范围。
- GPUI ↔ tokio 桥：GPUI 自带 `executor`，桥接层很薄，不值得依赖 0.1 版第三方。
- 重试/退避：若通用 crate（如 `backoff`）语义不贴合，自研约 100 行。

**不建议自研**：GPU 渲染管线、文本整形、TLS、SQLite 引擎、JSON Schema 子集清洗（先借鉴 rig 的 `sanitize_schema` 思路自己实现小版本即可）。

## 4. 已识别风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| GPUI 仍 pre-1.0，版本间常有破坏性变更 | 升级即返工 | 固定版本；只经 `gpui-kit` 间接依赖 GPUI；把 GPUI 调用收敛到一个 `ui` crate 内 |
| GPUI 官方文档滞后（`docs.rs` 写「需 macOS/Linux」，README 写 Windows 无需 feature） | 环境判断出错 | **实测为准**：Windows 用 Win32 + DirectWrite，构建需 Windows 10 SDK ≥ 10.0.20348.0 |
| 各家结构化输出能力不一致（部分只有 `json_object`） | 功能降级 | 能力协商 `Capability` + 降级路径（`A2` §1） |
| 流式工具参数为分片 JSON | 解析失败率高 | 独立状态机 + 只在收尾时反序列化 |
| 只依赖单一 crate 生态（如 rig）会绑定其演进节奏 | 被动 | 自研契约，把 rig 当「参考实现」而非依赖 |

## 5. 未决项

首批适配器优先级、是否支持 MCP、本地模型是否一等公民 → 统一登记在 [`S2`](../10-now/open-questions.md)。
