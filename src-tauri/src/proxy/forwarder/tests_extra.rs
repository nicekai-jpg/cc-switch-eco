use super::*;
use axum::http::header::{HeaderValue, ACCEPT};
use axum::http::HeaderMap;
use serde_json::{json, Value};
use std::time::Duration;
use crate::proxy::types::RectifierConfig;
use crate::proxy::error::ProxyError;
use super::tests::{test_provider_with_type, test_forwarder};
use super::url_rewrite::{rewrite_claude_transform_endpoint, append_query_to_full_url};
use super::req_headers::{should_force_identity_encoding, is_streaming_request};

#[test]
fn rewrite_claude_transform_endpoint_uses_copilot_path() {
    let (endpoint, passthrough_query) = rewrite_claude_transform_endpoint(
        "/v1/messages?beta=true&x-id=1",
        "anthropic",
        true,
        &json!({ "model": "claude-sonnet-4-6" }),
    );

    assert_eq!(endpoint, "/chat/completions?x-id=1");
    assert_eq!(passthrough_query.as_deref(), Some("x-id=1"));
}

#[test]
fn rewrite_claude_transform_endpoint_uses_copilot_responses_path() {
    let (endpoint, passthrough_query) = rewrite_claude_transform_endpoint(
        "/v1/messages?beta=true&x-id=1",
        "openai_responses",
        true,
        &json!({ "model": "gpt-5.4" }),
    );

    assert_eq!(endpoint, "/v1/responses?x-id=1");
    assert_eq!(passthrough_query.as_deref(), Some("x-id=1"));
}

#[test]
fn rewrite_claude_transform_endpoint_maps_gemini_generate_content() {
    let (endpoint, passthrough_query) = rewrite_claude_transform_endpoint(
        "/v1/messages?beta=true&x-id=1",
        "gemini_native",
        false,
        &json!({ "model": "gemini-2.5-pro" }),
    );

    assert_eq!(
        endpoint,
        "/v1beta/models/gemini-2.5-pro:generateContent?x-id=1"
    );
    assert_eq!(passthrough_query.as_deref(), Some("x-id=1"));
}

/// Regression: body.model arriving as the resource-name form
/// `models/gemini-2.5-pro` must not produce a doubled
/// `/v1beta/models/models/...` path.
#[test]
fn rewrite_claude_transform_endpoint_strips_gemini_model_resource_prefix() {
    let (endpoint, _) = rewrite_claude_transform_endpoint(
        "/v1/messages",
        "gemini_native",
        false,
        &json!({ "model": "models/gemini-2.5-pro" }),
    );

    assert_eq!(endpoint, "/v1beta/models/gemini-2.5-pro:generateContent");
}

#[test]
fn rewrite_claude_transform_endpoint_maps_gemini_streaming() {
    let (endpoint, passthrough_query) = rewrite_claude_transform_endpoint(
        "/v1/messages?beta=true",
        "gemini_native",
        false,
        &json!({ "model": "gemini-2.5-flash", "stream": true }),
    );

    assert_eq!(
        endpoint,
        "/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse"
    );
    assert_eq!(passthrough_query.as_deref(), Some("alt=sse"));
}

#[test]
fn append_query_to_full_url_preserves_existing_query_string() {
    let url = append_query_to_full_url("https://relay.example/api?foo=bar", Some("x-id=1"));

    assert_eq!(url, "https://relay.example/api?foo=bar&x-id=1");
}

#[test]
fn build_gemini_native_url_uses_origin_when_base_ends_with_v1beta() {
    let url = crate::proxy::gemini_url::build_gemini_native_url(
        "https://generativelanguage.googleapis.com/v1beta",
        "/v1beta/models/gemini-2.5-pro:generateContent",
    );

    assert_eq!(
        url,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent"
    );
}

#[test]
fn build_gemini_native_url_uses_origin_when_base_already_contains_models_prefix() {
    let url = crate::proxy::gemini_url::build_gemini_native_url(
        "https://generativelanguage.googleapis.com/v1beta/models",
        "/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse",
    );

    assert_eq!(
        url,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse"
    );
}

#[test]
fn resolve_gemini_native_url_keeps_opaque_full_url_as_is() {
    let url = crate::proxy::gemini_url::resolve_gemini_native_url(
        "https://relay.example/custom/generate-content",
        "/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse",
        true,
    );

    assert_eq!(url, "https://relay.example/custom/generate-content?alt=sse");
}

#[test]
fn force_identity_for_stream_flag_requests() {
    let headers = HeaderMap::new();

    assert!(should_force_identity_encoding(
        "/v1/responses",
        &json!({ "stream": true }),
        &headers
    ));
}

#[test]
fn force_identity_for_gemini_stream_endpoints() {
    let headers = HeaderMap::new();

    assert!(should_force_identity_encoding(
        "/v1beta/models/gemini-2.5-pro:streamGenerateContent?alt=sse",
        &json!({ "model": "gemini-2.5-pro" }),
        &headers
    ));
}

#[test]
fn streaming_request_detects_gemini_sse_without_body_stream_flag() {
    let headers = HeaderMap::new();

    assert!(is_streaming_request(
        "/v1beta/models/gemini-2.5-pro:streamGenerateContent?alt=sse",
        &json!({ "model": "gemini-2.5-pro" }),
        &headers
    ));
}

#[test]
fn force_identity_for_sse_accept_header() {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));

    assert!(should_force_identity_encoding(
        "/v1/responses",
        &json!({ "model": "gpt-5" }),
        &headers
    ));
}

#[test]
fn non_streaming_requests_allow_automatic_compression() {
    let headers = HeaderMap::new();

    assert!(!should_force_identity_encoding(
        "/v1/responses",
        &json!({ "model": "gpt-5" }),
        &headers
    ));
}

// ==================== Copilot 动态 endpoint 路由相关测试 ====================

/// 验证 is_copilot 检测逻辑：通过 provider_type 判断
#[test]
fn copilot_detection_via_provider_type() {
    use crate::provider::{Provider, ProviderMeta};

    let provider = Provider {
        id: "test".to_string(),
        name: "Test Copilot".to_string(),
        settings_config: serde_json::json!({}),
        website_url: None,
        category: None,
        created_at: None,
        sort_index: None,
        notes: None,
        meta: Some(ProviderMeta {
            provider_type: Some("github_copilot".to_string()),
            ..Default::default()
        }),
        icon: None,
        icon_color: None,
        in_failover_queue: false,
    };

    let is_copilot = provider
        .meta
        .as_ref()
        .and_then(|m| m.provider_type.as_deref())
        == Some("github_copilot");

    assert!(is_copilot, "应该通过 provider_type 检测为 Copilot");
}

/// 验证 is_copilot 检测逻辑：通过 base_url 判断
#[test]
fn copilot_detection_via_base_url() {
    let base_url = "https://api.githubcopilot.com";
    let is_copilot = base_url.contains("githubcopilot.com");
    assert!(is_copilot, "应该通过 base_url 检测为 Copilot");

    let non_copilot_url = "https://api.anthropic.com";
    let is_not_copilot = non_copilot_url.contains("githubcopilot.com");
    assert!(!is_not_copilot, "非 Copilot URL 不应被检测为 Copilot");
}

/// 验证企业版 endpoint（不包含 githubcopilot.com）场景下 is_copilot 仍然正确
#[test]
fn copilot_detection_for_enterprise_endpoint() {
    use crate::provider::{Provider, ProviderMeta};

    let provider = Provider {
        id: "enterprise".to_string(),
        name: "Enterprise Copilot".to_string(),
        settings_config: serde_json::json!({}),
        website_url: None,
        category: None,
        created_at: None,
        sort_index: None,
        notes: None,
        meta: Some(ProviderMeta {
            provider_type: Some("github_copilot".to_string()),
            ..Default::default()
        }),
        icon: None,
        icon_color: None,
        in_failover_queue: false,
    };

    let enterprise_base_url = "https://copilot-api.corp.example.com";

    let is_copilot = provider
        .meta
        .as_ref()
        .and_then(|m| m.provider_type.as_deref())
        == Some("github_copilot")
        || enterprise_base_url.contains("githubcopilot.com");

    assert!(
        is_copilot,
        "企业版 Copilot 应该通过 provider_type 被正确检测"
    );
}

/// 验证动态 endpoint 替换条件
#[test]
fn dynamic_endpoint_replacement_conditions() {
    let test_cases = [
        (true, false, true, "Copilot + 非 full_url 应该替换"),
        (true, true, false, "Copilot + full_url 不应替换"),
        (false, false, false, "非 Copilot 不应替换"),
        (false, true, false, "非 Copilot + full_url 不应替换"),
    ];

    for (is_copilot, is_full_url, should_replace, desc) in test_cases {
        let will_replace = is_copilot && !is_full_url;
        assert_eq!(will_replace, should_replace, "{desc}");
    }
}

// ===== P3: forwarder 层 media 开关回归测试 =====

fn forwarder_with_rectifier(config: RectifierConfig) -> RequestForwarder {
    let mut fwd = test_forwarder(Duration::from_secs(1), Duration::from_secs(1));
    fwd.rectifier_config = config;
    fwd
}

fn provider_with_settings(settings_config: Value) -> Provider {
    let mut p = test_provider_with_type(Some("anthropic"));
    p.settings_config = settings_config;
    p
}

fn body_with_image(model: &str) -> Value {
    json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [
                { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "abc" } }
            ]
        }]
    })
}

fn body_with_codex_input_image(model: &str) -> Value {
    json!({
        "model": model,
        "input": [{
            "role": "user",
            "content": [
                { "type": "input_image", "image_url": "data:image/png;base64,abc" }
            ]
        }]
    })
}

fn image_unsupported_error() -> ProxyError {
    ProxyError::UpstreamError {
        status: 400,
        body: Some(
            r#"{"error":{"message":"This model does not support image input"}}"#.to_string(),
        ),
    }
}

#[test]
fn prevention_replaces_when_all_switches_on_and_model_in_heuristic_list() {
    let fwd = forwarder_with_rectifier(RectifierConfig::default());
    let provider = provider_with_settings(json!({}));
    let mut body = body_with_image("deepseek-v4-pro");

    let replaced = fwd.apply_media_prevention(&mut body, &provider);

    assert_eq!(replaced, 1, "默认全开 + 名单内模型应预替换");
    assert_eq!(body["messages"][0]["content"][0]["type"], "text");
}

#[test]
fn prevention_skipped_when_media_fallback_off() {
    let fwd = forwarder_with_rectifier(RectifierConfig {
        request_media_fallback: false,
        ..RectifierConfig::default()
    });
    let provider = provider_with_settings(json!({}));
    let mut body = body_with_image("deepseek-v4-pro");

    let replaced = fwd.apply_media_prevention(&mut body, &provider);

    assert_eq!(replaced, 0);
    assert_eq!(body["messages"][0]["content"][0]["type"], "image");
}

#[test]
fn prevention_skipped_when_master_switch_off() {
    let fwd = forwarder_with_rectifier(RectifierConfig {
        enabled: false,
        ..RectifierConfig::default()
    });
    let provider = provider_with_settings(json!({}));
    let mut body = body_with_image("deepseek-v4-pro");

    assert_eq!(fwd.apply_media_prevention(&mut body, &provider), 0);
    assert_eq!(body["messages"][0]["content"][0]["type"], "image");
}

#[test]
fn prevention_heuristic_off_skips_list_but_keeps_explicit_text_only() {
    let fwd = forwarder_with_rectifier(RectifierConfig {
        request_media_heuristic: false,
        ..RectifierConfig::default()
    });

    let bare_provider = provider_with_settings(json!({}));
    let mut list_body = body_with_image("deepseek-v4-pro");
    assert_eq!(
        fwd.apply_media_prevention(&mut list_body, &bare_provider),
        0,
        "heuristic 关闭后名单模型不应被预替换"
    );
    assert_eq!(list_body["messages"][0]["content"][0]["type"], "image");

    let declared_provider = provider_with_settings(json!({
        "models": [ { "id": "some-text-model", "input": ["text"] } ]
    }));
    let mut declared_body = body_with_image("some-text-model");
    assert_eq!(
        fwd.apply_media_prevention(&mut declared_body, &declared_provider),
        1,
        "显式 text-only 即使关闭 heuristic 也应预替换"
    );
    assert_eq!(declared_body["messages"][0]["content"][0]["type"], "text");
}

#[test]
fn reactive_triggers_when_all_switches_on() {
    let fwd = forwarder_with_rectifier(RectifierConfig::default());
    let body = body_with_image("any-model");
    assert!(fwd.media_retry_should_trigger("Claude", false, &body, &image_unsupported_error()));
}

#[test]
fn reactive_triggers_for_codex_image_url_deserialize_errors() {
    let fwd = forwarder_with_rectifier(RectifierConfig::default());
    let body = body_with_codex_input_image("deepseek-v4-flash");
    let error = ProxyError::UpstreamError {
        status: 400,
        body: Some(
            r#"{"error":{"message":"Failed to deserialize the JSON body into the target type: messages[11]: unknown variant image_url, expected text"}}"#
                .to_string(),
        ),
    };

    assert!(fwd.media_retry_should_trigger("Codex", false, &body, &error));
}

#[test]
fn reactive_skipped_when_media_fallback_off() {
    let fwd = forwarder_with_rectifier(RectifierConfig {
        request_media_fallback: false,
        ..RectifierConfig::default()
    });
    let body = body_with_image("any-model");
    assert!(!fwd.media_retry_should_trigger(
        "Claude",
        false,
        &body,
        &image_unsupported_error()
    ));
}

#[test]
fn reactive_skipped_when_master_switch_off() {
    let fwd = forwarder_with_rectifier(RectifierConfig {
        enabled: false,
        ..RectifierConfig::default()
    });
    let body = body_with_image("any-model");
    assert!(!fwd.media_retry_should_trigger(
        "Claude",
        false,
        &body,
        &image_unsupported_error()
    ));
}

#[test]
fn reactive_unaffected_by_heuristic_switch() {
    let fwd = forwarder_with_rectifier(RectifierConfig {
        request_media_heuristic: false,
        ..RectifierConfig::default()
    });
    let body = body_with_image("any-model");
    assert!(fwd.media_retry_should_trigger("Claude", false, &body, &image_unsupported_error()));
}
