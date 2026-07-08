//! 请求转发器
//!
//! 负责将请求转发到上游Provider，支持故障转移

use super::hyper_client::ProxyResponse;
use crate::provider::Provider;
use std::sync::Arc;
use tokio::sync::RwLock;

// Declaring the submodules
mod logging;
mod url_rewrite;
mod req_headers;
mod media;
mod rectifier;
mod copilot;
mod retry;
mod forward;
mod response_prime;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_extra;
#[cfg(test)]
mod tests_retry;

// Re-export what is needed internally and externally
pub(crate) use retry::RetryOutcome;

pub struct ForwardResult {
    pub response: ProxyResponse,
    pub provider: Provider,
    pub claude_api_format: Option<String>,
    /// 实际发往上游的模型名（路由接管/模型映射后的真值）。
    pub outbound_model: Option<String>,
    /// 活跃连接 RAII guard
    pub(crate) connection_guard: Option<ActiveConnectionGuard>,
}

pub struct ForwardError {
    pub error: crate::proxy::ProxyError,
    pub provider: Option<Provider>,
}

/// 活跃连接 RAII guard
pub(crate) struct ActiveConnectionGuard {
    status: Arc<RwLock<crate::proxy::types::ProxyStatus>>,
}

impl ActiveConnectionGuard {
    pub(crate) async fn acquire(status: Arc<RwLock<crate::proxy::types::ProxyStatus>>) -> Self {
        {
            let mut s = status.write().await;
            s.active_connections = s.active_connections.saturating_add(1);
        }
        Self { status }
    }
}

impl Drop for ActiveConnectionGuard {
    fn drop(&mut self) {
        // Drop 不能 await：把减量操作调度到 tokio runtime
        let status = self.status.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut s = status.write().await;
                s.active_connections = s.active_connections.saturating_sub(1);
            });
        }
    }
}

pub struct RequestForwarder {
    /// 共享的 ProviderRouter（持有熔断器状态）
    pub(super) router: Arc<crate::proxy::provider_router::ProviderRouter>,
    pub(super) status: Arc<RwLock<crate::proxy::types::ProxyStatus>>,
    pub(super) current_providers: Arc<RwLock<std::collections::HashMap<String, (String, String)>>>,
    pub(super) gemini_shadow: Arc<crate::proxy::providers::gemini_shadow::GeminiShadowStore>,
    pub(super) codex_chat_history: Arc<crate::proxy::providers::codex_chat_history::CodexChatHistoryStore>,
    /// 故障转移切换管理器
    pub(super) failover_manager: Arc<crate::proxy::failover_switch::FailoverSwitchManager>,
    /// AppHandle，用于发射事件和更新托盘
    pub(super) app_handle: Option<tauri::AppHandle>,
    /// 请求开始时的"当前供应商 ID"（用于判断是否需要同步 UI/托盘）
    pub(super) current_provider_id_at_start: String,
    /// 代理会话 ID（用于 Gemini Native shadow replay）
    pub(super) session_id: String,
    /// Session ID 是否由客户端提供；生成值不能作为上游缓存身份。
    pub(super) session_client_provided: bool,
    /// 整流器配置
    pub(super) rectifier_config: crate::proxy::types::RectifierConfig,
    /// 优化器配置
    pub(super) optimizer_config: crate::proxy::types::OptimizerConfig,
    /// Copilot 优化器配置
    pub(super) copilot_optimizer_config: crate::proxy::types::CopilotOptimizerConfig,
    /// 非流式请求超时（秒）
    pub(super) non_streaming_timeout: std::time::Duration,
    /// 流式请求响应头等待超时（秒）
    pub(super) streaming_first_byte_timeout: std::time::Duration,
    /// 单个客户端请求最多尝试的 provider 数。
    pub(super) max_attempts: usize,
}

impl RequestForwarder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        router: Arc<crate::proxy::provider_router::ProviderRouter>,
        non_streaming_timeout: u64,
        status: Arc<RwLock<crate::proxy::types::ProxyStatus>>,
        current_providers: Arc<RwLock<std::collections::HashMap<String, (String, String)>>>,
        gemini_shadow: Arc<crate::proxy::providers::gemini_shadow::GeminiShadowStore>,
        codex_chat_history: Arc<crate::proxy::providers::codex_chat_history::CodexChatHistoryStore>,
        failover_manager: Arc<crate::proxy::failover_switch::FailoverSwitchManager>,
        app_handle: Option<tauri::AppHandle>,
        current_provider_id_at_start: String,
        session_id: String,
        session_client_provided: bool,
        streaming_first_byte_timeout: u64,
        _streaming_idle_timeout: u64,
        rectifier_config: crate::proxy::types::RectifierConfig,
        optimizer_config: crate::proxy::types::OptimizerConfig,
        copilot_optimizer_config: crate::proxy::types::CopilotOptimizerConfig,
        max_retries: u32,
    ) -> Self {
        // max_retries 是「失败后重试次数」语义，attempt 上限 = retries + 1。
        let max_attempts = (max_retries as usize).saturating_add(1);
        Self {
            router,
            status,
            current_providers,
            gemini_shadow,
            codex_chat_history,
            failover_manager,
            app_handle,
            current_provider_id_at_start,
            session_id,
            session_client_provided,
            rectifier_config,
            optimizer_config,
            copilot_optimizer_config,
            non_streaming_timeout: std::time::Duration::from_secs(non_streaming_timeout),
            streaming_first_byte_timeout: std::time::Duration::from_secs(
                streaming_first_byte_timeout,
            ),
            max_attempts,
        }
    }
}
