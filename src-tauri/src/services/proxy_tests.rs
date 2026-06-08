use super::*;
use crate::provider::ProviderMeta;
use serial_test::serial;
use std::env;
use tempfile::TempDir;

struct TempHome {
    #[allow(dead_code)]
    dir: TempDir,
    original_home: Option<String>,
    original_userprofile: Option<String>,
    original_test_home: Option<String>,
}

impl TempHome {
    fn new() -> Self {
        let dir = TempDir::new().expect("failed to create temp home");
        let original_home = env::var("HOME").ok();
        let original_userprofile = env::var("USERPROFILE").ok();
        let original_test_home = env::var("CC_SWITCH_TEST_HOME").ok();

        env::set_var("HOME", dir.path());
        env::set_var("USERPROFILE", dir.path());
        env::set_var("CC_SWITCH_TEST_HOME", dir.path());

        Self {
            dir,
            original_home,
            original_userprofile,
            original_test_home,
        }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        match &self.original_home {
            Some(value) => env::set_var("HOME", value),
            None => env::remove_var("HOME"),
        }

        match &self.original_userprofile {
            Some(value) => env::set_var("USERPROFILE", value),
            None => env::remove_var("USERPROFILE"),
        }

        match &self.original_test_home {
            Some(value) => env::set_var("CC_SWITCH_TEST_HOME", value),
            None => env::remove_var("CC_SWITCH_TEST_HOME"),
        }
    }
}

fn assert_env_str(env: &Map<String, Value>, key: &str, expected: Option<&str>) {
    assert_eq!(env.get(key).and_then(|value| value.as_str()), expected);
}

#[test]
fn managed_account_claude_takeover_uses_api_key_placeholder() {
    let mut provider = Provider::with_id(
        "copilot".to_string(),
        "GitHub Copilot".to_string(),
        json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.githubcopilot.com",
                "ANTHROPIC_MODEL": "claude-haiku-4.5"
            }
        }),
        None,
    );
    provider.meta = Some(ProviderMeta {
        provider_type: Some("github_copilot".to_string()),
        ..Default::default()
    });

    let mut live_config = provider.settings_config.clone();
    ProxyService::apply_claude_takeover_fields_for_provider(
        &mut live_config,
        "http://127.0.0.1:15721",
        &provider,
    );

    let env = live_config
        .get("env")
        .and_then(|value| value.as_object())
        .expect("env should exist");
    assert_eq!(
        env.get("ANTHROPIC_API_KEY")
            .and_then(|value| value.as_str()),
        Some(PROXY_TOKEN_PLACEHOLDER)
    );
    assert!(
        env.get("ANTHROPIC_AUTH_TOKEN").is_none(),
        "managed OAuth providers should avoid Claude Auth Token login semantics"
    );
}

#[test]
fn managed_account_claude_takeover_sources_copilot_models_from_provider() {
    let mut provider = Provider::with_id(
        "copilot".to_string(),
        "GitHub Copilot".to_string(),
        json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.githubcopilot.com",
                "ANTHROPIC_MODEL": "claude-sonnet-4.6",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4.5",
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4.6",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-sonnet-4.6"
            }
        }),
        None,
    );
    provider.meta = Some(ProviderMeta {
        provider_type: Some("github_copilot".to_string()),
        ..Default::default()
    });

    let mut live_config = json!({
        "env": {
            "ANTHROPIC_BASE_URL": "https://stale.example.com",
            "ANTHROPIC_API_KEY": "stale-key",
            "ANTHROPIC_MODEL": "stale-model",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL": "stale-haiku",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME": "Stale Haiku",
            "ANTHROPIC_DEFAULT_SONNET_MODEL": "stale-sonnet",
            "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME": "Stale Sonnet",
            "ANTHROPIC_DEFAULT_OPUS_MODEL": "stale-opus",
            "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME": "Stale Opus"
        }
    });
    ProxyService::apply_claude_takeover_fields_for_provider(
        &mut live_config,
        "http://127.0.0.1:15721",
        &provider,
    );

    let env = live_config
        .get("env")
        .and_then(|value| value.as_object())
        .expect("env should exist");
    assert_env_str(env, "ANTHROPIC_MODEL", None);
    assert_env_str(
        env,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        Some("claude-haiku-4-5"),
    );
    assert_env_str(
        env,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME",
        Some("claude-haiku-4.5"),
    );
    assert_env_str(
        env,
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        Some("claude-sonnet-4-6"),
    );
    assert_env_str(
        env,
        "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
        Some("claude-sonnet-4.6"),
    );
    assert_env_str(env, "ANTHROPIC_DEFAULT_OPUS_MODEL", Some("claude-opus-4-7"));
    assert_env_str(
        env,
        "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME",
        Some("claude-sonnet-4.6"),
    );
    assert_env_str(env, "ANTHROPIC_API_KEY", Some(PROXY_TOKEN_PLACEHOLDER));
    assert_env_str(env, "ANTHROPIC_AUTH_TOKEN", None);
}

#[test]
fn managed_account_claude_takeover_sources_codex_models_from_provider() {
    let mut provider = Provider::with_id(
        "codex".to_string(),
        "Codex".to_string(),
        json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://chatgpt.com/backend-api/codex",
                "ANTHROPIC_MODEL": "gpt-5.4",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "gpt-5.4-mini",
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "gpt-5.4",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "gpt-5.4"
            }
        }),
        None,
    );
    provider.meta = Some(ProviderMeta {
        provider_type: Some("codex_oauth".to_string()),
        ..Default::default()
    });

    let mut live_config = json!({
        "env": {
            "ANTHROPIC_BASE_URL": "https://stale.example.com",
            "ANTHROPIC_AUTH_TOKEN": "stale-token",
            "ANTHROPIC_MODEL": "stale-model",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL": "stale-haiku",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME": "Stale Haiku",
            "ANTHROPIC_DEFAULT_SONNET_MODEL": "stale-sonnet",
            "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME": "Stale Sonnet",
            "ANTHROPIC_DEFAULT_OPUS_MODEL": "stale-opus",
            "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME": "Stale Opus"
        }
    });
    ProxyService::apply_claude_takeover_fields_for_provider(
        &mut live_config,
        "http://127.0.0.1:15721",
        &provider,
    );

    let env = live_config
        .get("env")
        .and_then(|value| value.as_object())
        .expect("env should exist");
    assert_env_str(env, "ANTHROPIC_MODEL", None);
    assert_env_str(
        env,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        Some("claude-haiku-4-5"),
    );
    assert_env_str(
        env,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME",
        Some("gpt-5.4-mini"),
    );
    assert_env_str(
        env,
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        Some("claude-sonnet-4-6"),
    );
    assert_env_str(env, "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME", Some("gpt-5.4"));
    assert_env_str(env, "ANTHROPIC_DEFAULT_OPUS_MODEL", Some("claude-opus-4-7"));
    assert_env_str(env, "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME", Some("gpt-5.4"));
    assert_env_str(env, "ANTHROPIC_API_KEY", Some(PROXY_TOKEN_PLACEHOLDER));
    assert_env_str(env, "ANTHROPIC_AUTH_TOKEN", None);
}

#[test]
fn normal_claude_takeover_without_token_keeps_auth_token_fallback() {
    let mut live_config = json!({
        "env": {
            "ANTHROPIC_BASE_URL": "https://api.example.com",
            "ANTHROPIC_MODEL": "claude-haiku-4.5"
        }
    });

    ProxyService::apply_claude_takeover_fields(&mut live_config, "http://127.0.0.1:15721");

    assert_eq!(
        live_config
            .get("env")
            .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
            .and_then(|value| value.as_str()),
        Some(PROXY_TOKEN_PLACEHOLDER)
    );
    assert!(
        live_config
            .get("env")
            .and_then(|env| env.get("ANTHROPIC_API_KEY"))
            .is_none(),
        "non-managed providers should retain the legacy fallback behavior"
    );
}

#[test]
#[serial]
fn codex_custom_provider_live_write_preserves_oauth_auth_json() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db);
    let oauth_auth = json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "id_token": "oauth-id",
            "access_token": "oauth-access"
        }
    });
    crate::codex_config::write_codex_live_atomic(
        &oauth_auth,
        Some(
            r#"model_provider = "openai"
model = "gpt-5-codex"
"#,
        ),
    )
    .expect("seed live OAuth auth");

    let mut provider = Provider::with_id(
        "rightcode".to_string(),
        "RightCode".to_string(),
        json!({
            "auth": {
                "OPENAI_API_KEY": "rightcode-key"
            },
            "config": r#"model_provider = "rightcode"
model = "gpt-5-codex"

[model_providers.rightcode]
name = "RightCode"
base_url = "https://rightcode.example/v1"
wire_api = "responses"
"#
        }),
        None,
    );
    provider.category = Some("custom".to_string());
    let takeover_settings = json!({
        "auth": {
            "OPENAI_API_KEY": PROXY_TOKEN_PLACEHOLDER
        },
        "config": r#"model_provider = "rightcode"
model = "gpt-5-codex"

[model_providers.rightcode]
name = "RightCode"
base_url = "http://127.0.0.1:15721/v1"
wire_api = "responses"
"#
    });

    service
        .write_codex_live_for_provider(&takeover_settings, Some(&provider))
        .expect("write provider-driven Codex live config");

    let live_auth: Value =
        crate::config::read_json_file(&crate::codex_config::get_codex_auth_path())
            .expect("read live auth");
    assert_eq!(
        live_auth, oauth_auth,
        "third-party Codex proxy writes must not overwrite ChatGPT OAuth login state"
    );

    let live_config = std::fs::read_to_string(crate::codex_config::get_codex_config_path())
        .expect("read live config");
    assert!(
        live_config.contains("experimental_bearer_token"),
        "proxy placeholder should move into config.toml instead of auth.json"
    );
    assert!(
        live_config.contains(PROXY_TOKEN_PLACEHOLDER),
        "live config should carry the proxy placeholder token"
    );
}

#[test]
fn update_toml_base_url_updates_active_model_provider_base_url() {
    let input = r#"
model_provider = "any"
model = "gpt-5.1-codex"
disable_response_storage = true

[model_providers.any]
name = "any"
base_url = "https://anyrouter.top/v1"
wire_api = "responses"
requires_openai_auth = true
"#;

    let new_url = "http://127.0.0.1:5000/v1";
    let output = ProxyService::update_toml_base_url(input, new_url);

    let parsed: toml::Value =
        toml::from_str(&output).expect("updated config should be valid TOML");

    let base_url = parsed
        .get("model_providers")
        .and_then(|v| v.get("any"))
        .and_then(|v| v.get("base_url"))
        .and_then(|v| v.as_str())
        .expect("model_providers.any.base_url should exist");

    assert_eq!(base_url, new_url);
    assert!(
        parsed.get("base_url").is_none(),
        "should not write top-level base_url"
    );

    let wire_api = parsed
        .get("model_providers")
        .and_then(|v| v.get("any"))
        .and_then(|v| v.get("wire_api"))
        .and_then(|v| v.as_str())
        .expect("model_providers.any.wire_api should exist");
    assert_eq!(wire_api, "responses");
}

#[test]
fn apply_codex_proxy_toml_config_forces_local_responses_wire_api() {
    let input = r#"
model_provider = "chat_only"
model = "gpt-5.1-codex"

[model_providers.chat_only]
name = "Chat Only"
base_url = "https://chat-only.example/v1"
wire_api = "chat"
"#;

    let proxy_url = "http://127.0.0.1:5000/v1";
    let output =
        ProxyService::apply_codex_proxy_toml_config_for_provider(input, proxy_url, None);
    let parsed: toml::Value =
        toml::from_str(&output).expect("updated config should be valid TOML");

    let provider = parsed
        .get("model_providers")
        .and_then(|v| v.get("chat_only"))
        .expect("model_providers.chat_only should exist");

    assert_eq!(
        provider.get("base_url").and_then(|v| v.as_str()),
        Some(proxy_url)
    );
    assert_eq!(
        provider.get("wire_api").and_then(|v| v.as_str()),
        Some("responses")
    );
}

#[test]
fn apply_codex_proxy_toml_config_keeps_upstream_model_for_chat_provider() {
    let input = r#"
model_provider = "deepseek"
model = "deepseek-v4-flash"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "responses"
"#;
    let mut provider = Provider::with_id(
        "deepseek".to_string(),
        "DeepSeek".to_string(),
        json!({
            "config": input
        }),
        None,
    );
    provider.meta = Some(ProviderMeta {
        api_format: Some("openai_chat".to_string()),
        ..Default::default()
    });

    let proxy_url = "http://127.0.0.1:5000/v1";
    let output = ProxyService::apply_codex_proxy_toml_config_for_provider(
        input,
        proxy_url,
        Some(&provider),
    );
    let parsed: toml::Value =
        toml::from_str(&output).expect("updated config should be valid TOML");

    assert_eq!(
        parsed.get("model").and_then(|v| v.as_str()),
        Some("deepseek-v4-flash")
    );
    assert_eq!(
        parsed
            .get("model_providers")
            .and_then(|v| v.get("deepseek"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str()),
        Some(proxy_url)
    );
}

#[test]
fn apply_codex_proxy_toml_config_preserves_model_for_responses_provider() {
    let input = r#"
model_provider = "responses"
model = "upstream-responses-model"

[model_providers.responses]
name = "Responses"
base_url = "https://responses.example/v1"
wire_api = "responses"
"#;
    let mut provider = Provider::with_id(
        "responses".to_string(),
        "Responses".to_string(),
        json!({
            "config": input
        }),
        None,
    );
    provider.meta = Some(ProviderMeta {
        api_format: Some("openai_responses".to_string()),
        ..Default::default()
    });

    let output = ProxyService::apply_codex_proxy_toml_config_for_provider(
        input,
        "http://127.0.0.1:5000/v1",
        Some(&provider),
    );
    let parsed: toml::Value =
        toml::from_str(&output).expect("updated config should be valid TOML");

    assert_eq!(
        parsed.get("model").and_then(|v| v.as_str()),
        Some("upstream-responses-model")
    );
}

#[test]
fn apply_codex_proxy_toml_config_restores_upstream_model_for_responses_provider() {
    let input = r#"
model_provider = "responses"
model = "gpt-5.4"

[model_providers.responses]
name = "Responses"
base_url = "http://127.0.0.1:5000/v1"
wire_api = "responses"
"#;
    let mut provider = Provider::with_id(
        "responses".to_string(),
        "Responses".to_string(),
        json!({
            "config": r#"model_provider = "responses"
model = "upstream-responses-model"

[model_providers.responses]
name = "Responses"
base_url = "https://responses.example/v1"
wire_api = "responses"
"#
        }),
        None,
    );
    provider.meta = Some(ProviderMeta {
        api_format: Some("openai_responses".to_string()),
        ..Default::default()
    });

    let output = ProxyService::apply_codex_proxy_toml_config_for_provider(
        input,
        "http://127.0.0.1:5000/v1",
        Some(&provider),
    );
    let parsed: toml::Value =
        toml::from_str(&output).expect("updated config should be valid TOML");

    assert_eq!(
        parsed.get("model").and_then(|v| v.as_str()),
        Some("upstream-responses-model")
    );
}

#[test]
fn update_toml_base_url_falls_back_to_top_level_base_url() {
    let input = r#"
model = "gpt-5.1-codex"
"#;

    let new_url = "http://127.0.0.1:5000/v1";
    let output = ProxyService::update_toml_base_url(input, new_url);

    let parsed: toml::Value =
        toml::from_str(&output).expect("updated config should be valid TOML");

    let base_url = parsed
        .get("base_url")
        .and_then(|v| v.as_str())
        .expect("base_url should exist");

    assert_eq!(base_url, new_url);
}

#[tokio::test]
#[serial]
async fn sync_claude_token_does_not_add_anthropic_api_key() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    let provider = Provider::with_id(
        "p1".to_string(),
        "P1".to_string(),
        json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
                "ANTHROPIC_AUTH_TOKEN": "stale"
            }
        }),
        None,
    );
    db.save_provider("claude", &provider)
        .expect("save provider");
    db.set_current_provider("claude", "p1")
        .expect("set current provider");

    let live_config = json!({
        "env": {
            "ANTHROPIC_AUTH_TOKEN": "fresh"
        }
    });

    service
        .sync_live_config_to_provider(&AppType::Claude, &live_config)
        .await
        .expect("sync");

    let updated = db
        .get_provider_by_id("p1", "claude")
        .expect("get provider")
        .expect("provider exists");
    let env = updated
        .settings_config
        .get("env")
        .and_then(|v| v.as_object())
        .expect("env object");

    assert_eq!(
        env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()),
        Some("fresh")
    );
    assert!(
        !env.contains_key("ANTHROPIC_API_KEY"),
        "should not add ANTHROPIC_API_KEY when absent"
    );
}

#[tokio::test]
#[serial]
async fn sync_claude_token_respects_existing_api_key_field() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    let provider = Provider::with_id(
        "p1".to_string(),
        "P1".to_string(),
        json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
                "ANTHROPIC_API_KEY": "stale"
            }
        }),
        None,
    );
    db.save_provider("claude", &provider)
        .expect("save provider");
    db.set_current_provider("claude", "p1")
        .expect("set current provider");

    let live_config = json!({
        "env": {
            "ANTHROPIC_AUTH_TOKEN": "fresh"
        }
    });

    service
        .sync_live_config_to_provider(&AppType::Claude, &live_config)
        .await
        .expect("sync");

    let updated = db
        .get_provider_by_id("p1", "claude")
        .expect("get provider")
        .expect("provider exists");
    let env = updated
        .settings_config
        .get("env")
        .and_then(|v| v.as_object())
        .expect("env object");

    assert_eq!(
        env.get("ANTHROPIC_API_KEY").and_then(|v| v.as_str()),
        Some("fresh")
    );
    assert!(
        !env.contains_key("ANTHROPIC_AUTH_TOKEN"),
        "should not add ANTHROPIC_AUTH_TOKEN when absent"
    );
}

#[tokio::test]
#[serial]
async fn switch_proxy_target_updates_live_backup_when_taken_over() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    let provider_a = Provider::with_id(
        "a".to_string(),
        "A".to_string(),
        json!({
            "env": {
                "ANTHROPIC_API_KEY": "a-key"
            }
        }),
        None,
    );
    let provider_b = Provider::with_id(
        "b".to_string(),
        "B".to_string(),
        json!({
            "env": {
                "ANTHROPIC_API_KEY": "b-key"
            }
        }),
        None,
    );
    db.save_provider("claude", &provider_a)
        .expect("save provider a");
    db.save_provider("claude", &provider_b)
        .expect("save provider b");
    db.set_current_provider("claude", "a")
        .expect("set current provider");

    // 模拟"已接管"状态：存在 Live 备份（内容不重要，会被热切换更新）
    db.save_live_backup("claude", "{\"env\":{}}")
        .await
        .expect("seed live backup");

    service
        .switch_proxy_target("claude", "b")
        .await
        .expect("switch proxy target");

    // 断言：本地 settings 的 current provider 已同步
    assert_eq!(
        crate::settings::get_current_provider(&AppType::Claude).as_deref(),
        Some("b")
    );

    // 断言：Live 备份已更新为目标供应商配置（用于 stop_with_restore 恢复）
    let backup = db
        .get_live_backup("claude")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let expected = serde_json::to_string(&provider_b.settings_config).expect("serialize");
    assert_eq!(backup.original_config, expected);
}

#[tokio::test]
#[serial]
async fn hot_switch_provider_updates_claude_live_while_preserving_takeover_fields() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    let provider_a = Provider::with_id(
        "a".to_string(),
        "A".to_string(),
        json!({
            "env": {
                "ANTHROPIC_API_KEY": "a-key",
                "ANTHROPIC_BASE_URL": "https://api.a.example",
                "ANTHROPIC_MODEL": "claude-old"
            },
            "permissions": { "allow": ["Bash"] }
        }),
        None,
    );
    let provider_b = Provider::with_id(
        "b".to_string(),
        "B".to_string(),
        json!({
            "env": {
                "ANTHROPIC_API_KEY": "b-key",
                "ANTHROPIC_BASE_URL": "https://api.b.example",
                "ANTHROPIC_MODEL": "claude-new",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "deepseek-v4-flash",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME": "DeepSeek V4 Flash",
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "deepseek-v4-pro[1M]",
                "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME": "DeepSeek V4 Pro",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "deepseek-v4-ultra [1m]"
            },
            "permissions": { "allow": ["Read"] }
        }),
        None,
    );

    db.save_provider("claude", &provider_a)
        .expect("save provider a");
    db.save_provider("claude", &provider_b)
        .expect("save provider b");
    db.set_current_provider("claude", "a")
        .expect("set current provider");
    crate::settings::set_current_provider(&AppType::Claude, Some("a"))
        .expect("set local current provider");
    db.save_live_backup(
        "claude",
        &serde_json::to_string(&provider_a.settings_config).expect("serialize provider a"),
    )
    .await
    .expect("seed live backup");
    service
        .write_claude_live(&json!({
            "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721",
                "ANTHROPIC_API_KEY": PROXY_TOKEN_PLACEHOLDER,
                "ANTHROPIC_MODEL": "stale-model",
                "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME": "Stale Sonnet"
            },
            "permissions": { "allow": ["Bash"] }
        }))
        .expect("seed taken-over live file");

    service
        .hot_switch_provider("claude", "b")
        .await
        .expect("hot switch provider");

    let live = service.read_claude_live().expect("read live config");
    assert_eq!(
        live.get("permissions"),
        provider_b.settings_config.get("permissions"),
        "provider-derived live settings should be refreshed"
    );
    assert_eq!(
        live.get("env")
            .and_then(|env| env.get("ANTHROPIC_API_KEY"))
            .and_then(|v| v.as_str()),
        Some(PROXY_TOKEN_PLACEHOLDER),
        "takeover token placeholder should be preserved"
    );
    assert_eq!(
        live.get("env")
            .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
            .and_then(|v| v.as_str()),
        Some("http://127.0.0.1:15721"),
        "takeover proxy URL should remain active"
    );
    assert!(
        live.get("env")
            .and_then(|env| env.get("ANTHROPIC_MODEL"))
            .is_none(),
        "fallback model override should be removed in takeover mode"
    );
    let live_env = live
        .get("env")
        .and_then(|env| env.as_object())
        .expect("live env");
    assert_eq!(
        live_env
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
            .and_then(|v| v.as_str()),
        Some("claude-haiku-4-5"),
        "takeover mode should expose a stable Haiku role model"
    );
    assert_eq!(
        live_env
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME")
            .and_then(|v| v.as_str()),
        Some("DeepSeek V4 Flash"),
        "model menu should show the current provider Haiku display name"
    );
    assert_eq!(
        live_env
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .and_then(|v| v.as_str()),
        Some("claude-sonnet-4-6[1M]"),
        "Sonnet role should carry the local 1M declaration for Claude Code"
    );
    assert_eq!(
        live_env
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL_NAME")
            .and_then(|v| v.as_str()),
        Some("DeepSeek V4 Pro"),
        "stale model display names should be replaced during hot switch"
    );
    assert_eq!(
        live_env
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .and_then(|v| v.as_str()),
        Some("claude-opus-4-7[1M]"),
        "Opus role should preserve the current provider 1M capability marker"
    );
    assert_eq!(
        live_env
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL_NAME")
            .and_then(|v| v.as_str()),
        Some("deepseek-v4-ultra"),
        "implicit display names should strip the local 1M marker"
    );

    let backup = db
        .get_live_backup("claude")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let expected = serde_json::to_string(&provider_b.settings_config).expect("serialize");
    assert_eq!(backup.original_config, expected);
}

#[tokio::test]
#[serial]
async fn hot_switch_provider_serializes_same_app_switches() {
    use tokio::time::{sleep, Duration};

    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    let provider_a = Provider::with_id(
        "a".to_string(),
        "A".to_string(),
        json!({ "env": { "ANTHROPIC_API_KEY": "a-key" } }),
        None,
    );
    let provider_b = Provider::with_id(
        "b".to_string(),
        "B".to_string(),
        json!({ "env": { "ANTHROPIC_API_KEY": "b-key" } }),
        None,
    );
    let provider_c = Provider::with_id(
        "c".to_string(),
        "C".to_string(),
        json!({ "env": { "ANTHROPIC_API_KEY": "c-key" } }),
        None,
    );

    db.save_provider("claude", &provider_a)
        .expect("save provider a");
    db.save_provider("claude", &provider_b)
        .expect("save provider b");
    db.save_provider("claude", &provider_c)
        .expect("save provider c");
    db.set_current_provider("claude", "a")
        .expect("set current provider");
    crate::settings::set_current_provider(&AppType::Claude, Some("a"))
        .expect("set local current provider");
    db.save_live_backup("claude", "{\"env\":{}}")
        .await
        .expect("seed live backup");

    let guard = service.lock_switch_for_test("claude").await;
    let service_for_b = service.clone();
    let service_for_c = service.clone();

    let switch_b = tokio::spawn(async move {
        service_for_b
            .hot_switch_provider("claude", "b")
            .await
            .expect("switch to b")
    });
    sleep(Duration::from_millis(20)).await;
    let switch_c = tokio::spawn(async move {
        service_for_c
            .hot_switch_provider("claude", "c")
            .await
            .expect("switch to c")
    });

    sleep(Duration::from_millis(20)).await;
    drop(guard);

    let outcome_b = switch_b.await.expect("join switch b");
    let outcome_c = switch_c.await.expect("join switch c");
    assert!(outcome_b.logical_target_changed);
    assert!(outcome_c.logical_target_changed);

    assert_eq!(
        crate::settings::get_effective_current_provider(&db, &AppType::Claude)
            .expect("effective current"),
        Some("c".to_string())
    );
    assert_eq!(
        crate::settings::get_current_provider(&AppType::Claude).as_deref(),
        Some("c")
    );
    assert_eq!(
        db.get_current_provider("claude").expect("db current"),
        Some("c".to_string())
    );

    let backup = db
        .get_live_backup("claude")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let expected = serde_json::to_string(&provider_c.settings_config).expect("serialize");
    assert_eq!(backup.original_config, expected);
}

#[tokio::test]
#[serial]
async fn restore_waits_for_hot_switch_and_restores_latest_backup() {
    use tokio::time::{sleep, Duration};

    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    let provider_a = Provider::with_id(
        "a".to_string(),
        "A".to_string(),
        json!({ "env": { "ANTHROPIC_API_KEY": "a-key" } }),
        None,
    );
    let provider_b = Provider::with_id(
        "b".to_string(),
        "B".to_string(),
        json!({ "env": { "ANTHROPIC_API_KEY": "b-key" } }),
        None,
    );

    db.save_provider("claude", &provider_a)
        .expect("save provider a");
    db.save_provider("claude", &provider_b)
        .expect("save provider b");
    db.set_current_provider("claude", "a")
        .expect("set current provider");
    crate::settings::set_current_provider(&AppType::Claude, Some("a"))
        .expect("set local current provider");
    db.save_live_backup(
        "claude",
        &serde_json::to_string(&provider_a.settings_config).expect("serialize provider a"),
    )
    .await
    .expect("seed live backup");
    service
        .write_claude_live(&json!({ "env": { "ANTHROPIC_API_KEY": "stale" } }))
        .expect("seed live file");

    let guard = service.lock_switch_for_test("claude").await;
    let service_for_switch = service.clone();
    let service_for_restore = service.clone();

    let switch_to_b = tokio::spawn(async move {
        service_for_switch
            .hot_switch_provider("claude", "b")
            .await
            .expect("switch to b")
    });
    sleep(Duration::from_millis(20)).await;
    let restore = tokio::spawn(async move {
        service_for_restore
            .restore_live_config_for_app_with_fallback(&AppType::Claude)
            .await
            .expect("restore claude live")
    });

    sleep(Duration::from_millis(20)).await;
    drop(guard);

    let outcome = switch_to_b.await.expect("join switch");
    restore.await.expect("join restore");
    assert!(outcome.logical_target_changed);

    assert_eq!(
        crate::settings::get_effective_current_provider(&db, &AppType::Claude)
            .expect("effective current"),
        Some("b".to_string())
    );

    let backup = db
        .get_live_backup("claude")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let expected = serde_json::to_string(&provider_b.settings_config).expect("serialize");
    assert_eq!(backup.original_config, expected);
    assert_eq!(
        service.read_claude_live().expect("read live"),
        provider_b.settings_config
    );
}

#[tokio::test]
#[serial]
async fn update_live_backup_from_provider_applies_claude_common_config() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    db.set_config_snippet(
        "claude",
        Some(
            serde_json::json!({
                "includeCoAuthoredBy": false
            })
            .to_string(),
        ),
    )
    .expect("set common config snippet");

    let service = ProxyService::new(db.clone());

    let mut provider = Provider::with_id(
        "p1".to_string(),
        "P1".to_string(),
        json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "token",
                "ANTHROPIC_BASE_URL": "https://claude.example"
            }
        }),
        None,
    );
    provider.meta = Some(ProviderMeta {
        common_config_enabled: Some(true),
        ..Default::default()
    });

    service
        .update_live_backup_from_provider("claude", &provider)
        .await
        .expect("update live backup");

    let backup = db
        .get_live_backup("claude")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let stored: Value =
        serde_json::from_str(&backup.original_config).expect("parse backup json");

    assert_eq!(
        stored.get("includeCoAuthoredBy").and_then(|v| v.as_bool()),
        Some(false),
        "common config should be applied into Claude restore backup"
    );
}

#[tokio::test]
#[serial]
async fn update_live_backup_from_provider_applies_codex_common_config() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    db.set_config_snippet(
        "codex",
        Some("disable_response_storage = true\n".to_string()),
    )
    .expect("set common config snippet");

    let service = ProxyService::new(db.clone());

    let mut provider = Provider::with_id(
        "p1".to_string(),
        "P1".to_string(),
        json!({
            "auth": {
                "OPENAI_API_KEY": "token"
            },
            "config": r#"model_provider = "any"
model = "gpt-5"

[model_providers.any]
base_url = "https://codex.example/v1"
"#
        }),
        None,
    );
    provider.meta = Some(ProviderMeta {
        common_config_enabled: Some(true),
        ..Default::default()
    });

    service
        .update_live_backup_from_provider("codex", &provider)
        .await
        .expect("update live backup");

    let backup = db
        .get_live_backup("codex")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let stored: Value =
        serde_json::from_str(&backup.original_config).expect("parse backup json");
    let config = stored
        .get("config")
        .and_then(|v| v.as_str())
        .expect("config string");

    assert!(
        config.contains("disable_response_storage = true"),
        "common config should be applied into Codex restore backup"
    );
}

#[tokio::test]
#[serial]
async fn update_live_backup_from_provider_preserves_codex_mcp_servers() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    db.save_live_backup(
        "codex",
        &serde_json::to_string(&json!({
            "auth": {
                "OPENAI_API_KEY": "old-token"
            },
            "config": r#"model_provider = "any"
model = "gpt-4"

[model_providers.any]
base_url = "https://old.example/v1"

[mcp_servers.echo]
command = "npx"
args = ["echo-server"]
"#
        }))
        .expect("serialize seed backup"),
    )
    .await
    .expect("seed live backup");

    let provider = Provider::with_id(
        "p2".to_string(),
        "P2".to_string(),
        json!({
            "auth": {
                "OPENAI_API_KEY": "new-token"
            },
            "config": r#"model_provider = "any"
model = "gpt-5"

[model_providers.any]
base_url = "https://new.example/v1"
"#
        }),
        None,
    );

    service
        .update_live_backup_from_provider("codex", &provider)
        .await
        .expect("update live backup");

    let backup = db
        .get_live_backup("codex")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let stored: Value =
        serde_json::from_str(&backup.original_config).expect("parse backup json");
    let config = stored
        .get("config")
        .and_then(|v| v.as_str())
        .expect("config string");

    assert!(
        config.contains("[mcp_servers.echo]"),
        "existing Codex MCP section should survive proxy hot-switch backup update"
    );
    assert!(
        config.contains("https://new.example/v1"),
        "provider-specific base_url should still update to the new provider"
    );
}

#[tokio::test]
#[serial]
async fn hot_switch_codex_provider_keeps_model_provider_stable_in_backup_and_restore() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    let provider_a = Provider::with_id(
        "a".to_string(),
        "RightCode".to_string(),
        json!({
            "auth": {
                "OPENAI_API_KEY": "rightcode-key"
            },
            "config": r#"model_provider = "rightcode"
model = "gpt-5.4"

[model_providers.rightcode]
name = "RightCode"
base_url = "https://rightcode.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        }),
        None,
    );
    let provider_b = Provider::with_id(
        "b".to_string(),
        "AiHubMix".to_string(),
        json!({
            "auth": {
                "OPENAI_API_KEY": "aihubmix-key"
            },
            "config": r#"model_provider = "aihubmix"
model = "gpt-5.4"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        }),
        None,
    );

    db.save_provider("codex", &provider_a)
        .expect("save provider a");
    db.save_provider("codex", &provider_b)
        .expect("save provider b");
    db.set_current_provider("codex", "a")
        .expect("set current provider");
    crate::settings::set_current_provider(&AppType::Codex, Some("a"))
        .expect("set local current provider");
    db.save_live_backup(
        "codex",
        &serde_json::to_string(&provider_a.settings_config).expect("serialize provider a"),
    )
    .await
    .expect("seed live backup");
    service
        .write_codex_live(&json!({
            "auth": {
                "OPENAI_API_KEY": PROXY_TOKEN_PLACEHOLDER
            },
            "config": r#"model_provider = "rightcode"
model = "gpt-5.4"

[model_providers.rightcode]
name = "RightCode"
base_url = "http://127.0.0.1:15721/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        }))
        .expect("seed taken-over Codex live config");

    service
        .hot_switch_provider("codex", "b")
        .await
        .expect("hot switch Codex provider");

    let backup = db
        .get_live_backup("codex")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let stored: Value =
        serde_json::from_str(&backup.original_config).expect("parse backup json");
    let backup_config = stored
        .get("config")
        .and_then(|v| v.as_str())
        .expect("backup config string");
    let parsed_backup: toml::Value =
        toml::from_str(backup_config).expect("parse backup config");
    assert_eq!(
        parsed_backup.get("model_provider").and_then(|v| v.as_str()),
        Some("custom"),
        "provider-derived restore backup should retain stable Codex model_provider"
    );
    let backup_model_providers = parsed_backup
        .get("model_providers")
        .and_then(|v| v.get("custom"))
        .and_then(|v| v.as_table())
        .expect("backup model_providers");
    assert!(parsed_backup.get("model_providers").and_then(|v| v.get("aihubmix")).is_none());
    assert_eq!(
        backup_model_providers
            .get("base_url")
            .and_then(|v| v.as_str()),
        Some("https://aihubmix.example/v1"),
        "stable provider id should point at the hot-switched provider endpoint"
    );

    service
        .restore_live_config_for_app_with_fallback(&AppType::Codex)
        .await
        .expect("restore Codex live config");

    let live = service.read_codex_live().expect("read Codex live config");
    let live_config = live
        .get("config")
        .and_then(|v| v.as_str())
        .expect("live config string");
    let parsed_live: toml::Value = toml::from_str(live_config).expect("parse live config");
    assert_eq!(
        parsed_live.get("model_provider").and_then(|v| v.as_str()),
        Some("custom"),
        "restored Codex live config should not switch history buckets"
    );
    assert_eq!(
        live.get("auth")
            .and_then(|auth| auth.get("OPENAI_API_KEY"))
            .and_then(|v| v.as_str()),
        Some("aihubmix-key"),
        "restore should still use the hot-switched provider auth"
    );
}

#[tokio::test]
#[serial]
async fn hot_switch_codex_chat_provider_uses_upstream_model_without_changing_live_provider() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    let provider_a = Provider::with_id(
        "a".to_string(),
        "Responses".to_string(),
        json!({
            "auth": {
                "OPENAI_API_KEY": "responses-key"
            },
            "config": r#"model_provider = "stable"
model = "responses-model"

[model_providers.stable]
name = "Stable"
base_url = "https://responses.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        }),
        None,
    );
    let mut provider_b = Provider::with_id(
        "b".to_string(),
        "DeepSeek".to_string(),
        json!({
            "auth": {
                "OPENAI_API_KEY": "deepseek-key"
            },
            "config": r#"model_provider = "deepseek"
model = "deepseek-v4-flash"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        }),
        None,
    );
    provider_b.meta = Some(ProviderMeta {
        api_format: Some("openai_chat".to_string()),
        ..Default::default()
    });

    db.save_provider("codex", &provider_a)
        .expect("save provider a");
    db.save_provider("codex", &provider_b)
        .expect("save provider b");
    db.set_current_provider("codex", "a")
        .expect("set current provider");
    crate::settings::set_current_provider(&AppType::Codex, Some("a"))
        .expect("set local current provider");
    db.save_live_backup(
        "codex",
        &serde_json::to_string(&provider_a.settings_config).expect("serialize provider a"),
    )
    .await
    .expect("seed live backup");
    service
        .write_codex_live(&json!({
            "auth": {
                "OPENAI_API_KEY": PROXY_TOKEN_PLACEHOLDER
            },
            "config": r#"model_provider = "stable"
model = "responses-model"

[model_providers.stable]
name = "Stable"
base_url = "http://127.0.0.1:15721/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        }))
        .expect("seed taken-over Codex live config");

    service
        .hot_switch_provider("codex", "b")
        .await
        .expect("hot switch Codex provider");

    let live = service.read_codex_live().expect("read Codex live config");
    let live_config = live
        .get("config")
        .and_then(|v| v.as_str())
        .expect("live config string");
    let parsed_live: toml::Value = toml::from_str(live_config).expect("parse live config");

    assert_eq!(
        parsed_live.get("model_provider").and_then(|v| v.as_str()),
        Some("custom")
    );
    assert_eq!(
        parsed_live.get("model").and_then(|v| v.as_str()),
        Some("deepseek-v4-flash")
    );
    assert_eq!(
        live.get("auth")
            .and_then(|auth| auth.get("OPENAI_API_KEY"))
            .and_then(|v| v.as_str()),
        Some(PROXY_TOKEN_PLACEHOLDER)
    );
}

#[tokio::test]
#[serial]
async fn update_live_backup_from_provider_keeps_new_codex_mcp_entries_on_conflict() {
    let _home = TempHome::new();
    crate::settings::reload_settings().expect("reload settings");

    let db = Arc::new(Database::memory().expect("init db"));
    let service = ProxyService::new(db.clone());

    db.save_live_backup(
        "codex",
        &serde_json::to_string(&json!({
            "auth": {
                "OPENAI_API_KEY": "old-token"
            },
            "config": r#"[mcp_servers.shared]
command = "old-command"

[mcp_servers.legacy]
command = "legacy-command"
"#
        }))
        .expect("serialize seed backup"),
    )
    .await
    .expect("seed live backup");

    let provider = Provider::with_id(
        "p2".to_string(),
        "P2".to_string(),
        json!({
            "auth": {
                "OPENAI_API_KEY": "new-token"
            },
            "config": r#"[mcp_servers.shared]
command = "new-command"

[mcp_servers.latest]
command = "latest-command"
"#
        }),
        None,
    );

    service
        .update_live_backup_from_provider("codex", &provider)
        .await
        .expect("update live backup");

    let backup = db
        .get_live_backup("codex")
        .await
        .expect("get live backup")
        .expect("backup exists");
    let stored: Value =
        serde_json::from_str(&backup.original_config).expect("parse backup json");
    let config = stored
        .get("config")
        .and_then(|v| v.as_str())
        .expect("config string");
    let parsed: toml::Value = toml::from_str(config).expect("parse merged codex config");

    let mcp_servers = parsed
        .get("mcp_servers")
        .expect("mcp_servers should be present");
    assert_eq!(
        mcp_servers
            .get("shared")
            .and_then(|v| v.get("command"))
            .and_then(|v| v.as_str()),
        Some("new-command"),
        "new provider/common-config MCP definition should win on conflict"
    );
    assert_eq!(
        mcp_servers
            .get("legacy")
            .and_then(|v| v.get("command"))
            .and_then(|v| v.as_str()),
        Some("legacy-command"),
        "backup-only MCP entries should still be preserved"
    );
    assert_eq!(
        mcp_servers
            .get("latest")
            .and_then(|v| v.get("command"))
            .and_then(|v| v.as_str()),
        Some("latest-command"),
        "new MCP entries should remain in the restore backup"
    );
}
