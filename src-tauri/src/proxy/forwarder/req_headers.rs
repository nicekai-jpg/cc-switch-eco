use serde_json::Value;
use crate::provider::Provider;
use crate::proxy::ProxyError;
use crate::proxy::body_filter::filter_private_params_with_whitelist;
use crate::proxy::json_canonical::canonicalize_value;

const PROXY_AUTH_PLACEHOLDER: &str = "PROXY_MANAGED";

pub(super) fn build_codex_oauth_session_headers(
    session_id: &str,
) -> Vec<(http::HeaderName, http::HeaderValue)> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Vec::new();
    }

    let mut headers = Vec::new();
    if let Ok(value) = http::HeaderValue::from_str(session_id) {
        headers.push((http::HeaderName::from_static("session_id"), value.clone()));
        headers.push((http::HeaderName::from_static("x-client-request-id"), value));
    }

    let window_id = format!("{session_id}:0");
    if let Ok(value) = http::HeaderValue::from_str(&window_id) {
        headers.push((http::HeaderName::from_static("x-codex-window-id"), value));
    }

    headers
}

pub(super) fn reject_proxy_placeholder_for_managed_account_upstream(
    url: &str,
    headers: &http::HeaderMap,
) -> Result<(), ProxyError> {
    if !is_managed_account_upstream_url(url) || !headers_contain_proxy_placeholder(headers) {
        return Ok(());
    }

    Err(ProxyError::AuthError(
        "Managed account proxy auth was not resolved; PROXY_MANAGED must not be sent upstream"
            .to_string(),
    ))
}

pub(super) fn is_managed_account_upstream_url(url: &str) -> bool {
    let Ok(uri) = url.parse::<http::Uri>() else {
        return false;
    };

    let Some(host) = uri.host().map(str::to_ascii_lowercase) else {
        return false;
    };

    host == "githubcopilot.com"
        || host.ends_with(".githubcopilot.com")
        || (host == "chatgpt.com" && uri.path().starts_with("/backend-api/codex"))
}

pub(super) fn headers_contain_proxy_placeholder(headers: &http::HeaderMap) -> bool {
    headers.values().any(|value| {
        value
            .to_str()
            .map(|value| value.contains(PROXY_AUTH_PLACEHOLDER))
            .unwrap_or(false)
    })
}

pub(super) fn should_preserve_exact_header_case(
    adapter_name: &str,
    provider: &Provider,
    resolved_claude_api_format: Option<&str>,
    is_copilot: bool,
) -> bool {
    if matches!(adapter_name, "Codex" | "Gemini") {
        return false;
    }

    if is_copilot || provider.is_codex_oauth() {
        return false;
    }

    matches!(resolved_claude_api_format, None | Some("anthropic"))
}

pub(super) fn is_streaming_request(endpoint: &str, body: &Value, headers: &axum::http::HeaderMap) -> bool {
    if body
        .get("stream")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        return true;
    }

    if endpoint.contains("streamGenerateContent") || endpoint.contains("alt=sse") {
        return true;
    }

    headers
        .get(axum::http::header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|accept| accept.contains("text/event-stream"))
        .unwrap_or(false)
}

#[cfg(test)]
pub(super) fn should_force_identity_encoding(
    endpoint: &str,
    body: &Value,
    headers: &axum::http::HeaderMap,
) -> bool {
    is_streaming_request(endpoint, body, headers)
}

pub(super) fn map_reqwest_send_error(error: reqwest::Error) -> ProxyError {
    if error.is_timeout() {
        ProxyError::Timeout(format!("请求超时: {error}"))
    } else if error.is_connect() {
        ProxyError::ForwardFailed(format!("连接失败: {error}"))
    } else {
        ProxyError::ForwardFailed(error.to_string())
    }
}

pub(super) fn prepare_upstream_request_body(request_body: Value) -> Value {
    canonicalize_value(filter_private_params_with_whitelist(request_body, &[]))
}

pub(super) fn prepare_request_headers(
    headers: &axum::http::HeaderMap,
    is_copilot: bool,
    should_send_anthropic_headers: bool,
    force_identity_encoding: bool,
    custom_user_agent: &Option<http::HeaderValue>,
    anthropic_beta_value: &Option<String>,
    auth_headers: &[(http::HeaderName, http::HeaderValue)],
    codex_oauth_session_headers: &[(http::HeaderName, http::HeaderValue)],
    copilot_optimization: &Option<(
        crate::proxy::copilot_optimizer::CopilotClassification,
        Option<String>,
        Option<String>,
    )>,
    copilot_optimizer_config: &crate::proxy::types::CopilotOptimizerConfig,
    upstream_host: &Option<String>,
) -> http::HeaderMap {
    let mut auth_headers = auth_headers.to_vec();

    // --- Copilot 优化器：动态 header 注入 ---
    if let Some((ref classification, ref det_request_id, ref interaction_id)) =
        copilot_optimization
    {
        for (name, value) in auth_headers.iter_mut() {
            match name.as_str() {
                "x-initiator" if copilot_optimizer_config.request_classification => {
                    *value = http::HeaderValue::from_static(classification.initiator);
                }
                "x-interaction-type" if classification.is_subagent => {
                    // 子代理请求：conversation-subagent 不计 premium interaction
                    *value = http::HeaderValue::from_static("conversation-subagent");
                }
                "x-request-id" | "x-agent-task-id" => {
                    if let Some(ref det_id) = det_request_id {
                        if let Ok(hv) = http::HeaderValue::from_str(det_id) {
                            *value = hv;
                        }
                    }
                }
                _ => {}
            }
        }

        // x-interaction-id：仅在有 session 时注入（不在 get_auth_headers 中）
        if let Some(ref iid) = interaction_id {
            if let Ok(hv) = http::HeaderValue::from_str(iid) {
                auth_headers.push((http::HeaderName::from_static("x-interaction-id"), hv));
            }
        }

        if classification.is_subagent {
            log::info!(
                "[Copilot] 子代理请求: x-initiator=agent, x-interaction-type=conversation-subagent"
            );
        }
    }

    // Copilot 指纹头名（由 get_auth_headers 注入，需在原始头中去重）
    let copilot_fingerprint_headers: &[&str] = if is_copilot {
        &[
            "user-agent",
            "editor-version",
            "editor-plugin-version",
            "copilot-integration-id",
            "x-github-api-version",
            "openai-intent",
            "x-initiator",
            "x-interaction-type",
            "x-interaction-id",
            "x-vscode-user-agent-library-version",
            "x-request-id",
            "x-agent-task-id",
        ]
    } else {
        &[]
    };

    let mut ordered_headers = http::HeaderMap::new();
    let mut saw_auth = false;
    let mut saw_accept_encoding = false;
    let mut saw_user_agent = false;
    let mut saw_anthropic_beta = false;
    let mut saw_anthropic_version = false;

    for (key, value) in headers {
        let key_str = key.as_str();

        // --- host — 原位替换为上游 host（保持客户端原始位置） ---
        if key_str.eq_ignore_ascii_case("host") {
            if let Some(ref host_val) = upstream_host {
                if let Ok(hv) = http::HeaderValue::from_str(host_val) {
                    ordered_headers.append(key.clone(), hv);
                }
            }
            continue;
        }

        // --- 连接 / 追踪 / CDN 类 — 无条件跳过 ---
        if matches!(
            key_str,
            "content-length"
                | "transfer-encoding"
                | "x-forwarded-host"
                | "x-forwarded-port"
                | "x-forwarded-proto"
                | "forwarded"
                | "cf-connecting-ip"
                | "cf-ipcountry"
                | "cf-ray"
                | "cf-visitor"
                | "true-client-ip"
                | "fastly-client-ip"
                | "x-azure-clientip"
                | "x-azure-fdid"
                | "x-azure-ref"
                | "akamai-origin-hop"
                | "x-akamai-config-log-detail"
                | "x-request-id"
                | "x-correlation-id"
                | "x-trace-id"
                | "x-amzn-trace-id"
                | "x-b3-traceid"
                | "x-b3-spanid"
                | "x-b3-parentspanid"
                | "x-b3-sampled"
                | "traceparent"
                | "tracestate"
        ) {
            continue;
        }

        // --- 认证类 — 用 adapter 提供的认证头替换（在原始位置） ---
        if key_str.eq_ignore_ascii_case("authorization")
            || key_str.eq_ignore_ascii_case("x-api-key")
            || key_str.eq_ignore_ascii_case("x-goog-api-key")
        {
            if !saw_auth {
                saw_auth = true;
                for (ah_name, ah_value) in &auth_headers {
                    ordered_headers.append(ah_name.clone(), ah_value.clone());
                }
            }
            continue;
        }

        // --- accept-encoding — transform / SSE 路径强制 identity，其余保留原值 ---
        if key_str.eq_ignore_ascii_case("accept-encoding") {
            if !saw_accept_encoding {
                saw_accept_encoding = true;
                if force_identity_encoding {
                    ordered_headers.append(
                        http::header::ACCEPT_ENCODING,
                        http::HeaderValue::from_static("identity"),
                    );
                } else {
                    ordered_headers.append(key.clone(), value.clone());
                }
            }
            continue;
        }

        // --- user-agent: provider-level override for local proxy routing ---
        if !is_copilot && key_str.eq_ignore_ascii_case("user-agent") {
            if !saw_user_agent {
                saw_user_agent = true;
                if let Some(ref ua) = custom_user_agent {
                    ordered_headers.append(http::header::USER_AGENT, ua.clone());
                } else {
                    ordered_headers.append(key.clone(), value.clone());
                }
            }
            continue;
        }

        // --- anthropic-beta — 用重建值替换（确保含 claude-code 标记） ---
        if key_str.eq_ignore_ascii_case("anthropic-beta") {
            if !saw_anthropic_beta {
                saw_anthropic_beta = true;
                if let Some(ref beta_val) = anthropic_beta_value {
                    if let Ok(hv) = http::HeaderValue::from_str(beta_val) {
                        ordered_headers.append("anthropic-beta", hv);
                    }
                }
            }
            continue;
        }

        // --- anthropic-version — 透传客户端值 ---
        if key_str.eq_ignore_ascii_case("anthropic-version") {
            if should_send_anthropic_headers {
                saw_anthropic_version = true;
                ordered_headers.append(key.clone(), value.clone());
            }
            continue;
        }

        // --- Copilot 指纹头 — 跳过（由 auth_headers 提供） ---
        if copilot_fingerprint_headers
            .iter()
            .any(|h| key_str.eq_ignore_ascii_case(h))
        {
            continue;
        }

        // --- 默认：透传 ---
        ordered_headers.append(key.clone(), value.clone());
    }

    // 如果原始请求中没有认证头，在末尾追加
    if !saw_auth && !auth_headers.is_empty() {
        for (ah_name, ah_value) in &auth_headers {
            ordered_headers.append(ah_name.clone(), ah_value.clone());
        }
    }

    // transform / SSE 路径在缺失时补 identity；普通透传不主动补 accept-encoding
    if !saw_accept_encoding && force_identity_encoding {
        ordered_headers.append(
            http::header::ACCEPT_ENCODING,
            http::HeaderValue::from_static("identity"),
        );
    }

    if !saw_user_agent {
        if let Some(ref ua) = custom_user_agent {
            ordered_headers.append(http::header::USER_AGENT, ua.clone());
        }
    }

    // 如果原始请求中没有 anthropic-beta 且有值需要添加，追加
    if !saw_anthropic_beta {
        if let Some(ref beta_val) = anthropic_beta_value {
            if let Ok(hv) = http::HeaderValue::from_str(beta_val) {
                ordered_headers.append("anthropic-beta", hv);
            }
        }
    }

    // anthropic-version：仅在缺失时补充默认值
    if should_send_anthropic_headers && !saw_anthropic_version {
        ordered_headers.append(
            "anthropic-version",
            http::HeaderValue::from_static("2023-06-01"),
        );
    }

    // Codex OAuth 反代尽量对齐官方 Codex CLI 的会话路由信号。
    // 只发送客户端提供的 session_id；生成的 UUID 每次不同，反而会破坏前缀缓存。
    for (name, value) in codex_oauth_session_headers {
        ordered_headers.insert(name, value.clone());
    }

    ordered_headers
}
