//! 设置切片的核心测试（断网、**不碰真实钥匙串**，`Q1`）。
//!
//! 密钥一律走 [`MemoryBackend`]；配置文件写在临时目录里，每个用例各用一个文件。

use std::sync::Arc;

use agent_config::{ConfigPaths, SecretString, load};
use agent_settings::{Draft, KeySource, MemoryBackend, Settings, SettingsService};

/// 每个用例一个临时配置文件路径（先清掉，避免上一个用例残留）。
fn paths(name: &str) -> ConfigPaths {
    let path = std::env::temp_dir().join(format!(
        "agent-settings-test-{}-{name}.toml",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    ConfigPaths::from_file(path)
}

fn service(name: &str) -> SettingsService {
    SettingsService::with_parts(paths(name), Arc::new(MemoryBackend::new()))
}

#[test]
fn draft_normalizes_whitespace_and_empty() {
    let draft = Draft {
        default_provider: "  openai-compatible  ".into(),
        default_model: "   ".into(),
        base_url: " https://example.test/v1 ".into(),
        proxy: String::new(),
        stream_idle_timeout_secs: " 30 ".into(),
    };
    let settings = draft.to_settings().expect("合法草稿");

    assert_eq!(
        settings.default_provider.as_deref(),
        Some("openai-compatible")
    );
    // 空白 = 不设置这一项，而不是「设置成空串」
    assert_eq!(settings.default_model, None);
    assert_eq!(
        settings.base_url.as_deref(),
        Some("https://example.test/v1")
    );
    assert_eq!(settings.proxy, None);
    assert_eq!(settings.stream_idle_timeout_secs, Some(30));
    // 密钥**不在** `Draft` / `Settings` 里：它按 provider 单独进钥匙串（`R1`），
    // 「不落文件」由 `api_key_goes_to_backend_and_never_into_the_file` 守着。
}

#[test]
fn validate_reports_every_problem_at_once() {
    let draft = Draft {
        // 空白 = 不设置这一项（归一化在 `to_settings` 里先做），所以不算错误
        default_provider: "   ".into(),
        default_model: String::new(),
        base_url: "example.test".into(),
        proxy: "socks4://127.0.0.1:1080".into(),
        stream_idle_timeout_secs: "99999".into(),
    };
    let errors = draft.to_settings().expect_err("应当报错");
    let fields: Vec<&str> = errors.iter().map(|e| e.field).collect();
    // 一次返回全部问题（不是第一个），界面才能逐项标红
    assert!(fields.contains(&"base_url"));
    assert!(fields.contains(&"proxy"));
    assert!(fields.contains(&"stream_idle_timeout_secs"));
    assert_eq!(errors.len(), 3, "空白项不该被当成错误，实际：{errors:?}");
}

#[test]
fn save_keeps_sections_this_slice_does_not_own() {
    let paths = paths("keep-sections");
    // 用户手写的配置：本切片界面不管 [providers.*]，保存时必须原样留着
    std::fs::write(
        paths.file(),
        "[providers.vllm]\nbase_url = \"http://127.0.0.1:8000/v1\"\nmodel = \"qwen\"\n",
    )
    .expect("写临时配置");
    let service = SettingsService::with_parts(paths.clone(), Arc::new(MemoryBackend::new()));

    let settings = Settings {
        default_model: Some("gpt-4o".into()),
        ..Settings::default()
    };
    service.save(&settings).expect("保存");

    let written = load(&paths).expect("读回");
    assert_eq!(written.default_model.as_deref(), Some("gpt-4o"));
    assert_eq!(
        written
            .providers
            .get("vllm")
            .and_then(|p| p.model.as_deref()),
        Some("qwen"),
        "[providers.*] 属于用户手写内容，不该被设置界面清掉"
    );
}

#[test]
fn roundtrip_reads_back_what_was_saved() {
    let service = service("roundtrip");
    assert_eq!(service.load().expect("空配置应当可用"), Settings::default());

    let settings = Settings {
        default_provider: Some("anthropic".into()),
        default_model: Some("claude-sonnet-4-5".into()),
        base_url: Some("https://api.anthropic.com".into()),
        proxy: Some("socks5://127.0.0.1:1080".into()),
        stream_idle_timeout_secs: Some(120),
    };
    service.save(&settings).expect("保存");
    assert_eq!(service.load().expect("读回"), settings);
}

#[test]
fn broken_config_is_reported_and_never_overwritten() {
    let paths = paths("broken");
    std::fs::write(paths.file(), "这不是 TOML = = =\n").expect("写坏文件");
    let service = SettingsService::with_parts(paths.clone(), Arc::new(MemoryBackend::new()));

    // 读：必须报错（app 启动时降级，但设置界面不能装作没事）
    assert!(service.load().is_err(), "坏文件应当报错");
    // 写：也必须失败 —— 否则一按保存就把用户手写的内容覆盖了
    assert!(
        service.save(&Settings::default()).is_err(),
        "读不动就不许写，避免覆盖用户文件"
    );
}

#[test]
fn connect_writes_one_provider_entry_and_keeps_the_rest() {
    let paths = paths("connect");
    // 用户手写的内容 + 另一条已连接条目：连接新厂商时都不能被动到
    std::fs::write(
        paths.file(),
        "default_model = \"gpt-4o\"\nproxy = \"http://127.0.0.1:7890\"\n\n[providers.vllm]\nbase_url = \"http://127.0.0.1:8000/v1\"\n",
    )
    .expect("写临时配置");
    let service = SettingsService::with_parts(paths.clone(), Arc::new(MemoryBackend::new()));

    service
        .connect_provider(
            "openrouter",
            "openai-compatible",
            Some("https://openrouter.ai/api/v1"),
            Some("qwen/qwen3-235b"),
        )
        .expect("连接");

    let written = load(&paths).expect("读回");
    assert_eq!(written.default_model.as_deref(), Some("gpt-4o"));
    assert_eq!(
        written.proxy.as_deref(),
        Some("http://127.0.0.1:7890"),
        "顶层手写项必须原样保留"
    );
    assert_eq!(
        written
            .providers
            .get("vllm")
            .and_then(|p| p.base_url.as_deref()),
        Some("http://127.0.0.1:8000/v1"),
        "别的 [providers.*] 条目必须原样保留"
    );
    let entry = written.providers.get("openrouter").expect("新条目");
    assert_eq!(entry.adapter.as_deref(), Some("openai-compatible"));
    assert_eq!(
        entry.base_url.as_deref(),
        Some("https://openrouter.ai/api/v1")
    );
    assert_eq!(entry.model.as_deref(), Some("qwen/qwen3-235b"));
}

#[test]
fn connect_overwrites_only_the_targeted_entry() {
    let paths = paths("connect-overwrite");
    let service = SettingsService::with_parts(paths.clone(), Arc::new(MemoryBackend::new()));
    service
        .connect_provider("ollama", "openai-compatible", None, None)
        .expect("首次连接");
    service
        .connect_provider(
            "ollama",
            "openai-compatible",
            Some("http://127.0.0.1:11434/v1"),
            Some("llama3"),
        )
        .expect("覆盖连接");

    let written = load(&paths).expect("读回");
    assert_eq!(written.providers.len(), 1, "同一个 id 只应有一条");
    let entry = written.providers.get("ollama").expect("条目");
    assert_eq!(entry.base_url.as_deref(), Some("http://127.0.0.1:11434/v1"));
    assert_eq!(entry.model.as_deref(), Some("llama3"));
}

#[test]
fn connect_never_writes_the_key_into_the_file() {
    let paths = paths("connect-secret");
    let backend = Arc::new(MemoryBackend::new());
    let service = SettingsService::with_parts(paths.clone(), backend.clone());

    service
        .connect_provider("openrouter", "openai-compatible", None, None)
        .expect("连接");
    service
        .save_key("openrouter", &SecretString::from("sk-do-not-leak"))
        .expect("写密钥");

    let file = std::fs::read_to_string(paths.file()).expect("读配置文件");
    assert!(
        !file.contains("sk-do-not-leak"),
        "明文密钥不得落进配置文件（`R1` / 硬约束 5）"
    );
    assert_eq!(
        service.key_source("openrouter", None).expect("查来源"),
        KeySource::Keyring
    );
}

#[test]
fn connect_refuses_to_overwrite_a_broken_config() {
    let paths = paths("connect-broken");
    std::fs::write(paths.file(), "这不是 TOML = = =\n").expect("写坏文件");
    let service = SettingsService::with_parts(paths.clone(), Arc::new(MemoryBackend::new()));

    assert!(
        service
            .connect_provider("openrouter", "openai-compatible", None, None)
            .is_err(),
        "读不动就不许写，避免覆盖用户文件"
    );
    assert_eq!(
        std::fs::read_to_string(paths.file()).expect("读回原文件"),
        "这不是 TOML = = =\n",
        "失败时不得改动原文件"
    );
}

#[test]
fn api_key_goes_to_backend_and_never_into_the_file() {
    let paths = paths("secrets");
    let backend = Arc::new(MemoryBackend::new());
    let service = SettingsService::with_parts(paths.clone(), backend.clone());

    let secret = SecretString::from("sk-super-secret-value");
    assert_eq!(
        service
            .key_source("openai-compatible", None)
            .expect("查来源"),
        KeySource::None
    );
    service
        .save_key("openai-compatible", &secret)
        .expect("写密钥");
    assert_eq!(
        service
            .key_source("openai-compatible", None)
            .expect("查来源"),
        KeySource::Keyring
    );

    // 配置文件里绝不能出现明文（密钥只进钥匙串，`R1`）
    service.save(&Settings::default()).expect("保存设置");
    let file = std::fs::read_to_string(paths.file()).expect("读配置文件");
    assert!(
        !file.contains("sk-super-secret-value"),
        "明文密钥不得落进配置文件"
    );

    // 删除是幂等的（`R1`）：存过没存过都算成功
    service.delete_key("openai-compatible").expect("删密钥");
    service
        .delete_key("openai-compatible")
        .expect("再删一次也成功");
    assert_eq!(
        service
            .key_source("openai-compatible", None)
            .expect("查来源"),
        KeySource::None
    );
}
