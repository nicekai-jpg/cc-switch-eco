use std::pin::Pin;
use std::sync::Arc;
use crate::provider::Provider;
use crate::proxy::error::ProxyError;
use serde_json::Value;
use bytes::Bytes;
use futures::Stream;
use super::gemini_shadow::GeminiShadowStore;
use super::transform_gemini::AnthropicToolSchemaHints;

pub type BoxedStream = Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>;

#[derive(Clone)]
pub struct TransformContext {
    pub provider: Provider,
    pub session_id: Option<String>,
    pub gemini_shadow: Option<Arc<GeminiShadowStore>>,
    pub tool_schema_hints: Option<AnthropicToolSchemaHints>,
}

pub trait PayloadTransformer: Send + Sync {
    fn transform_request(
        &self,
        body: Value,
        context: &TransformContext,
    ) -> Result<Value, ProxyError>;

    fn transform_response(
        &self,
        body: Value,
        context: &TransformContext,
    ) -> Result<Value, ProxyError>;

    fn transform_stream(
        &self,
        stream: BoxedStream,
        context: &TransformContext,
    ) -> BoxedStream;
}

pub struct PassthroughTransformer;

impl PayloadTransformer for PassthroughTransformer {
    fn transform_request(
        &self,
        body: Value,
        _context: &TransformContext,
    ) -> Result<Value, ProxyError> {
        Ok(body)
    }

    fn transform_response(
        &self,
        body: Value,
        _context: &TransformContext,
    ) -> Result<Value, ProxyError> {
        Ok(body)
    }

    fn transform_stream(
        &self,
        stream: BoxedStream,
        _context: &TransformContext,
    ) -> BoxedStream {
        stream
    }
}

pub struct OpenAiChatTransformer;

impl PayloadTransformer for OpenAiChatTransformer {
    fn transform_request(
        &self,
        body: Value,
        context: &TransformContext,
    ) -> Result<Value, ProxyError> {
        let preserve_reasoning_content =
            should_preserve_reasoning_content_for_openai_chat(&context.provider, &body);
        let mut result = super::transform::anthropic_to_openai_with_reasoning_content(
            body,
            preserve_reasoning_content,
        )?;
        if let Some(key) = context.provider
            .meta
            .as_ref()
            .and_then(|m| m.prompt_cache_key.as_deref())
        {
            result["prompt_cache_key"] = serde_json::json!(key);
        }
        Ok(result)
    }

    fn transform_response(
        &self,
        body: Value,
        _context: &TransformContext,
    ) -> Result<Value, ProxyError> {
        super::transform::openai_to_anthropic(body)
    }

    fn transform_stream(
        &self,
        stream: BoxedStream,
        _context: &TransformContext,
    ) -> BoxedStream {
        Box::pin(super::streaming::create_anthropic_sse_stream(stream))
    }
}

pub struct OpenAiResponsesTransformer;

impl PayloadTransformer for OpenAiResponsesTransformer {
    fn transform_request(
        &self,
        body: Value,
        context: &TransformContext,
    ) -> Result<Value, ProxyError> {
        let cache_key = get_cache_key(&context.provider, &body, context.session_id.as_deref());
        let is_codex_oauth = context.provider.is_codex_oauth();
        let codex_fast_mode = context.provider.codex_fast_mode_enabled();
        super::transform_responses::anthropic_to_responses(
            body,
            cache_key.as_deref(),
            is_codex_oauth,
            codex_fast_mode,
        )
    }

    fn transform_response(
        &self,
        body: Value,
        _context: &TransformContext,
    ) -> Result<Value, ProxyError> {
        super::transform_responses::responses_to_anthropic(body)
    }

    fn transform_stream(
        &self,
        stream: BoxedStream,
        _context: &TransformContext,
    ) -> BoxedStream {
        Box::pin(super::streaming_responses::create_anthropic_sse_stream_from_responses(stream))
    }
}

pub struct GeminiNativeTransformer;

impl PayloadTransformer for GeminiNativeTransformer {
    fn transform_request(
        &self,
        body: Value,
        context: &TransformContext,
    ) -> Result<Value, ProxyError> {
        super::transform_gemini::anthropic_to_gemini_with_shadow(
            body,
            context.gemini_shadow.as_deref(),
            Some(&context.provider.id),
            context.session_id.as_deref(),
        )
    }

    fn transform_response(
        &self,
        body: Value,
        context: &TransformContext,
    ) -> Result<Value, ProxyError> {
        super::transform_gemini::gemini_to_anthropic_with_shadow_and_hints(
            body,
            context.gemini_shadow.as_deref(),
            Some(&context.provider.id),
            context.session_id.as_deref(),
            context.tool_schema_hints.as_ref(),
        )
    }

    fn transform_stream(
        &self,
        stream: BoxedStream,
        context: &TransformContext,
    ) -> BoxedStream {
        Box::pin(super::streaming_gemini::create_anthropic_sse_stream_from_gemini(
            stream,
            context.gemini_shadow.clone(),
            Some(context.provider.id.clone()),
            context.session_id.clone(),
            context.tool_schema_hints.clone(),
        ))
    }
}

pub fn get_transformer(api_format: &str) -> Box<dyn PayloadTransformer> {
    match api_format {
        "openai_chat" => Box::new(OpenAiChatTransformer),
        "openai_responses" => Box::new(OpenAiResponsesTransformer),
        "gemini_native" => Box::new(GeminiNativeTransformer),
        _ => Box::new(PassthroughTransformer),
    }
}

// --- Helper functions for OpenAI chat / responses ---

fn is_reasoning_content_compatible_identifier(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("moonshot")
        || value.contains("kimi")
        || value.contains("deepseek")
        || value.contains("mimo")
        || value.contains("xiaomimimo")
}

fn should_preserve_reasoning_content_for_openai_chat(
    provider: &Provider,
    body: &Value,
) -> bool {
    if body
        .get("model")
        .and_then(|m| m.as_str())
        .is_some_and(is_reasoning_content_compatible_identifier)
    {
        return true;
    }

    let settings = &provider.settings_config;
    let base_urls = [
        settings
            .get("env")
            .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
            .and_then(|v| v.as_str()),
        settings.get("base_url").and_then(|v| v.as_str()),
        settings.get("baseURL").and_then(|v| v.as_str()),
        settings.get("apiEndpoint").and_then(|v| v.as_str()),
    ];

    base_urls
        .into_iter()
        .flatten()
        .any(is_reasoning_content_compatible_identifier)
}

fn get_cache_key(
    provider: &Provider,
    body: &Value,
    session_id: Option<&str>,
) -> Option<String> {
    let is_copilot = provider
        .meta
        .as_ref()
        .and_then(|m| m.provider_type.as_deref())
        == Some("github_copilot")
        || provider
            .settings_config
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .is_some_and(|u| u.contains("githubcopilot.com"));
    let session_cache_key: Option<String> = if is_copilot {
        let metadata = body.get("metadata");
        metadata
            .and_then(|m| m.get("user_id"))
            .and_then(|v| v.as_str())
            .and_then(crate::proxy::session::parse_session_from_user_id)
            .or_else(|| {
                metadata
                    .and_then(|m| m.get("session_id"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            })
    } else {
        session_id
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    };

    let explicit_cache_key = provider
        .meta
        .as_ref()
        .and_then(|m| m.prompt_cache_key.as_deref());
    if let Some(key) = explicit_cache_key {
        Some(key.to_string())
    } else {
        session_cache_key
    }
}
