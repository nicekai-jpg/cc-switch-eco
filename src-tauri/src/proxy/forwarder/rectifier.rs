use super::{RequestForwarder, RetryOutcome, ForwardError, ForwardResult};
use crate::app_config::AppType;
use crate::provider::Provider;
use crate::proxy::{
    error::ProxyError,
    providers::{ProviderAdapter, ProviderType},
};
use serde_json::Value;
use http::Extensions;

impl RequestForwarder {
    pub(super) async fn try_rectifier_retries(
        &self,
        app_type: &AppType,
        method: &http::Method,
        provider: &Provider,
        endpoint: &str,
        provider_body: &mut Value,
        headers: &axum::http::HeaderMap,
        extensions: &Extensions,
        adapter: &dyn ProviderAdapter,
        error: ProxyError,
        rectifier_retried: &mut bool,
        budget_rectifier_retried: &mut bool,
        used_half_open_permit: bool,
        last_error: &mut Option<ProxyError>,
        last_provider: &mut Option<Provider>,
        signature_rectifier_non_retryable_client_error: &mut bool,
    ) -> Result<RetryOutcome, ForwardError> {
        let app_type_str = app_type.as_str();
        let provider_type = ProviderType::from_app_type_and_config(app_type, provider);
        let is_anthropic_provider = matches!(
            provider_type,
            ProviderType::Claude | ProviderType::ClaudeAuth
        );

        if is_anthropic_provider {
            let error_message = super::logging::extract_error_message(&error);

            // thinking signature check
            if crate::proxy::thinking_rectifier::should_rectify_thinking_signature(
                error_message.as_deref(),
                &self.rectifier_config,
            ) {
                if *rectifier_retried {
                    log::warn!("[{app_type_str}] [RECT-005] 整流器已触发过，不再重试");
                    self.router
                        .release_permit_neutral(
                            &provider.id,
                            app_type_str,
                            used_half_open_permit,
                        )
                        .await;
                    let mut status = self.status.write().await;
                    status.failed_requests += 1;
                    status.last_error = Some(error.to_string());
                    if status.total_requests > 0 {
                        status.success_rate = (status.success_requests as f32
                            / status.total_requests as f32)
                            * 100.0;
                    }
                    return Err(ForwardError {
                        error,
                        provider: Some(provider.clone()),
                    });
                }

                let rectified = crate::proxy::thinking_rectifier::rectify_anthropic_request(provider_body);

                if !rectified.applied {
                    log::warn!(
                        "[{app_type_str}] [RECT-006] thinking 签名整流器触发但无可整流内容，继续检查 budget；若 budget 也未命中则按客户端错误返回"
                    );
                    *signature_rectifier_non_retryable_client_error = true;
                } else {
                    log::info!(
                        "[{}] [RECT-001] thinking 签名整流器触发, 移除 {} thinking blocks, {} redacted_thinking blocks, {} signature fields",
                        app_type_str,
                        rectified.removed_thinking_blocks,
                        rectified.removed_redacted_thinking_blocks,
                        rectified.removed_signature_fields
                    );

                    *rectifier_retried = true;

                    match self
                        .forward(
                            app_type,
                            method,
                            provider,
                            endpoint,
                            provider_body,
                            headers,
                            extensions,
                            adapter,
                        )
                        .await
                    {
                        Ok((response, claude_api_format, outbound_model)) => {
                            log::info!("[{app_type_str}] [RECT-002] 整流重试成功");
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
                                    status.success_rate = (status.success_requests
                                        as f32
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
                                "[{app_type_str}] [RECT-003] 整流重试仍失败: {retry_err}"
                            );
                            if let Some(err) = self
                                .handle_rectifier_retry_failure(
                                    retry_err,
                                    provider,
                                    app_type_str,
                                    used_half_open_permit,
                                    "整流",
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

            // budget rectifier check
            let error_message = super::logging::extract_error_message(&error);
            if crate::proxy::thinking_budget_rectifier::should_rectify_thinking_budget(
                error_message.as_deref(),
                &self.rectifier_config,
            ) {
                if *budget_rectifier_retried {
                    log::warn!(
                        "[{app_type_str}] [RECT-013] budget 整流器已触发过，不再重试"
                    );
                    self.router
                        .release_permit_neutral(
                            &provider.id,
                            app_type_str,
                            used_half_open_permit,
                        )
                        .await;
                    let mut status = self.status.write().await;
                    status.failed_requests += 1;
                    status.last_error = Some(error.to_string());
                    if status.total_requests > 0 {
                        status.success_rate = (status.success_requests as f32
                            / status.total_requests as f32)
                            * 100.0;
                    }
                    return Err(ForwardError {
                        error,
                        provider: Some(provider.clone()),
                    });
                }

                let budget_rectified = crate::proxy::thinking_budget_rectifier::rectify_thinking_budget(provider_body);
                if !budget_rectified.applied {
                    log::warn!(
                        "[{app_type_str}] [RECT-014] budget 整流器触发但无可整流内容，不做无意义重试"
                    );
                    self.router
                        .release_permit_neutral(
                            &provider.id,
                            app_type_str,
                            used_half_open_permit,
                        )
                        .await;
                    let mut status = self.status.write().await;
                    status.failed_requests += 1;
                    status.last_error = Some(error.to_string());
                    if status.total_requests > 0 {
                        status.success_rate = (status.success_requests as f32
                            / status.total_requests as f32)
                            * 100.0;
                    }
                    return Err(ForwardError {
                        error,
                        provider: Some(provider.clone()),
                    });
                }

                log::info!(
                    "[{}] [RECT-010] thinking budget 整流器触发, before={:?}, after={:?}",
                    app_type_str,
                    budget_rectified.before,
                    budget_rectified.after
                );

                *budget_rectifier_retried = true;

                match self
                    .forward(
                        app_type,
                        method,
                        provider,
                        endpoint,
                        provider_body,
                        headers,
                        extensions,
                        adapter,
                    )
                    .await
                {
                    Ok((response, claude_api_format, outbound_model)) => {
                        log::info!("[{app_type_str}] [RECT-011] budget 整流重试成功");
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
                            "[{app_type_str}] [RECT-012] budget 整流重试仍失败: {retry_err}"
                        );
                        if let Some(err) = self
                            .handle_rectifier_retry_failure(
                                retry_err,
                                provider,
                                app_type_str,
                                used_half_open_permit,
                                "budget 整流",
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
