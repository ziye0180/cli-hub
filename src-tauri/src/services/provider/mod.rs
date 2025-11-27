mod types;
mod gemini;
mod claude;
mod live_config;
mod endpoints;
mod usage;
mod validation;
mod credentials;

pub use types::ProviderSortUpdate;
pub use gemini::GeminiAuthDetector;
pub use claude::ClaudeModelNormalizer;
pub use live_config::LiveConfigSync;
pub use endpoints::EndpointManager;
pub use usage::UsageQueryExecutor;
pub use validation::ProviderValidator;
pub use credentials::CredentialsExtractor;

use indexmap::IndexMap;
use serde_json::{json, Value};

use crate::app_config::AppType;
use crate::codex_config::get_codex_auth_path;
use crate::config::{get_claude_settings_path, read_json_file};
use crate::error::AppError;
use crate::provider::{Provider, UsageResult};
use crate::services::mcp::McpService;
use crate::settings::CustomEndpoint;
use crate::store::AppState;

pub struct ProviderService;

impl ProviderService {
    pub fn sync_current_from_db(state: &AppState) -> Result<(), AppError> {
        LiveConfigSync::sync_current_from_db(state)
    }

    pub fn list(
        state: &AppState,
        app_type: AppType,
    ) -> Result<IndexMap<String, Provider>, AppError> {
        state.db.get_all_providers(app_type.as_str())
    }

    pub fn current(state: &AppState, app_type: AppType) -> Result<String, AppError> {
        state
            .db
            .get_current_provider(app_type.as_str())
            .map(|opt| opt.unwrap_or_default())
    }

    pub fn add(state: &AppState, app_type: AppType, provider: Provider) -> Result<bool, AppError> {
        let mut provider = provider;
        ClaudeModelNormalizer::normalize_provider_if_claude(&app_type, &mut provider);
        ProviderValidator::validate_provider_settings(&app_type, &provider)?;

        state.db.save_provider(app_type.as_str(), &provider)?;

        let current = state.db.get_current_provider(app_type.as_str())?;
        if current.is_none() {
            state
                .db
                .set_current_provider(app_type.as_str(), &provider.id)?;
            LiveConfigSync::write_live_snapshot(&app_type, &provider)?;
        }

        Ok(true)
    }

    pub fn update(
        state: &AppState,
        app_type: AppType,
        provider: Provider,
    ) -> Result<bool, AppError> {
        let mut provider = provider;
        ClaudeModelNormalizer::normalize_provider_if_claude(&app_type, &mut provider);
        ProviderValidator::validate_provider_settings(&app_type, &provider)?;

        let current_id = state.db.get_current_provider(app_type.as_str())?;
        let is_current = current_id.as_deref() == Some(provider.id.as_str());

        state.db.save_provider(app_type.as_str(), &provider)?;

        if is_current {
            LiveConfigSync::write_live_snapshot(&app_type, &provider)?;
            McpService::sync_all_enabled(state)?;
        }

        Ok(true)
    }

    pub fn import_default_config(state: &AppState, app_type: AppType) -> Result<(), AppError> {
        {
            let providers = state.db.get_all_providers(app_type.as_str())?;
            if !providers.is_empty() {
                return Ok(());
            }
        }

        let settings_config = match app_type {
            AppType::Codex => {
                let auth_path = get_codex_auth_path();
                if !auth_path.exists() {
                    return Err(AppError::localized(
                        "codex.live.missing",
                        "Codex 配置文件不存在",
                        "Codex configuration file is missing",
                    ));
                }
                let auth: Value = read_json_file(&auth_path)?;
                let config_str = crate::codex_config::read_and_validate_codex_config_text()?;
                json!({ "auth": auth, "config": config_str })
            }
            AppType::Claude => {
                let settings_path = get_claude_settings_path();
                if !settings_path.exists() {
                    return Err(AppError::localized(
                        "claude.live.missing",
                        "Claude Code 配置文件不存在",
                        "Claude settings file is missing",
                    ));
                }
                let mut v = read_json_file::<Value>(&settings_path)?;
                let _ = ClaudeModelNormalizer::normalize_claude_models_in_value(&mut v);
                v
            }
            AppType::Gemini => {
                use crate::gemini_config::{
                    env_to_json, get_gemini_env_path, get_gemini_settings_path, read_gemini_env,
                };

                let env_path = get_gemini_env_path();
                if !env_path.exists() {
                    return Err(AppError::localized(
                        "gemini.live.missing",
                        "Gemini 配置文件不存在",
                        "Gemini configuration file is missing",
                    ));
                }

                let env_map = read_gemini_env()?;
                let env_json = env_to_json(&env_map);
                let env_obj = env_json.get("env").cloned().unwrap_or_else(|| json!({}));

                let settings_path = get_gemini_settings_path();
                let config_obj = if settings_path.exists() {
                    read_json_file(&settings_path)?
                } else {
                    json!({})
                };

                json!({
                    "env": env_obj,
                    "config": config_obj
                })
            }
        };

        let mut provider = Provider::with_id(
            "default".to_string(),
            "default".to_string(),
            settings_config,
            None,
        );
        provider.category = Some("custom".to_string());

        state.db.save_provider(app_type.as_str(), &provider)?;
        state
            .db
            .set_current_provider(app_type.as_str(), &provider.id)?;

        Ok(())
    }

    pub fn read_live_settings(app_type: AppType) -> Result<Value, AppError> {
        LiveConfigSync::read_live_settings(app_type)
    }

    pub fn get_custom_endpoints(
        state: &AppState,
        app_type: AppType,
        provider_id: &str,
    ) -> Result<Vec<CustomEndpoint>, AppError> {
        EndpointManager::get_custom_endpoints(state, app_type, provider_id)
    }

    pub fn add_custom_endpoint(
        state: &AppState,
        app_type: AppType,
        provider_id: &str,
        url: String,
    ) -> Result<(), AppError> {
        EndpointManager::add_custom_endpoint(state, app_type, provider_id, url)
    }

    pub fn remove_custom_endpoint(
        state: &AppState,
        app_type: AppType,
        provider_id: &str,
        url: String,
    ) -> Result<(), AppError> {
        EndpointManager::remove_custom_endpoint(state, app_type, provider_id, url)
    }

    pub fn update_endpoint_last_used(
        state: &AppState,
        app_type: AppType,
        provider_id: &str,
        url: String,
    ) -> Result<(), AppError> {
        EndpointManager::update_endpoint_last_used(state, app_type, provider_id, url)
    }

    pub fn update_sort_order(
        state: &AppState,
        app_type: AppType,
        updates: Vec<ProviderSortUpdate>,
    ) -> Result<bool, AppError> {
        let mut providers = state.db.get_all_providers(app_type.as_str())?;

        for update in updates {
            if let Some(provider) = providers.get_mut(&update.id) {
                provider.sort_index = Some(update.sort_index);
                state.db.save_provider(app_type.as_str(), provider)?;
            }
        }

        Ok(true)
    }

    pub async fn query_usage(
        state: &AppState,
        app_type: AppType,
        provider_id: &str,
    ) -> Result<UsageResult, AppError> {
        UsageQueryExecutor::query_usage(state, app_type, provider_id).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn test_usage_script(
        state: &AppState,
        app_type: AppType,
        provider_id: &str,
        script_code: &str,
        timeout: u64,
        api_key: Option<&str>,
        base_url: Option<&str>,
        access_token: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<UsageResult, AppError> {
        UsageQueryExecutor::test_usage_script(
            state,
            app_type,
            provider_id,
            script_code,
            timeout,
            api_key,
            base_url,
            access_token,
            user_id,
        )
        .await
    }

    #[allow(dead_code)]
    pub(crate) fn ensure_packycode_security_flag(provider: &Provider) -> Result<(), AppError> {
        GeminiAuthDetector::ensure_packycode_security_flag(provider)
    }

    #[allow(dead_code)]
    pub(crate) fn ensure_google_oauth_security_flag(provider: &Provider) -> Result<(), AppError> {
        GeminiAuthDetector::ensure_google_oauth_security_flag(provider)
    }

    pub(crate) fn write_gemini_live(provider: &Provider) -> Result<(), AppError> {
        LiveConfigSync::write_gemini_live(provider)
    }

    #[allow(dead_code)]
    fn validate_provider_settings(app_type: &AppType, provider: &Provider) -> Result<(), AppError> {
        ProviderValidator::validate_provider_settings(app_type, provider)
    }

    #[allow(dead_code)]
    fn extract_credentials(
        provider: &Provider,
        app_type: &AppType,
    ) -> Result<(String, String), AppError> {
        CredentialsExtractor::extract_credentials(provider, app_type)
    }

    pub fn delete(state: &AppState, app_type: AppType, id: &str) -> Result<(), AppError> {
        let current = state.db.get_current_provider(app_type.as_str())?;
        if current.as_deref() == Some(id) {
            return Err(AppError::Message(
                "无法删除当前正在使用的供应商".to_string(),
            ));
        }
        state.db.delete_provider(app_type.as_str(), id)
    }

    pub fn switch(state: &AppState, app_type: AppType, id: &str) -> Result<(), AppError> {
        let providers = state.db.get_all_providers(app_type.as_str())?;
        let provider = providers
            .get(id)
            .ok_or_else(|| AppError::Message(format!("供应商 {id} 不存在")))?;

        state.db.set_current_provider(app_type.as_str(), id)?;

        LiveConfigSync::write_live_snapshot(&app_type, provider)?;

        McpService::sync_all_enabled(state)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_provider_settings_rejects_missing_auth() {
        let provider = Provider::with_id(
            "codex".into(),
            "Codex".into(),
            json!({ "config": "base_url = \"https://example.com\"" }),
            None,
        );
        let err = ProviderService::validate_provider_settings(&AppType::Codex, &provider)
            .expect_err("missing auth should be rejected");
        assert!(
            err.to_string().contains("auth"),
            "expected auth error, got {err:?}"
        );
    }

    #[test]
    fn extract_credentials_returns_expected_values() {
        let provider = Provider::with_id(
            "claude".into(),
            "Claude".into(),
            json!({
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "token",
                    "ANTHROPIC_BASE_URL": "https://claude.example"
                }
            }),
            None,
        );
        let (api_key, base_url) =
            ProviderService::extract_credentials(&provider, &AppType::Claude).unwrap();
        assert_eq!(api_key, "token");
        assert_eq!(base_url, "https://claude.example");
    }
}
