# AGENTS.md — 文档索引入口

> 定位：本项目的**唯一常驻入口**。
> 原则：**索引常驻，细节按需**。这里不放实现细节，只回答两件事——「有什么文档」和「什么时候读哪一篇」。

## 0. 读取协议（所有会话强制遵守）

1. 会话开始**只读本文件**，不要递归扫 `docs/`。
2. 按任务命中索引条目，**只加载相关文档（通常 ≤ 3 篇）**。
3. 状态含义：`✅ 已就绪` 可直接读；`⬜ 待创建` 表示文件尚不存在 —— 先复制 `docs/_template.md` 建骨架，写完后**回填本索引状态为 ✅**。
4. 任何对 `docs/` 的增删改，**必须同步更新本索引**（路径 / 摘要 / 状态）。
5. 单篇超过约 300 行 → 拆分并登记新条目，禁止无限膨胀。
6. 架构、选型、破坏性变更 → 写 ADR（`docs/adr/`），不留口头结论。
7. 不确定读哪篇时，先读 `docs/00-goals-and-routes.md`。

## 1. 索引

### 1.1 目标与路线

| ID | 主题 | 路径 | 什么时候读 | 状态 |
|----|------|------|-----------|------|
| G1 | 目标、技术路线总览、自研边界 | `docs/00-goals-and-routes.md` | 开工前、争论要不要自研某个轮子、确定优先级 | ✅ |

### 1.2 架构与契约

| ID | 主题 | 路径 | 什么时候读 | 状态 |
|----|------|------|-----------|------|
| A1 | 架构总览与分层 | `docs/10-architecture.md` | 新建模块、改目录结构、讨论整体设计 | ✅ |
| A2 | Provider 接入契约 | `docs/11-provider-contract.md` | 新增/修改任何模型供应商、改调用接口 | ✅ |
| A3 | 消息与多模态协议 | `docs/12-message-protocol.md` | 涉及文本/图片/文件/音频输入、role 语义 | ⬜ |
| A4 | 流式、取消与超时 | `docs/13-streaming.md` | SSE/WebSocket、中断生成、超时策略 | ⬜ |
| A5 | 工具调用（Function Calling / MCP） | `docs/14-tool-calling.md` | 工具定义、并行调用、结果回填 | ⬜ |
| A6 | 错误分类、重试与降级 | `docs/15-errors-and-fallback.md` | 限流/超时/审核/模型不可用 | ⬜ |
| A7 | 上下文、Token 与记忆 | `docs/16-context-and-tokens.md` | 上下文裁剪、摘要、计费统计 | ⬜ |

### 1.3 模型供应商

| ID | 主题 | 路径 | 什么时候读 | 状态 |
|----|------|------|-----------|------|
| P1 | OpenAI / Azure OpenAI | `docs/providers/openai.md` | 接 GPT 系列 | ⬜ |
| P2 | Anthropic Claude | `docs/providers/anthropic.md` | 接 Claude 系列 | ⬜ |
| P3 | Google Gemini | `docs/providers/gemini.md` | 接 Gemini 系列 | ⬜ |
| P4 | 国产厂商 | `docs/providers/cn-vendors.md` | DeepSeek / 通义 / Kimi / GLM / 豆包 | ⬜ |
| P5 | 本地与自建 | `docs/providers/local.md` | Ollama / vLLM / LM Studio | ⬜ |
| P6 | OpenAI 兼容聚合层 | `docs/providers/openai-compatible.md` | OpenRouter / OneAPI / 硅基流动等 | ⬜ |

### 1.4 运行时

| ID | 主题 | 路径 | 什么时候读 | 状态 |
|----|------|------|-----------|------|
| R1 | 配置与密钥管理 | `docs/20-config-and-secrets.md` | 加配置项、接密钥、多环境 | ⬜ |
| R2 | 模型注册表与路由 | `docs/21-model-registry.md` | 模型清单、别名、按能力/成本路由 | ⬜ |
| R3 | 可观测性与成本 | `docs/22-observability.md` | 日志 / trace / 指标 / 计费 | ⬜ |
| R4 | 缓存、限流与并发 | `docs/23-cache-and-ratelimit.md` | 重复请求缓存、QPS 控制 | ⬜ |

### 1.5 质量

| ID | 主题 | 路径 | 什么时候读 | 状态 |
|----|------|------|-----------|------|
| Q1 | 测试策略 | `docs/30-testing.md` | 写单测、契约测试、mock 供应商 | ⬜ |
| Q2 | 效果评测与回归 | `docs/31-eval.md` | 建评测集、prompt 回归 | ⬜ |
| Q3 | 安全与合规 | `docs/32-security.md` | 密钥、审计、数据合规、敏感内容 | ⬜ |

### 1.6 技术调研

| ID | 主题 | 路径 | 什么时候读 | 状态 |
|----|------|------|-----------|------|
| X1 | Rust 后端技术路线 | `docs/research/rust-backend-stack.md` | 定 crate 选型、workspace 结构、SSE/存储/密钥/打包 | ✅ |
| X2 | GPUI 前端技术路线 | `docs/research/gpui-frontend-stack.md` | 动 UI、升 GPUI 版本、遇到平台/IME/渲染问题 | ✅ |
| X3 | 多模型统一抽象路线 | `docs/research/llm-abstraction.md` | 讨论要不要用 rig/genai、协议归一化、工具/结构化输出差异 | ✅ |
| X4 | UI 交付形态（GPUI / TUI / WebView） | `docs/research/ui-delivery-options.md` | 讨论界面怎么做、TUI 用什么、GPUI 能不能做终端 | ✅ |

### 1.7 决策记录与模板

| ID | 主题 | 路径 | 什么时候读 | 状态 |
|----|------|------|-----------|------|
| D0 | ADR 模板 | `docs/adr/_template.md` | 要写新 ADR 时 | ✅ |
| D1 | ADR-001 技术栈选型（Rust + GPUI） | `docs/adr/0001-tech-stack.md` | 涉及语言/框架/依赖选型 | ✅ 已接受 |
| D2 | ADR-002 多模型抽象路线（自研契约） | `docs/adr/0002-llm-abstraction.md` | 是否引入 rig/genai、契约归属 | ✅ 已接受 |
| D3 | ADR-003 UI 交付形态（提案） | `docs/adr/0003-ui-delivery-form.md` | 决定 GPUI / TUI / Tauri，或是否分阶段 | 🚧 待决 |
| T0 | 文档骨架模板 | `docs/_template.md` | 新建任何 `docs/` 文档时 | ✅ |

## 2. 任务 → 加载清单

| 任务 | 依次加载 |
|------|---------|
| 新增一家模型供应商 | A2 → 对应 P* → X3 → R2 → Q1 |
| 写第一个 Provider adapter | A2 → X1 → X3 → Q1 |
| 决定界面形态 / 换 UI 技术 | X4 → D3 → X1/X2 |
| 动 UI / GPUI 相关 | X2 → X4 §5 → A4 |
| 写 TUI（Ratatui） | X4 §2/§4 → A4 → Q1 |
| 修一个调用 Bug | A6 → 对应 P* → Q1 |
| 加一种输入模态（图/文件） | A3 → A2 → 相关 P* |
| 做模型路由 / 兜底 | R2 → A6 → R3 |
| 控制调用成本 | R3 → A7 → R2 |
| 决定「自研还是找现成轮子」 | G1 §3 → X1/X2/X3 |
| 选型 / 结构大改 | D0 → A1 → 写新 ADR |

## 3. 始终生效的工程约束

1. **技术栈已定**：Rust（edition 2024）+ tokio 后端，GPUI（经 `gpui-kit`）前端。不要引入 Electron/WebView/Tauri 方案。
2. 供应商差异**不得泄漏到 Agent Core**：核心只依赖 A2 的契约。
3. 新增能力顺序：先写/改契约 → 再改适配层 → 最后接业务。
4. 密钥、真实 endpoint、账号信息**不得入库**；示例一律用占位符。
5. 对外接口**流式优先**，非流式是退化路径。
6. UI 线程禁止阻塞在网络 IO 上；GPUI API 只能出现在 `agent-ui` crate 内。
7. 文档与代码同批次提交；改了行为没改文档视为未完成。
8. 未决事项写进 ADR 或文档「待定问题」小节。

## 4. 待定问题（开工前确认）

- 首批适配器优先级：OpenAI 兼容族先行，还是 Anthropic 优先？→ 影响 `docs/providers/*`
- 是否本期支持 MCP 客户端（`rig` 0.42 已移除 `rmcp`，需自研或引第三方）
- 是否要求本地模型（Ollama）作为一等公民
- 目标产物体积 / 是否需要代码签名
