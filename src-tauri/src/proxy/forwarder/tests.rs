use super::*;
use crate::database::Database;
use axum::http::header::HeaderValue;
use axum::http::HeaderMap;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use crate::proxy::log_codes::fwd as log_fwd;
use crate::proxy::{
    ProviderRouter, ProxyStatus, ProxyError,
    types::{RectifierConfig, OptimizerConfig, CopilotOptimizerConfig},
    providers::{
        gemini_shadow::GeminiShadowStore,
        codex_chat_history::CodexChatHistoryStore,
    },
    failover_switch::FailoverSwitchManager,
    json_canonical::short_value_hash,
};
use super::logging::{
    build_retryable_failure_log, build_terminal_failure_log, summarize_upstream_body,
    summarize_text_for_log,
};
use super::req_headers::{
    build_codex_oauth_session_headers, reject_proxy_placeholder_for_managed_account_upstream,
    should_preserve_exact_header_case, prepare_upstream_request_body,
};
use super::url_rewrite::{
    rewrite_claude_transform_endpoint, rewrite_codex_responses_endpoint_to_chat,
};

pub(super) fn test_provider_with_type(provider_type: Option<&str>) -> Provider {
    Provider {
        id: "provider-1".to_string(),
        name: "Provider 1".to_string(),
        settings_config: json!({}),
        website_url: None,
        category: None,
        created_at: None,
        sort_index: None,
        notes: None,
        meta: provider_type.map(|value| crate::provider::ProviderMeta {
            provider_type: Some(value.to_string()),
            ..Default::default()
        }),
        icon: None,
        icon_color: None,
        in_failover_queue: false,
    }
}

pub(super) fn test_forwarder(
    non_streaming_timeout: Duration,
    streaming_first_byte_timeout: Duration,
) -> RequestForwarder {
    let db = Arc::new(Database::memory().expect("memory db"));

    RequestForwarder {
        router: Arc::new(ProviderRouter::new(db.clone())),
        status: Arc::new(RwLock::new(ProxyStatus::default())),
        current_providers: Arc::new(RwLock::new(HashMap::new())),
        gemini_shadow: Arc::new(GeminiShadowStore::new()),
        codex_chat_history: Arc::new(CodexChatHistoryStore::default()),
        failover_manager: Arc::new(FailoverSwitchManager::new(db)),
        app_handle: None,
        current_provider_id_at_start: String::new(),
        session_id: String::new(),
        session_client_provided: false,
        rectifier_config: RectifierConfig::default(),
        optimizer_config: OptimizerConfig::default(),
        copilot_optimizer_config: CopilotOptimizerConfig::default(),
        non_streaming_timeout,
        streaming_first_byte_timeout,
        max_attempts: 1,
    }
}

#[test]
fn single_provider_retryable_log_uses_single_provider_code() {
    let error = ProxyError::UpstreamError {
        status: 429,
        body: Some(r#"{"error":{"message":"rate limit exceeded"}}"#.to_string()),
    };

    let (code, message) = build_retryable_failure_log("PackyCode-response", 1, 1, &error);

    assert_eq!(code, log_fwd::SINGLE_PROVIDER_FAILED);
    assert!(message.contains("Provider PackyCode-response 请求失败"));
    assert!(message.contains("上游 HTTP 429"));
    assert!(message.contains("rate limit exceeded"));
    assert!(!message.contains("切换下一个"));
}

#[test]
fn multi_provider_retryable_log_keeps_failover_wording() {
    let error = ProxyError::Timeout("upstream timed out after 30s".to_string());

    let (code, message) = build_retryable_failure_log("primary", 1, 3, &error);

    assert_eq!(code, log_fwd::PROVIDER_FAILED_RETRY);
    assert!(message.contains("继续尝试下一个 (1/3)"));
    assert!(message.contains("请求超时"));
}

#[test]
fn single_provider_has_no_terminal_all_failed_log() {
    assert!(build_terminal_failure_log(1, 1, None).is_none());
}

#[test]
fn multi_provider_terminal_log_contains_last_error_summary() {
    let error = ProxyError::ForwardFailed("connection reset by peer".to_string());

    let (code, message) =
        build_terminal_failure_log(2, 2, Some(&error)).expect("expected terminal log");

    assert_eq!(code, log_fwd::ALL_PROVIDERS_FAILED);
    assert!(message.contains("已尝试 2/2 个 Provider，均失败"));
    assert!(message.contains("connection reset by peer"));
}

#[test]
fn summarize_upstream_body_prefers_json_message() {
    let body = json!({
        "error": {
            "message": "invalid_request_error: unsupported field"
        },
        "request_id": "req_123"
    });

    let summary = summarize_upstream_body(&body.to_string());

    assert_eq!(summary, "invalid_request_error: unsupported field");
}

#[test]
fn summarize_text_for_log_collapses_whitespace_and_truncates() {
    let summary = summarize_text_for_log("line1\n\n line2   line3", 12);

    assert_eq!(summary, "line1 line2...");
}

#[test]
fn canonical_json_sorts_object_keys_for_cache_trace_hashes() {
    let left = json!({
        "tools": [
            {
                "parameters": {
                    "properties": {
                        "b": {"type": "string"},
                        "a": {"type": "number"}
                    },
                    "type": "object"
                },
                "name": "lookup"
            }
        ]
    });
    let right = json!({
        "tools": [
            {
                "name": "lookup",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "a": {"type": "number"},
                        "b": {"type": "string"}
                    }
                }
            }
        ]
    });

    assert_eq!(
        crate::proxy::json_canonical::canonical_json_string(&left),
        crate::proxy::json_canonical::canonical_json_string(&right)
    );
    assert_eq!(
        short_value_hash(Some(&left)),
        short_value_hash(Some(&right))
    );
}

#[test]
fn prepare_upstream_request_body_filters_private_fields_and_canonicalizes_order() {
    let body = json!({
        "z": 1,
        "_internal": "drop",
        "tools": [
            {
                "name": "lookup",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "_id": {
                            "_private_note": "drop",
                            "type": "string"
                        },
                        "b": {"type": "number"},
                        "a": {"type": "string"}
                    }
                }
            }
        ],
        "a": 2
    });

    let prepared = prepare_upstream_request_body(body);

    assert!(prepared.get("_internal").is_none());
    assert!(prepared["tools"][0]["parameters"]["properties"]
        .get("_id")
        .is_some());
    assert!(prepared["tools"][0]["parameters"]["properties"]["_id"]
        .get("_private_note")
        .is_none());
}

#[test]
fn codex_oauth_session_headers_match_codex_cache_identity() {
    let headers = build_codex_oauth_session_headers("ab-12-cd");
    let mut mapped = HashMap::new();
    for (name, val) in headers {
        mapped.insert(name.to_string(), val.to_str().unwrap().to_string());
    }

    assert_eq!(mapped.get("session_id").unwrap(), "ab-12-cd");
    assert_eq!(mapped.get("x-client-request-id").unwrap(), "ab-12-cd");
    assert_eq!(mapped.get("x-codex-window-id").unwrap(), "ab-12-cd:0");
}

#[test]
fn codex_oauth_upstream_rejects_proxy_managed_placeholder_header() {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", HeaderValue::from_static("Bearer PROXY_MANAGED"));

    let err = reject_proxy_placeholder_for_managed_account_upstream(
        "https://api.githubcopilot.com/chat/completions",
        &headers,
    );
    assert!(err.is_err());
}

#[test]
fn managed_account_upstream_rejects_proxy_managed_placeholder_header() {
    let mut headers = HeaderMap::new();
    headers.insert("chatgpt-account-id", HeaderValue::from_static("PROXY_MANAGED"));

    let err = reject_proxy_placeholder_for_managed_account_upstream(
        "https://chatgpt.com/backend-api/codex/responses",
        &headers,
    );
    assert!(err.is_err());
}

#[test]
fn non_managed_upstream_allows_proxy_managed_placeholder_guard() {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", HeaderValue::from_static("Bearer PROXY_MANAGED"));

    let err = reject_proxy_placeholder_for_managed_account_upstream(
        "https://api.anthropic.com/v1/messages",
        &headers,
    );
    assert!(err.is_ok());
}

#[test]
fn exact_header_case_preserved_for_native_claude_only() {
    let bare_anthropic = test_provider_with_type(Some("anthropic"));
    assert!(should_preserve_exact_header_case(
        "Claude",
        &bare_anthropic,
        None,
        false
    ));
    assert!(should_preserve_exact_header_case(
        "Claude",
        &bare_anthropic,
        Some("anthropic"),
        false
    ));
    assert!(!should_preserve_exact_header_case(
        "Claude",
        &bare_anthropic,
        Some("gemini_native"),
        false
    ));
}

#[test]
fn exact_header_case_skipped_for_codex_oauth_and_copilot() {
    let codex_oauth = test_provider_with_type(Some("codex_oauth"));
    let copilot = test_provider_with_type(Some("github_copilot"));

    assert!(!should_preserve_exact_header_case(
        "Claude",
        &codex_oauth,
        Some("openai_responses"),
        false
    ));
    assert!(!should_preserve_exact_header_case(
        "Claude",
        &copilot,
        Some("openai_chat"),
        true
    ));
}

#[test]
fn rewrite_claude_transform_endpoint_strips_beta_for_chat_completions() {
    let (endpoint, passthrough_query) = rewrite_claude_transform_endpoint(
        "/v1/messages?beta=true&foo=bar",
        "openai_chat",
        false,
        &json!({ "model": "gpt-5.4" }),
    );

    assert_eq!(endpoint, "/v1/chat/completions?foo=bar");
    assert_eq!(passthrough_query.as_deref(), Some("foo=bar"));
}

#[test]
fn rewrite_claude_transform_endpoint_strips_beta_for_responses() {
    let (endpoint, passthrough_query) = rewrite_claude_transform_endpoint(
        "/claude/v1/messages?beta=true&x-id=1",
        "openai_responses",
        false,
        &json!({ "model": "gpt-5.4" }),
    );

    assert_eq!(endpoint, "/v1/responses?x-id=1");
    assert_eq!(passthrough_query.as_deref(), Some("x-id=1"));
}

#[test]
fn rewrite_codex_responses_endpoint_to_chat_preserves_query() {
    let (endpoint, passthrough_query) =
        rewrite_codex_responses_endpoint_to_chat("/v1/responses?foo=bar");

    assert_eq!(endpoint, "/chat/completions?foo=bar");
    assert_eq!(passthrough_query.as_deref(), Some("foo=bar"));
}

#[test]
fn rewrite_codex_responses_compact_endpoint_to_chat_preserves_query() {
    let (endpoint, passthrough_query) =
        rewrite_codex_responses_endpoint_to_chat("/v1/responses/compact?foo=bar");

    assert_eq!(endpoint, "/chat/completions?foo=bar");
    assert_eq!(passthrough_query.as_deref(), Some("foo=bar"));
}
