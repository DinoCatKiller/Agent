# R1 · 配置与密钥

| | |
|---|---|
| ID | `R1` |
| 类型 | 规格（归属 `agent-config`） |
| 状态 | ✅ 已落地（2026-10-11） |
| 更新 | 2026-10-13 |
| 何时读 | 加配置项、接密钥、讨论「密钥从哪来」时 |
| 规模 | ~1.8k token |

## TL;DR

- **两条独立存储**：**配置文件**（TOML）只放非敏感项；**密钥**一律进系统钥匙串。
  UI 偏好（模型显隐）另存 `models.toml`，见 §2.2。
- **密钥优先级**：`--api-key`（CLI） > 环境变量 > 系统钥匙串。都没有 → 打印一行提示，程序**照常运行**（无鉴权端点可忽略）。
  **环境变量名由内置目录按条目给出**（`providers/src/catalog.rs`）：`ollama` 这类没有约定名的条目
  **只看钥匙串**，绝不回退到适配器的名字（否则会把你的 OpenAI 密钥发给 ollama.com，2026-10-13 修）。
- **端点优先级**：`--base-url` > `providers.<条目>.base_url` > 顶层 `base_url` > 目录的 `base_url` > 适配器官方端点。
- **「条目」是厂商或自定义供应商**（`ollama` / `openrouter` / 你自己起的名字），
  它用哪个**适配器**（`openai-compatible` / `anthropic`）由目录或 `adapter` 字段决定 ——
  用户不需要知道适配器这回事。
- **配置路径**：`--config` 显式指定；缺省走平台标准目录（Windows `%APPDATA%\CodingRocket\config\config.toml`）。
  注意多一层 `config` 子目录（`directories` 的 `config_dir()` 行为）+ 文件名本身叫 `config.toml`）。
  **文件不存在 = 默认配置**，不是错误。
- **失败一律降级**：配置读不了 / 解不开 → 提示一行后继续；钥匙串后端不可用 → 跳过该来源，不阻断启动。
- 密钥用 `SecretString` 承载（`Debug` 打码）；**配置错误信息不回显原文**（防止误把密钥写进配置时泄漏）。

## 1. 优先级

| 来源 | 密钥 | 端点 |
|------|------|------|
| CLI 参数 | `--api-key` | `--base-url` |
| 环境变量 | **由内置目录按条目给出**（`OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `OPENROUTER_API_KEY` …）；目录里没写约定名的条目（本地服务）**不回退** | — |
| 配置文件 | ❌ **禁止**（未知字段直接报错） | `providers.<条目>.base_url` → 顶层 `base_url` |
| 系统钥匙串 | `CodingRocket` 服务，账号 `<条目 id>:api_key`（**按条目**，不按适配器） | — |
| 内置默认 | — | 目录里该条目的 `base_url` → 适配器官方端点 |

**CLI 的 `--api-key` / `--base-url` 只属于启动时那个条目**：界面切到别家后不再沿用
（`worker::ProviderFactory` 与 `setup::declared_for` 是同一条规则）——否则等于把 A 家的密钥与端点发给 B 家。

### 1.1 provider / model 的解析顺序（2026-10-12 补，2026-10-13 扩）

| 项 | 顺序 |
|----|------|
| provider | `--provider` > 配置文件 `default_provider` > **报错退出**。可用值 = **目录 id**（`ollama` / `openrouter` …）∪ **用户条目 id**（需写 `adapter`）∪ 两个适配器 id |
| model | `--model` 首个 > `providers.<条目>.model` > `default_model` > **报错退出** |

三点约定：

1. **缺了就报错，不给猜**：模型选错是真花钱的事（`A2` §2 的精神）。报错文案要指路
   （「在配置文件里设 `default_provider` / `default_model`」）。
2. **通用 vs 具体的查找顺序与端点一致**：`providers.<id>.model` 优先于顶层 `default_model`，
   和 `base_url_for` 的「供应商级 → 顶层」是同一条规则，不是两套。
3. **降级链只由 CLI 表达**（`--model a,b`，`R2`）：配置里的默认值不带降级链。

> **背景（为什么现在才写这段）**：`default_provider` / `default_model` 在 2026-10-12 之前是
> **只写不读**的 —— 各子命令把 `--provider` / `--model` 设成必填，默认值永远轮不到，
> 于是设置界面让用户改的两个值毫无作用。改成「CLI 可省 + 缺失报错」后它们才真正生效；
> `model_for()` 也才第一次被 `app` 调用。

## 2. 配置文件

只允许这些字段（`deny_unknown_fields`，拼错即报错）：

```toml
default_provider = "ollama"
default_model = "gemma4:31b"
base_url = "https://fallback.example/v1"     # 顶层兜底端点（对任何条目生效）
proxy = "http://127.0.0.1:7890"
stream_idle_timeout_secs = 60

[providers.ollama]                     # 目录里的条目：只需覆盖想改的字段
models = []                            # 该端点已知的模型 id 清单（C2：联网拉取 GET /models 写回，也可手写）
base_url = "https://ollama.com/v1"

[providers.my-vllm]                    # 目录里没有的条目：**必须写 adapter**
adapter = "openai-compatible"
base_url = "https://my-server.example/v1"
model = "qwen3-32b"
```

条目字段：

| 字段 | 说明 |
|------|------|
| `adapter` | `openai-compatible` / `anthropic`。**目录里没有的 id 必填**；目录里有的可省（由目录推断） |
| `base_url` | 覆盖目录默认端点 |
| `model` | 该条目的默认模型（优先于顶层 `default_model`） |
| `models` | 该端点已知的模型 id 清单（`C2`）：联网刷新 `GET /models` 的结果写这里，也允许手写。选择器候选与本地能力协商都读它 —— 拉到的模型因此复用**已有的逐模型显隐偏好**（`models.toml`），不引入额外状态。**声明过的 id 才算「已知」**；未知 id 的能力是按保守占位（`capabilities_known = false`）处理的，不据此拦截 |

- 路径：`--config <path>`，或平台标准目录（`directories::ProjectDirs("dev", "", "CodingRocket")`）。
- 读写 API：`ConfigPaths` / `load` / `save` / **`update`**；**不存在返回默认**，无配置即可运行。
  - 任何「读全量 → 改自己管的字段 → 写回」都必须走 `update`（持**进程内写锁**）：worker 写回
    拉取的模型清单与设置界面保存 / 连接条目若并发，后写者会基于过期快照把先写者的改动整段抹掉（`C2`）。
- 条目解析：`agent_providers::catalog::resolve(id, Declared{..})` —— 适配器 / 端点 / 密钥变量名
  一次算清，`build_provider`、模型清单、设置页三处共用同一套（不会出现"列得出来但连不上"）。
- 老的 `AppConfig::base_url_for` / `model_for` 仍在（`app` 已改用 `resolve`，保留给只读场景）。

### 2.2 UI 偏好（`models.toml`）

「模型选择器里不显示哪些模型」存在**独立文件**里（与 `config.toml` 同目录），
不并进 `config.toml`：后者带 `deny_unknown_fields`，加键会让旧版本二进制读到新配置直接解析失败。
偏好的两条取舍与配置**刻意相反**：容忍未知键（新版写的偏好旧版照读）、坏了不报错（降级默认值）。
读写 API：`Preferences::load` / `save`、`ConfigPaths::preferences_file()`。

## 3. 密钥与钥匙串

- 载体：`SecretString`（`secrecy`），`Debug` 打码；本 crate 统一 re-export，使用方不必直接依赖 `secrecy`。
- 读：`SecretStore::get` —— **「没存过」返回 `Ok(None)`**，只有真故障才 `Err`。
- 写 / 删：`set` / `delete`（删除幂等）。
- 解析：`resolve_api_key(provider, env)` 实现「环境变量 > 钥匙串」——
  `env` 是**调用方给的变量名**（`config` 是机制层，不认识供应商目录，`D6`；
  映射在 `agent_providers::catalog`）。`None` = 该条目没有约定名，只看钥匙串。

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
- [x] **UI 偏好独立文件** `models.toml`（2026-10-13，切片 `B`）：`prefs` 模块 + `Preferences`。
      与 `config.toml` 相反地**容忍未知键**（偏好不该有单向兼容问题）。见 §2.2。
- [x] **供应商条目化**（2026-10-13，切片 `C1a` / `C1b`）：`ProviderConfig.adapter`、
      `resolve_api_key(provider, env)`（环境变量名不再是本 crate 的写死规则）、
      `providers.<id>` 的 id 可以是厂商名。厂商知识落在 `agent_providers::catalog`（`R5`）。
      ⚠️ `adapter` 是本文件的新键，**旧版本二进制读到会解析失败** —— 与 §2 的老取舍一致。

## 相关

- 选型（`keyring` / `toml` + `directories` / `SecretString`） → `X1` §1、§4
- 密钥约定（不得写死、走配置层） → `A2` §6 ｜ 分层判据 → `D6` §1
- 厂商 → 适配器 / 端点 / 密钥变量名 → `R5`（`crates/kernel/providers/docs/catalog.md`）
- 使用方：`crates/app`（装配）｜ 门面 → `crates/kernel/config` 的 `README.md`
- 未决项 → `S2`
