use super::{RequestForwarder, RetryOutcome, ForwardError, ForwardResult};
use crate::app_config::AppType;
use crate::provider::Provider;
use crate::proxy::{
    error::ProxyError,
    providers::ProviderAdapter,
};
use serde_json::Value;
use http::Extensions;

impl RequestForwarder {
    /// 预防式 media 降级：发送前对 text-only 模型把图片块替换为标记。
    ///
    /// 受 `enabled && request_media_fallback` 管辖；其中"启发式模型名单预测"
    /// 再受 `request_media_heuristic` 单独管辖（显式声明 text-only 始终生效）。
    /// 返回被替换的图片块数量（0 = 未触发或开关关闭）。
    pub(super) fn apply_media_prevention(&self, body: &mut Value, provider: &Provider) -> usize {
        if !(self.rectifier_config.enabled && self.rectifier_config.request_media_fallback) {
            return 0;
        }
        let replaced_images = crate::proxy::media_sanitizer::replace_images_for_text_only_model(
            body,
            provider,
            self.rectifier_config.request_media_heuristic,
        );
        if replaced_images > 0 {
            let model = body.get("model").and_then(Value::as_str).unwrap_or("");
            log::info!(
                "[Media] Replaced {replaced_images} image block(s) with {} for text-only provider={}, model={}",
                crate::proxy::media_sanitizer::UNSUPPORTED_IMAGE_MARKER,
                provider.id,
                model
            );
        }
        replaced_images
    }

    /// 反应式 media 重试判定：上游因图片输入报错后，是否应替换图片块并对同一供应商重试一次。
    ///
    /// 受 `enabled && request_media_fallback` 管辖；不涉及 `request_media_heuristic`——
    /// 这里是上游"实测"错误后的纯恢复，不是预测，故启发式开关与它无关。
    pub(super) fn media_retry_should_trigger(
        &self,
        adapter_name: &str,
        already_retried: bool,
        provider_body: &Value,
        error: &ProxyError,
    ) -> bool {
        matches!(adapter_name, "Claude" | "Codex")
            && self.rectifier_config.enabled
            && self.rectifier_config.request_media_fallback
            && !already_retried
            && crate::proxy::media_sanitizer::contains_image_blocks(provider_body)
            && crate::proxy::media_sanitizer::is_unsupported_image_error(error)
    }

    pub(super) async fn try_media_retry(
        &self,
        app_type: &AppType,
        method: &http::Method,
        provider: &Provider,
        endpoint: &str,
        provider_body: &Value,
        headers: &axum::http::HeaderMap,
        extensions: &Extensions,
        adapter: &dyn ProviderAdapter,
        error: ProxyError,
        media_rectifier_retried: &mut bool,
        used_half_open_permit: bool,
        last_error: &mut Option<ProxyError>,
        last_provider: &mut Option<Provider>,
    ) -> Result<RetryOutcome, ForwardError> {
        let app_type_str = app_type.as_str();
        if self.media_retry_should_trigger(
            adapter.name(),
            *media_rectifier_retried,
            provider_body,
            &error,
        ) {
            let mut media_body = provider_body.clone();
            let replaced_images =
                crate::proxy::media_sanitizer::replace_image_blocks_with_marker(
                    &mut media_body,
                );

            if replaced_images > 0 {
                *media_rectifier_retried = true;
                let model = media_body
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                log::info!(
                    "[{app_type_str}] [Media] Upstream rejected image input; retrying provider={} model={} with {replaced_images} image block(s) replaced by {}",
                    provider.id,
                    model,
                    crate::proxy::media_sanitizer::UNSUPPORTED_IMAGE_MARKER
                );

                match self
                    .forward(
                        app_type,
                        method,
                        provider,
                        endpoint,
                        &media_body,
                        headers,
                        extensions,
                        adapter,
                    )
                    .await
                {
                    Ok((response, claude_api_format, outbound_model)) => {
                        log::info!(
                            "[{app_type_str}] [Media] Unsupported-image retry succeeded"
                        );
                        self.record_success_result(
                            &provider.id,
                            app_type_str,
                            used_half_open_permit,
                        )
                        .await;

                        {
                            let mut current_providers =
                                self.current_providers.write().await;
                            current_providers.insert(
                                app_type_str.to_string(),
                                (provider.id.clone(), provider.name.clone()),
                            );
                        }

                        {
                            let mut status = self.status.write().await;
                            status.success_requests += 1;
                            status.last_error = None;
                            let should_switch =
                                self.current_provider_id_at_start.as_str()
                                    != provider.id.as_str();
                            if should_switch {
                                status.failover_count += 1;
                                let fm = self.failover_manager.clone();
                                  let ah = self.app_handle.clone();
                                let pid = provider.id.clone();
                                let pname = provider.name.clone();
                                let at = app_type_str.to_string();

                                tokio::spawn(async move {
                                    let _ = fm
                                        .try_switch(ah.as_ref(), &at, &pid, &pname)
                                        .await;
                                });
                            }
                            if status.total_requests > 0 {
                                status.success_rate = (status.success_requests as f32
                                    / status.total_requests as f32)
                                    * 100.0;
                            }
                        }

                        return Ok(RetryOutcome::Success(ForwardResult {
                            response,
                            provider: provider.clone(),
                            claude_api_format,
                            outbound_model,
                            connection_guard: None,
                        }));
                    }
                    Err(retry_err) => {
                        log::warn!(
                            "[{app_type_str}] [Media] Unsupported-image retry still failed: {retry_err}"
                        );
                        if let Some(err) = self
                            .handle_rectifier_retry_failure(
                                retry_err,
                                provider,
                                app_type_str,
                                used_half_open_permit,
                                "media 降级",
                                last_error,
                                last_provider,
                            )
                            .await
                        {
                            return Err(err);
                        }
                        return Ok(RetryOutcome::NextProvider);
                    }
                }
            }
        }
        Ok(RetryOutcome::NoRetry(error))
    }
}
