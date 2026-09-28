# X1 · Rust 后端技术路线调研

> 索引 ID：`X1` ｜ 状态：✅ 调研完成 ｜ 最后更新：2026-09-27

## 目的

确定 Agent 后端（网络、流式、存储、密钥、配置、日志、打包）的具体 crate 选型与取舍。

## 1. 选型结论一览

| 关注点 | 选型 | 版本基线 | 理由 | 备选 |
|--------|------|---------|------|------|
| 异步运行时 | `tokio` | 1.x（full） | Rust 生态事实标准，`reqwest`/`sqlx` 等均围绕它 | `async-std`（生态弱） |
| HTTP 客户端 | `reqwest` | 0.12+ / rustls | 连接池、流式 body、超时、代理齐全 | `hyper` 直连（过度自研） |
| TLS | `rustls` | 默认 | 无 OpenSSL 依赖，交叉编译友好 | `native-tls` |
| SSE 解析 | `eventsource-stream` 或 `sse-rs` | 0.2 / 0.1+ | 把 `bytes_stream` 转成事件流；`sse-rs`（2026-05 发布）自带 `sse-core` 零 I/O 状态机，可 no_std | 自研状态机（200–400 行） |
| 序列化 | `serde` + `serde_json` | 1.x | 必需 | — |
| JSON Schema 生成 | `schemars` | 1.x | 由 Rust 类型生成工具入参 schema（rig 同款做法） | 手写 `serde_json::Value` |
| 时间/重试 | `tokio::time` + 自研退避 | — | 退避+抖动的语义各家不同，自研约 100 行 | `backoff` |
| 本地存储 | `rusqlite`（bundled） | 0.3x | 单机桌面无连接池需求；bundled 免装 SQLite | `sqlx`（异步 + 编译期 SQL，但引入异步连接池复杂度） |
| 密钥 | `keyring`（keyring-rs） | 3.x | 直连系统钥匙串：Windows Credential Manager / macOS Keychain / Linux Secret Service | 自加密文件（AES-GCM）+ 主密码 |
| 配置 | `toml` + `directories` | — | 分环境配置 + 平台标准目录 | `figment` / `config` |
| 日志/追踪 | `tracing` + `tracing-subscriber` | 0.1 / 0.3 | 结构化 span，能自然挂上「一次模型调用」的链路 | OpenTelemetry（后续按需） |
| 错误 | `thiserror`（库）+ `anyhow`（应用） | 2.x / 1.x | 契约层用强类型错误，业务层用 anyhow | — |
| 命令行/调试 | `clap` | 4.x | 顺手做 CLI 调试入口，便于脱离 UI 验证 adapter | — |
| 测试 | `cargo test` + `wiremock`/`httpmock` | — | mock HTTP 回放 fixtures，满足 G-3 | 自写 mock server |

## 2. SSE 流式处理的落地要点

- 三套事件语义需要分别解析（详见 `40-research/llm-abstraction.md` §3）：
  - OpenAI Chat / DeepSeek Chat：data-only SSE，`data: [DONE]` 结束。
  - OpenAI Responses / DeepSeek Responses：语义事件流，`response.completed` 结束。
  - Anthropic Messages：命名事件流（`message_start` / `content_block_delta` / `message_delta` / `message_stop`）。
- 工具参数在流里是**分片 JSON**，必须按 `index` / `id` 拼接，收尾时才反序列化。
- OpenAI 系要 `stream_options.include_usage = true` 才能在末块拿到 usage。
- 收尾语义要区分三类：传输错误、可恢复的坏帧、**EOF 无终止记录（截断）**；消费者应排空到流结束而不是见 `Err` 就停（rig 的做法可直接借鉴）。
- 必须有**空闲超时**兜底：部分厂商不发 `[DONE]`。

## 3. 工程结构（cargo workspace，草案）

```
agent/
  Cargo.toml            # workspace
  crates/
    agent-core/         # Agent 循环、工具编排；不依赖任何 provider
    agent-schema/       # 内部消息 / 事件 / 错误类型（契约的唯一来源）
    agent-providers/    # Provider trait 定义 + 各厂商 adapter
    agent-routing/      # 模型注册表、能力协商、路由与兜底
    agent-transport/    # reqwest 封装、SSE 解析、重试、超时
    agent-store/        # SQLite 会话/消息持久化
    agent-config/       # 配置 + keyring 密钥
    agent-observability/# tracing 初始化、token/成本统计
    agent-ui/           # GPUI 视图层（唯一接触 GPUI 的 crate）
    agent-app/          # 二进制入口，组装依赖
  tests/                # 契约测试 + fixtures
```

约束：依赖只能向下；`agent-core` 与 `agent-schema` 不得依赖 `gpui`、`reqwest` 之外的具体实现。

## 4. 密钥与配置

- 优先级：**环境变量 > 系统钥匙串 > 配置文件**（便于 CI 与临时覆盖）。
- 配置文件只存非敏感项（`provider`、`base_url`、`model`、超时、代理）；`api_key` 一律写钥匙串。
- 非交互场景（无钥匙串服务，如部分 Linux）降级为 AES-GCM 加密文件，并在 UI 明示。
- 日志中禁止打印 `Authorization`；`tracing` 上用 `SecretString` 包裹避免 `Debug` 泄漏。

## 5. 打包分发

| 平台 | 方案 | 备注 |
|------|------|------|
| Windows | `cargo-wix` 生成 MSI | 需要 WiX Toolset；无签名会有 SmartScreen 警告，后续补 `signtool` |
| macOS | `.app` + 签名/公证 | 需要开发者账号 |
| Linux | `cargo-deb` / AppImage | 后续按需 |

其他：GPUI 应用不需要 WebView/Electron 运行时，产物为单个可执行文件，体积优势明显。

## 6. 待办

- [ ] 用 `wiremock` 建 first adapter 的 fixtures 骨架
- [ ] 验证 `sse-rs` vs `eventsource-stream` 在坏帧/截断/超时下的行为差异，据此决定是否自研
- [ ] 验证 `keyring` 在 Windows 与本机环境的读写行为

## 相关

- 上游：`20-spec/goals-and-routes.md`
- 下游：`docs/30-decisions/0002-llm-abstraction.md`
