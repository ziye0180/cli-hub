use serde_json::{json, Value};

use crate::app_config::AppType;
use crate::codex_config::{get_codex_auth_path, get_codex_config_path, write_codex_live_atomic};
use crate::config::{get_claude_settings_path, read_json_file, write_json_file};
use crate::error::AppError;
use crate::provider::Provider;
use crate::services::mcp::McpService;
use crate::store::AppState;

use super::claude::ClaudeModelNormalizer;
use super::gemini::GeminiAuthDetector;
use super::types::GeminiAuthType;

pub struct LiveConfigSync;

impl LiveConfigSync {
    pub fn write_live_snapshot(app_type: &AppType, provider: &Provider) -> Result<(), AppError> {
        match app_type {
            AppType::Claude => {
                let path = get_claude_settings_path();
                write_json_file(&path, &provider.settings_config)?;
            }
            AppType::Codex => {
                let obj = provider.settings_config.as_object().ok_or_else(|| {
                    AppError::Config("Codex 供应商配置必须是 JSON 对象".to_string())
                })?;
                let auth = obj.get("auth").ok_or_else(|| {
                    AppError::Config("Codex 供应商配置缺少 'auth' 字段".to_string())
                })?;
                let config_str = obj.get("config").and_then(|v| v.as_str()).ok_or_else(|| {
                    AppError::Config("Codex 供应商配置缺少 'config' 字段或不是字符串".to_string())
                })?;

                let auth_path = get_codex_auth_path();
                write_json_file(&auth_path, auth)?;
                let config_path = get_codex_config_path();
                std::fs::write(&config_path, config_str)
                    .map_err(|e| AppError::io(&config_path, e))?;
            }
            AppType::Gemini => {
                use crate::gemini_config::{
                    get_gemini_settings_path, json_to_env, write_gemini_env_atomic,
                };

                let env_value = provider.settings_config.get("env");
                let config_value = provider.settings_config.get("config");

                if let Some(env) = env_value {
                    let env_map = json_to_env(env)?;
                    write_gemini_env_atomic(&env_map)?;
                }

                if let Some(config) = config_value {
                    let settings_path = get_gemini_settings_path();
                    write_json_file(&settings_path, config)?;
                }
            }
        }
        Ok(())
    }

    /// Sync current provider from database to live config
    pub fn sync_current_from_db(state: &AppState) -> Result<(), AppError> {
        for app_type in [AppType::Claude, AppType::Codex, AppType::Gemini] {
            let current_id = match state.db.get_current_provider(app_type.as_str())? {
                Some(id) => id,
                None => continue,
            };
            let providers = state.db.get_all_providers(app_type.as_str())?;
            if let Some(provider) = providers.get(&current_id) {
                Self::write_live_snapshot(&app_type, provider)?;
            } else {
                log::warn!(
                    "无法同步 live 配置: 当前供应商 {} ({}) 未找到",
                    current_id,
                    app_type.as_str()
                );
            }
        }

        McpService::sync_all_enabled(state)?;
        Ok(())
    }

    /// Read current live settings
    pub fn read_live_settings(app_type: AppType) -> Result<Value, AppError> {
        match app_type {
            AppType::Codex => {
                let auth_path = get_codex_auth_path();
                if !auth_path.exists() {
                    return Err(AppError::localized(
                        "codex.auth.missing",
                        "Codex 配置文件不存在：缺少 auth.json",
                        "Codex configuration missing: auth.json not found",
                    ));
                }
                let auth: Value = read_json_file(&auth_path)?;
                let cfg_text = crate::codex_config::read_and_validate_codex_config_text()?;
                Ok(json!({ "auth": auth, "config": cfg_text }))
            }
            AppType::Claude => {
                let path = get_claude_settings_path();
                if !path.exists() {
                    return Err(AppError::localized(
                        "claude.live.missing",
                        "Claude Code 配置文件不存在",
                        "Claude settings file is missing",
                    ));
                }
                read_json_file(&path)
            }
            AppType::Gemini => {
                use crate::gemini_config::{
                    env_to_json, get_gemini_env_path, get_gemini_settings_path, read_gemini_env,
                };

                let env_path = get_gemini_env_path();
                if !env_path.exists() {
                    return Err(AppError::localized(
                        "gemini.env.missing",
                        "Gemini .env 文件不存在",
                        "Gemini .env file not found",
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

                Ok(json!({
                    "env": env_obj,
                    "config": config_obj
                }))
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn write_codex_live(provider: &Provider) -> Result<(), AppError> {
        let settings = provider
            .settings_config
            .as_object()
            .ok_or_else(|| AppError::Config("Codex 配置必须是 JSON 对象".into()))?;
        let auth = settings
            .get("auth")
            .ok_or_else(|| AppError::Config(format!("供应商 {} 缺少 auth 配置", provider.id)))?;
        if !auth.is_object() {
            return Err(AppError::Config(format!(
                "供应商 {} 的 auth 必须是对象",
                provider.id
            )));
        }
        let cfg_text = settings.get("config").and_then(Value::as_str);

        write_codex_live_atomic(auth, cfg_text)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn write_claude_live(provider: &Provider) -> Result<(), AppError> {
        let settings_path = get_claude_settings_path();
        let mut content = provider.settings_config.clone();
        let _ = ClaudeModelNormalizer::normalize_claude_models_in_value(&mut content);
        write_json_file(&settings_path, &content)?;
        Ok(())
    }

    pub(crate) fn write_gemini_live(provider: &Provider) -> Result<(), AppError> {
        use crate::gemini_config::{
            get_gemini_settings_path, json_to_env, validate_gemini_settings_strict,
            write_gemini_env_atomic,
        };

        let auth_type = GeminiAuthDetector::detect_gemini_auth_type(provider);

        let mut env_map = json_to_env(&provider.settings_config)?;

        let mut config_to_write = if let Some(config_value) = provider.settings_config.get("config")
        {
            if config_value.is_null() {
                Some(json!({}))
            } else if config_value.is_object() {
                Some(config_value.clone())
            } else {
                return Err(AppError::localized(
                    "gemini.validation.invalid_config",
                    "Gemini 配置格式错误: config 必须是对象或 null",
                    "Gemini config invalid: config must be an object or null",
                ));
            }
        } else {
            None
        };

        if config_to_write.is_none() {
            let settings_path = get_gemini_settings_path();
            if settings_path.exists() {
                config_to_write = Some(read_json_file(&settings_path)?);
            }
        }

        match auth_type {
            GeminiAuthType::GoogleOfficial => {
                env_map.clear();
                write_gemini_env_atomic(&env_map)?;
            }
            GeminiAuthType::Packycode => {
                validate_gemini_settings_strict(&provider.settings_config)?;
                write_gemini_env_atomic(&env_map)?;
            }
            GeminiAuthType::Generic => {
                validate_gemini_settings_strict(&provider.settings_config)?;
                write_gemini_env_atomic(&env_map)?;
            }
        }

        if let Some(config_value) = config_to_write {
            let settings_path = get_gemini_settings_path();
            write_json_file(&settings_path, &config_value)?;
        }

        match auth_type {
            GeminiAuthType::GoogleOfficial => {
                GeminiAuthDetector::ensure_google_oauth_security_flag(provider)?
            }
            GeminiAuthType::Packycode => {
                GeminiAuthDetector::ensure_packycode_security_flag(provider)?
            }
            GeminiAuthType::Generic => {}
        }

        Ok(())
    }
}
