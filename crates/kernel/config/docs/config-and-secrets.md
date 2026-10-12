# R1 · 配置与密钥

| | |
|---|---|
| ID | `R1` |
| 类型 | 规格（归属 `agent-config`） |
| 状态 | ✅ 已落地（2026-10-11） |
| 更新 | 2026-10-11 |
| 何时读 | 加配置项、接密钥、讨论「密钥从哪来」时 |
| 规模 | ~1.5k token |

## TL;DR

- **两条独立存储**：**配置文件**（TOML）只放非敏感项；**密钥**一律进系统钥匙串。
- **密钥优先级**：`--api-key`（CLI） > 环境变量 > 系统钥匙串。都没有 → 打印一行提示，程序**照常运行**（无鉴权端点可忽略）。
- **端点优先级**：`--base-url` > 配置文件（供应商级 → 顶层） > 内置默认。
- **配置路径**：`--config` 显式指定；缺省走平台标准目录（Windows `%APPDATA%\CodingRocket\config.toml`）。**文件不存在 = 默认配置**，不是错误。
- **失败一律降级**：配置读不了 / 解不开 → 提示一行后继续；钥匙串后端不可用 → 跳过该来源，不阻断启动。
- 密钥用 `SecretString` 承载（`Debug` 打码）；**配置错误信息不回显原文**（防止误把密钥写进配置时泄漏）。

## 1. 优先级

| 来源 | 密钥 | 端点 |
|------|------|------|
| CLI 参数 | `--api-key` | `--base-url` |
| 环境变量 | `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`（按 provider 推断，`A2` §6） | — |
| 配置文件 | ❌ **禁止**（未知字段直接报错） | `providers.<id>.base_url` → 顶层 `base_url` |
| 系统钥匙串 | `CodingRocket` 服务，账号 `<provider>:api_key` | — |
| 内置默认 | — | OpenAI / Anthropic 官方端点 |

## 2. 配置文件

只允许这些字段（`deny_unknown_fields`，拼错即报错）：

```toml
default_provider = "openai-compatible"
default_model = "gpt-4o"
base_url = "https://fallback.example/v1"     # 顶层兜底端点
proxy = "http://127.0.0.1:7890"
stream_idle_timeout_secs = 60

[providers.anthropic]
base_url = "https://api.anthropic.com"
model = "claude-sonnet-4-5-20250929"
```

- 路径：`--config <path>`，或平台标准目录（`directories::ProjectDirs("dev", "", "CodingRocket")`）。
- 读写 API：`ConfigPaths` / `load` / `save`；**不存在返回默认**，无配置即可运行。
- 两级查找：`AppConfig::base_url_for` / `model_for`（供应商级覆盖优先，回退顶层）。

## 3. 密钥与钥匙串

- 载体：`SecretString`（`secrecy`），`Debug` 打码；本 crate 统一 re-export，使用方不必直接依赖 `secrecy`。
- 读：`SecretStore::get` —— **「没存过」返回 `Ok(None)`**，只有真故障才 `Err`。
- 写 / 删：`set` / `delete`（删除幂等）。
- 解析：`resolve_api_key(provider)` 实现「环境变量 > 钥匙串」。

### ⚠️ `keyring` 必须按平台显式开 backend

`keyring` 3.x 的 **default features 是空的**（`docs.rs` 元数据里列的那些 native feature 并不是 default）。不显式开启会**静默退化为 mock 后端**：

- `set` 返回 `Ok`，但 `get` 立刻读不到（mock 的持久化是 `EntryOnly`，只活在**单个 `Entry` 实例**里）；
- 系统凭据管理器里**查无此项**（`cmdkey /list` 看不到）。

因此 `Cargo.toml` 用 `[target.'cfg(...)'.dependencies]` 分平台启用：

| 平台 | feature |
|------|---------|
| Windows | `windows-native` |
| macOS | `apple-native` |
| Linux | `sync-secret-service` + `crypto-rust` |

真实读写验证（会写系统凭据管理器，默认 ignored）：

```powershell
cargo test -p agent-config -- --ignored
```

## 4. 安全约定

1. **配置文件绝不出现密钥**（硬约束 5）：未知字段被拒，且错误信息**不回显原文**（`toml` 的错误只取 `message()`，它的 `Display` 会回显出错那一行）。
2. **明文只在装配边界短暂存在**：`app` 用 `expose_secret()` 取出后交给适配器构造鉴权头，不进日志、不入库、不进 `Debug`。
3. **错误文案不含密钥**：`ConfigError` 的所有 `Display` 都不得出现明文。

## 降级路径（已识别，尚未实现）

`X1` §4 提到「无钥匙串服务的环境（部分 Linux）降级为 AES-GCM 加密文件，并在 UI 明示」。当前实现是**跳过钥匙串来源**（回退到环境变量），未做加密文件。真要做时补在本节并登记 `S2`。

## 落地清单

- [x] 新建 `kernel/config`：`file`（TOML + 平台目录）、`secret`（钥匙串）、`error`。
- [x] 分平台启用 `keyring` backend，并完成 Windows 真实读写验证。
- [x] `app` 接入：`--config` 参数、密钥三级优先级、端点两级查找、代理与空闲超时可配。
- [x] 用例：配置往返 / 缺文件为默认 / 未知字段被拒且不回显 / 密钥 Debug 打码 / 环境变量名。
- [ ] 加密文件降级（无钥匙串环境）—— 见上节。
- [x] `features/settings` 的设置界面（写配置 + 存钥匙串）—— ✅ 2026-10-12（`A` 语义：保存写盘、**重启生效**）。
      本 crate 为此**补了一个出口**：`account_for`（钥匙串账号命名 `<provider>:api_key`）原先
      `pub` 但未 re-export，外部要写密钥只能自己拼一份账号名——那会让这个约定出现两个家。

## 相关

- 选型（`keyring` / `toml` + `directories` / `SecretString`） → `X1` §1、§4
- 密钥约定（不得写死、走配置层） → `A2` §6 ｜ 分层判据 → `D6` §1
- 使用方：`crates/app`（装配）｜ 门面 → `crates/kernel/config` 的 `README.md`
- 未决项 → `S2`
