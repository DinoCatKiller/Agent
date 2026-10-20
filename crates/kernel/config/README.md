# agent-config · 配置与密钥

| | |
|---|---|
| 状态 | ✅ `R1` 落地：TOML 配置文件 + 系统钥匙串（2026-10-11） |
| 边界 | **机制**：配置从哪读、密钥往哪存。**不含**「用哪个 provider / 怎么呈现设置」——那是 `features/settings` 与宿主 |
| 上游 | 无（刻意不依赖 `agent-common`：配置错误与「一次调用」的错误是两套语义） |
| 下游 | `app`（装配时取密钥与端点）；`features/settings`（将来的设置界面） |
| 何时读 | 加配置项、动密钥存储、讨论「密钥从哪来」时 |

## 这个 crate 是什么

两条独立的存储：

- **配置文件**（TOML，平台标准目录）：**只放非敏感项** —— provider / base_url / model / 代理 / 超时。
- **密钥**（系统钥匙串）：`api_key` 一律在这里，绝不落文件、绝不进日志。

密钥解析优先级（`X1` §4）：**环境变量 > 系统钥匙串**；CLI 参数由 `app` 更优先处理。

## 核心内容

| 项 | 说明 |
|----|------|
| `ConfigPaths::discover()` | 平台标准目录定位（Windows `%APPDATA%\CodingRocket\config\config.toml`） |
| `ConfigPaths::from_file(path)` | 显式指定（测试 / `--config`） |
| `load` / `save` | TOML 读写；**文件不存在返回默认配置**（零配置即可运行） |
| `AppConfig::base_url_for` / `model_for` | 「供应商级覆盖 → 顶层回退」两级查找 |
| `SecretStore` | 钥匙串读写（Windows Credential Manager / macOS Keychain / Linux Secret Service）；**未存过返回 `None`，不是错误** |
| `resolve_api_key(provider)` | 环境变量 > 钥匙串；钥匙串后端不可用时**不阻断**，交上层提示「未配置密钥」 |
| `SecretString` | 密钥载体，`Debug` **打码**（`X1` §4） |

## 四个必须知道的约束

1. **配置文件禁放密钥**：未知字段被 `deny_unknown_fields` 拒绝，且错误信息不回显内容。
2. **「没找到」不是错误**：`load` 的「文件不存在」与 `SecretStore::get` 的「未存过」都返回正常值，只有真故障才是 `Err`。
3. **错误文案不含密钥**：所有 `ConfigError` 的 `Display` 都不得出现明文；`toml` 的解析错误也只取 `message()`（它的 `Display` 会回显出错那一行的原文）。
4. **`keyring` 必须按平台显式开 backend**：`keyring` 3.x 的 **default features 是空的**（`docs.rs` 元数据里列的那些 native feature 并不是 default）。不显式开启会**静默退化为 mock 后端** —— 写入只活在单个 `Entry` 实例里，表现为「`set` 返回 `Ok`、`get` 立刻读不到」，且系统凭据管理器里查无此项。本 crate 在 `Cargo.toml` 用 `[target.'cfg(...)'.dependencies]` 分平台启用（Windows `windows-native` / macOS `apple-native` / Linux `sync-secret-service` + `crypto-rust`）。
   真实读写验证：`cargo test -p agent-config -- --ignored`（会写系统凭据管理器，默认不跑）。

## 相关

- 规格 → `R1` ｜ 选型（`keyring` / `toml` + `directories`） → `X1` §1、§4 ｜ 分层判据 → `D6` §1
- 使用方：`crates/app`（装配）｜ 概览 → `S1` ｜ 地图 → [`AGENTS.md`](../../../AGENTS.md) §2
