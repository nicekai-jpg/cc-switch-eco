use super::{RequestForwarder, ForwardResult, ForwardError};
use crate::app_config::AppType;
use crate::provider::Provider;
use crate::proxy::{
    error::{ProxyError, ErrorCategory},
    providers::get_adapter,
};
use serde_json::Value;
use http::Extensions;

pub enum RetryOutcome {
    Success(ForwardResult),
    NextProvider,
    NoRetry(ProxyError),
}

impl RequestForwarder {
    pub(super) async fn record_success_result(
        &self,
        provider_id: &str,
        app_type: &str,
        used_half_open_permit: bool,
    ) {
        if used_half_open_permit {
            if let Err(e) = self
                .router
                .record_result(provider_id, app_type, true, true, None)
                .await
            {
                log::warn!(
                    "[{app_type}] 记录 Provider 成功结果失败: provider_id={provider_id}, error={e}"
                );
            }
            return;
        }

        let router = self.router.clone();
        let provider_id = provider_id.to_string();
        let app_type = app_type.to_string();
        tokio::spawn(async move {
            if let Err(e) = router
                .record_result(&provider_id, &app_type, false, true, None)
                .await
            {
                log::warn!(
                    "[{app_type}] 异步记录 Provider 成功结果失败: provider_id={provider_id}, error={e}"
                );
            }
        });
    }

    /// 整流（thinking signature 或 budget）重试失败后的统一收尾。
    pub(super) async fn handle_rectifier_retry_failure(
        &self,
        retry_err: ProxyError,
        provider: &Provider,
        app_type_str: &str,
        used_half_open_permit: bool,
        rectifier_label: &str,
        last_error: &mut Option<ProxyError>,
        last_provider: &mut Option<Provider>,
    ) -> Option<ForwardError> {
        let is_provider_error = match &retry_err {
            ProxyError::Timeout(_) | ProxyError::ForwardFailed(_) => true,
            ProxyError::UpstreamError { status, .. } => *status >= 500,
            _ => false,
        };

        if is_provider_error {
            let _ = self
                .router
                .record_result(
                    &provider.id,
                    app_type_str,
                    used_half_open_permit,
                    false,
                    Some(retry_err.to_string()),
                )
                .await;
            {
                let mut status = self.status.write().await;
                status.last_error = Some(format!(
                    "Provider {} {rectifier_label}重试失败: {}",
                    provider.name, retry_err
                ));
            }
            *last_error = Some(retry_err);
            *last_provider = Some(provider.clone());
            return None;
        }

        self.router
            .release_permit_neutral(&provider.id, app_type_str, used_half_open_permit)
            .await;
        let mut status = self.status.write().await;
        status.failed_requests += 1;
        status.last_error = Some(retry_err.to_string());
        if status.total_requests > 0 {
            status.success_rate =
                (status.success_requests as f32 / status.total_requests as f32) * 100.0;
        }
        Some(ForwardError {
            error: retry_err,
            provider: Some(provider.clone()),
        })
    }

    /// 转发请求（带故障转移）
    pub async fn forward_with_retry(
        &self,
        app_type: &AppType,
        method: http::Method,
        endpoint: &str,
        body: Value,
        headers: axum::http::HeaderMap,
        extensions: Extensions,
        providers: Vec<Provider>,
    ) -> Result<ForwardResult, ForwardError> {
        let guard = super::ActiveConnectionGuard::acquire(self.status.clone()).await;
        {
            let mut s = self.status.write().await;
            s.total_requests = s.total_requests.saturating_add(1);
            s.last_request_at = Some(chrono::Utc::now().to_rfc3339());
        }
        let result = self
            .forward_with_retry_inner(
                app_type, method, endpoint, body, headers, extensions, providers,
            )
            .await;
        result.map(|mut fr| {
            fr.connection_guard = Some(guard);
            fr
        })
    }

    /// 实际转发逻辑（不包含客户端维度的入口/出口计数）
    async fn forward_with_retry_inner(
        &self,
        app_type: &AppType,
        method: http::Method,
        endpoint: &str,
        body: Value,
        headers: axum::http::HeaderMap,
        extensions: Extensions,
        providers: Vec<Provider>,
    ) -> Result<ForwardResult, ForwardError> {
        let adapter = get_adapter(app_type);
        let app_type_str = app_type.as_str();

        if providers.is_empty() {
            return Err(ForwardError {
                error: ProxyError::NoAvailableProvider,
                provider: None,
            });
        }

        let mut last_error = None;
        let mut last_provider = None;
        let mut attempted_providers = 0usize;

        let bypass_circuit_breaker = providers.len() == 1;

        for provider in providers.iter() {
            let mut rectifier_retried = false;
            let mut budget_rectifier_retried = false;
            let mut media_rectifier_retried = false;

            if attempted_providers >= self.max_attempts {
                log::warn!(
                    "[{app_type_str}] 已达最大尝试次数上限 ({}/{}), 停止故障转移",
                    attempted_providers,
                    self.max_attempts
                );
                break;
            }

            let (allowed, used_half_open_permit) = if bypass_circuit_breaker {
                (true, false)
            } else {
                let permit = self
                    .router
                    .allow_provider_request(&provider.id, app_type_str)
                    .await;
                (permit.allowed, permit.used_half_open_permit)
            };

            if !allowed {
                continue;
            }

            let mut provider_body =
                if self.optimizer_config.enabled && super::url_rewrite::is_bedrock_provider(provider) {
                    let mut b = body.clone();
                    if self.optimizer_config.thinking_optimizer {
                        crate::proxy::thinking_optimizer::optimize(&mut b, &self.optimizer_config);
                    }
                    if self.optimizer_config.cache_injection {
                        crate::proxy::cache_injector::inject(&mut b, &self.optimizer_config);
                    }
                    b
                } else {
                    body.clone()
                };

            attempted_providers += 1;

            {
                let mut status = self.status.write().await;
                status.current_provider = Some(provider.name.clone());
                status.current_provider_id = Some(provider.id.clone());
            }

            match self
                .forward(
                    app_type,
                    &method,
                    provider,
                    endpoint,
                    &provider_body,
                    &headers,
                    &extensions,
                    adapter.as_ref(),
                )
                .await
            {
                Ok((response, claude_api_format, outbound_model)) => {
                    self.record_success_result(&provider.id, app_type_str, used_half_open_permit)
                        .await;

                    {
                        let mut current_providers = self.current_providers.write().await;
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
                            self.current_provider_id_at_start.as_str() != provider.id.as_str();
                        if should_switch {
                            status.failover_count += 1;

                            let fm = self.failover_manager.clone();
                            let ah = self.app_handle.clone();
                            let pid = provider.id.clone();
                            let pname = provider.name.clone();
                            let at = app_type_str.to_string();

                            tokio::spawn(async move {
                                let _ = fm.try_switch(ah.as_ref(), &at, &pid, &pname).await;
                            });
                        }
                        if status.total_requests > 0 {
                            status.success_rate = (status.success_requests as f32
                                / status.total_requests as f32)
                                * 100.0;
                        }
                    }

                    return Ok(ForwardResult {
                        response,
                        provider: provider.clone(),
                        claude_api_format,
                        outbound_model,
                        connection_guard: None,
                    });
                }
                Err(e) => {
                    // try media retry
                    match self.try_media_retry(
                        app_type,
                        &method,
                        provider,
                        endpoint,
                        &provider_body,
                        &headers,
                        &extensions,
                        adapter.as_ref(),
                        e,
                        &mut media_rectifier_retried,
                        used_half_open_permit,
                        &mut last_error,
                        &mut last_provider,
                    ).await {
                        Ok(RetryOutcome::Success(res)) => return Ok(res),
                        Ok(RetryOutcome::NextProvider) => continue,
                        Ok(RetryOutcome::NoRetry(e)) => {
                            let mut signature_rectifier_non_retryable_client_error = false;
                            match self.try_rectifier_retries(
                                app_type,
                                &method,
                                provider,
                                endpoint,
                                &mut provider_body,
                                &headers,
                                &extensions,
                                adapter.as_ref(),
                                e,
                                &mut rectifier_retried,
                                &mut budget_rectifier_retried,
                                used_half_open_permit,
                                &mut last_error,
                                &mut last_provider,
                                &mut signature_rectifier_non_retryable_client_error,
                            ).await {
                                Ok(RetryOutcome::Success(res)) => return Ok(res),
                                Ok(RetryOutcome::NextProvider) => continue,
                                Ok(RetryOutcome::NoRetry(e)) => {
                                    if signature_rectifier_non_retryable_client_error {
                                        self.router
                                            .release_permit_neutral(
                                                &provider.id,
                                                app_type_str,
                                                used_half_open_permit,
                                            )
                                            .await;
                                        let mut status = self.status.write().await;
                                        status.failed_requests += 1;
                                        status.last_error = Some(e.to_string());
                                        if status.total_requests > 0 {
                                            status.success_rate = (status.success_requests as f32
                                                / status.total_requests as f32)
                                                * 100.0;
                                        }
                                        return Err(ForwardError {
                                            error: e,
                                            provider: Some(provider.clone()),
                                        });
                                    }

                                    let category = self.categorize_proxy_error(&e);

                                    match category {
                                        ErrorCategory::Retryable => {
                                            let _ = self
                                                .router
                                                .record_result(
                                                    &provider.id,
                                                    app_type_str,
                                                    used_half_open_permit,
                                                    false,
                                                    Some(e.to_string()),
                                                )
                                                .await;

                                            {
                                                let mut status = self.status.write().await;
                                                status.last_error =
                                                    Some(format!("Provider {} 失败: {}", provider.name, e));
                                            }

                                            let (log_code, log_message) = super::logging::build_retryable_failure_log(
                                                &provider.name,
                                                attempted_providers,
                                                providers.len(),
                                                &e,
                                            );
                                            log::warn!("[{app_type_str}] [{log_code}] {log_message}");

                                            last_error = Some(e);
                                            last_provider = Some(provider.clone());
                                            continue;
                                        }
                                        ErrorCategory::NonRetryable | ErrorCategory::ClientAbort => {
                                            self.router
                                                .release_permit_neutral(
                                                    &provider.id,
                                                    app_type_str,
                                                    used_half_open_permit,
                                                )
                                                .await;
                                            {
                                                let mut status = self.status.write().await;
                                                status.failed_requests += 1;
                                                status.last_error = Some(e.to_string());
                                                if status.total_requests > 0 {
                                                    status.success_rate = (status.success_requests as f32
                                                        / status.total_requests as f32)
                                                        * 100.0;
                                                }
                                            }
                                            return Err(ForwardError {
                                                error: e,
                                                provider: Some(provider.clone()),
                                            });
                                        }
                                    }
                                }
                                Err(err) => return Err(err),
                            }
                        }
                        Err(err) => return Err(err),
                    }
                }
            }
        }

        if attempted_providers == 0 {
            {
                let mut status = self.status.write().await;
                status.failed_requests += 1;
                status.last_error = Some("所有供应商暂时不可用（熔断器限制）".to_string());
                if status.total_requests > 0 {
                    status.success_rate =
                        (status.success_requests as f32 / status.total_requests as f32) * 100.0;
                }
            }
            return Err(ForwardError {
                error: ProxyError::NoAvailableProvider,
                provider: None,
            });
        }

        {
            let mut status = self.status.write().await;
            status.failed_requests += 1;
            status.last_error = Some("所有供应商都失败".to_string());
            if status.total_requests > 0 {
                status.success_rate =
                    (status.success_requests as f32 / status.total_requests as f32) * 100.0;
            }
        }

        if let Some((log_code, log_message)) =
            super::logging::build_terminal_failure_log(attempted_providers, providers.len(), last_error.as_ref())
        {
            log::warn!("[{app_type_str}] [{log_code}] {log_message}");
        }

        Err(ForwardError {
            error: last_error.unwrap_or(ProxyError::MaxRetriesExceeded),
            provider: last_provider,
        })
    }
}
