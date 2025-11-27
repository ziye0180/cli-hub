use crate::error::AppError;
use crate::provider::Provider;
use crate::services::ProviderService;
use crate::store::AppState;
use crate::AppType;
use std::str::FromStr;

use super::types::DeepLinkImportRequest;
use super::utils::infer_homepage_from_endpoint;

/// Import a provider from a deep link request
///
/// This function:
/// 1. Validates the request
/// 2. Merges config file if provided (v3.8+)
/// 3. Converts it to a Provider structure
/// 4. Delegates to ProviderService for actual import
/// 5. Optionally sets as current provider if enabled=true
pub fn import_provider_from_deeplink(
    state: &AppState,
    request: DeepLinkImportRequest,
) -> Result<String, AppError> {
    // Verify this is a provider request
    if request.resource != "provider" {
        return Err(AppError::InvalidInput(format!(
            "Expected provider resource, got '{}'",
            request.resource
        )));
    }

    // Step 1: Merge config file if provided (v3.8+)
    let merged_request = parse_and_merge_config(&request)?;

    // Extract required fields (now as Option)
    let app_str = merged_request
        .app
        .as_ref()
        .ok_or_else(|| AppError::InvalidInput("Missing 'app' field for provider".to_string()))?;

    let api_key = merged_request.api_key.as_ref().ok_or_else(|| {
        AppError::InvalidInput("API key is required (either in URL or config file)".to_string())
    })?;

    if api_key.is_empty() {
        return Err(AppError::InvalidInput(
            "API key cannot be empty".to_string(),
        ));
    }

    let endpoint = merged_request.endpoint.as_ref().ok_or_else(|| {
        AppError::InvalidInput("Endpoint is required (either in URL or config file)".to_string())
    })?;

    if endpoint.is_empty() {
        return Err(AppError::InvalidInput(
            "Endpoint cannot be empty".to_string(),
        ));
    }

    let homepage = merged_request.homepage.as_ref().ok_or_else(|| {
        AppError::InvalidInput("Homepage is required (either in URL or config file)".to_string())
    })?;

    if homepage.is_empty() {
        return Err(AppError::InvalidInput(
            "Homepage cannot be empty".to_string(),
        ));
    }

    let name = merged_request
        .name
        .as_ref()
        .ok_or_else(|| AppError::InvalidInput("Missing 'name' field for provider".to_string()))?;

    // Parse app type
    let app_type = AppType::from_str(app_str)
        .map_err(|_| AppError::InvalidInput(format!("Invalid app type: {app_str}")))?;

    // Build provider configuration based on app type
    let mut provider = build_provider_from_request(&app_type, &merged_request)?;

    // Generate a unique ID for the provider using timestamp + sanitized name
    // This is similar to how frontend generates IDs
    let timestamp = chrono::Utc::now().timestamp_millis();
    let sanitized_name = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase();
    provider.id = format!("{sanitized_name}-{timestamp}");

    let provider_id = provider.id.clone();

    // Use ProviderService to add the provider
    ProviderService::add(state, app_type.clone(), provider)?;

    // If enabled=true, set as current provider
    if merged_request.enabled.unwrap_or(false) {
        // Use ProviderService::switch to set as current and sync to live config
        ProviderService::switch(state, app_type.clone(), &provider_id)?;
        log::info!("Provider '{provider_id}' set as current for {app_type:?}");
    }

    Ok(provider_id)
}

/// Build a Provider structure from a deep link request
pub fn build_provider_from_request(
    app_type: &AppType,
    request: &DeepLinkImportRequest,
) -> Result<Provider, AppError> {
    use serde_json::json;

    let settings_config = match app_type {
        AppType::Claude => {
            // Claude configuration structure
            let mut env = serde_json::Map::new();
            env.insert(
                "ANTHROPIC_AUTH_TOKEN".to_string(),
                json!(request.api_key.clone().unwrap_or_default()),
            );
            env.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                json!(request.endpoint.clone().unwrap_or_default()),
            );

            // Add default model if provided
            if let Some(model) = &request.model {
                env.insert("ANTHROPIC_MODEL".to_string(), json!(model));
            }

            // Add Claude-specific model fields (v3.7.1+)
            if let Some(haiku_model) = &request.haiku_model {
                env.insert(
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
                    json!(haiku_model),
                );
            }
            if let Some(sonnet_model) = &request.sonnet_model {
                env.insert(
                    "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
                    json!(sonnet_model),
                );
            }
            if let Some(opus_model) = &request.opus_model {
                env.insert(
                    "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
                    json!(opus_model),
                );
            }

            json!({ "env": env })
        }
        AppType::Codex => {
            // Codex configuration structure
            // For Codex, we store auth.json (JSON) and config.toml (TOML string) in settings_config。
            //
            // 这里尽量与前端 `getCodexCustomTemplate` 的默认模板保持一致，
            // 再根据深链接参数注入 base_url / model，避免出现"只有 base_url 行"的极简配置，
            // 让通过 UI 新建和通过深链接导入的 Codex 自定义供应商行为一致。

            // 1. 生成一个适合作为 model_provider 名的安全标识
            //    规则尽量与前端 codexProviderPresets.generateThirdPartyConfig 保持一致：
            //    - 转小写
            //    - 非 [a-z0-9_] 统一替换为下划线
            //    - 去掉首尾下划线
            //    - 若结果为空，则使用 "custom"
            let clean_provider_name = {
                let raw: String = request
                    .name
                    .clone()
                    .unwrap_or_else(|| "custom".to_string())
                    .chars()
                    .filter(|c| !c.is_control())
                    .collect();
                let lower = raw.to_lowercase();
                let mut key: String = lower
                    .chars()
                    .map(|c| match c {
                        'a'..='z' | '0'..='9' | '_' => c,
                        _ => '_',
                    })
                    .collect();

                // 去掉首尾下划线
                while key.starts_with('_') {
                    key.remove(0);
                }
                while key.ends_with('_') {
                    key.pop();
                }

                if key.is_empty() {
                    "custom".to_string()
                } else {
                    key
                }
            };

            // 2. 模型名称：优先使用 deeplink 中的 model，否则退回到 Codex 默认模型
            let model_name = request
                .model
                .as_deref()
                .unwrap_or("gpt-5-codex")
                .to_string();

            // 3. 端点：与 UI 中 Base URL 处理方式保持一致，去掉结尾多余的斜杠
            let endpoint = request
                .endpoint
                .as_deref()
                .unwrap_or("")
                .trim()
                .trim_end_matches('/')
                .to_string();

            // 4. 组装 config.toml 内容
            // 使用 Rust 1.58+ 的内联格式化语法，避免 clippy::uninlined_format_args 警告
            let config_toml = format!(
                r#"model_provider = "{clean_provider_name}"
model = "{model_name}"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.{clean_provider_name}]
name = "{clean_provider_name}"
base_url = "{endpoint}"
wire_api = "responses"
requires_openai_auth = true
"#
            );

            json!({
                "auth": {
                    "OPENAI_API_KEY": request.api_key,
                },
                "config": config_toml
            })
        }
        AppType::Gemini => {
            // Gemini configuration structure (.env format)
            let mut env = serde_json::Map::new();
            env.insert("GEMINI_API_KEY".to_string(), json!(request.api_key));
            env.insert(
                "GOOGLE_GEMINI_BASE_URL".to_string(),
                json!(request.endpoint),
            );

            // Add model if provided
            if let Some(model) = &request.model {
                env.insert("GEMINI_MODEL".to_string(), json!(model));
            }

            json!({ "env": env })
        }
    };

    let provider = Provider {
        id: String::new(), // Will be generated by ProviderService
        name: request.name.clone().unwrap_or_default(),
        settings_config,
        website_url: request.homepage.clone(),
        category: None,
        created_at: None,
        sort_index: None,
        notes: request.notes.clone(),
        meta: None,
        icon: request.icon.clone(),
        icon_color: None,
    };

    Ok(provider)
}

/// Parse and merge configuration from Base64 encoded config or remote URL
///
/// Priority: URL params > inline config > remote config
pub fn parse_and_merge_config(
    request: &DeepLinkImportRequest,
) -> Result<DeepLinkImportRequest, AppError> {
    use super::utils::decode_base64_param;

    // If no config provided, return original request
    if request.config.is_none() && request.config_url.is_none() {
        return Ok(request.clone());
    }

    // Step 1: Get config content
    let config_content = if let Some(config_b64) = &request.config {
        // Decode Base64 inline config
        let decoded = decode_base64_param("config", config_b64)?;
        String::from_utf8(decoded)
            .map_err(|e| AppError::InvalidInput(format!("Invalid UTF-8 in config: {e}")))?
    } else if let Some(_config_url) = &request.config_url {
        // Fetch remote config (TODO: implement remote fetching in next phase)
        return Err(AppError::InvalidInput(
            "Remote config URL is not yet supported. Use inline config instead.".to_string(),
        ));
    } else {
        return Ok(request.clone());
    };

    // Step 2: Parse config based on format
    let format = request.config_format.as_deref().unwrap_or("json");
    let config_value: serde_json::Value = match format {
        "json" => serde_json::from_str(&config_content)
            .map_err(|e| AppError::InvalidInput(format!("Invalid JSON config: {e}")))?,
        "toml" => {
            let toml_value: toml::Value = toml::from_str(&config_content)
                .map_err(|e| AppError::InvalidInput(format!("Invalid TOML config: {e}")))?;
            // Convert TOML to JSON for uniform processing
            serde_json::to_value(toml_value)
                .map_err(|e| AppError::Message(format!("Failed to convert TOML to JSON: {e}")))?
        }
        _ => {
            return Err(AppError::InvalidInput(format!(
                "Unsupported config format: {format}"
            )))
        }
    };

    // Step 3: Extract values from config based on app type and merge with URL params
    let mut merged = request.clone();

    // MCP, Skill and other resource types don't need config merging (they use config directly)
    // Only provider resource type needs merging
    if request.resource != "provider" {
        return Ok(merged);
    }

    match request.app.as_deref().unwrap_or("") {
        "claude" => merge_claude_config(&mut merged, &config_value)?,
        "codex" => merge_codex_config(&mut merged, &config_value)?,
        "gemini" => merge_gemini_config(&mut merged, &config_value)?,
        "" => {
            // No app specified, skip merging (this is valid for MCP imports)
            return Ok(merged);
        }
        _ => {
            return Err(AppError::InvalidInput(format!(
                "Invalid app type: {:?}",
                request.app
            )))
        }
    }

    Ok(merged)
}

/// Merge Claude configuration from config file
///
/// Priority: URL params override config file values
fn merge_claude_config(
    request: &mut DeepLinkImportRequest,
    config: &serde_json::Value,
) -> Result<(), AppError> {
    let env = config
        .get("env")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            AppError::InvalidInput("Claude config must have 'env' object".to_string())
        })?;

    // Auto-fill API key if not provided in URL
    if request.api_key.is_none() || request.api_key.as_ref().unwrap().is_empty() {
        if let Some(token) = env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()) {
            request.api_key = Some(token.to_string());
        }
    }

    // Auto-fill endpoint if not provided in URL
    if request.endpoint.is_none() || request.endpoint.as_ref().unwrap().is_empty() {
        if let Some(base_url) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) {
            request.endpoint = Some(base_url.to_string());
        }
    }

    // Auto-fill homepage from endpoint if not provided
    if (request.homepage.is_none() || request.homepage.as_ref().unwrap().is_empty())
        && request.endpoint.is_some()
        && !request.endpoint.as_ref().unwrap().is_empty()
    {
        request.homepage = infer_homepage_from_endpoint(request.endpoint.as_ref().unwrap());
        if request.homepage.is_none() {
            request.homepage = Some("https://anthropic.com".to_string());
        }
    }

    // Auto-fill model fields (URL params take priority)
    if request.model.is_none() {
        request.model = env
            .get("ANTHROPIC_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if request.haiku_model.is_none() {
        request.haiku_model = env
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if request.sonnet_model.is_none() {
        request.sonnet_model = env
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if request.opus_model.is_none() {
        request.opus_model = env
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    Ok(())
}

/// Merge Codex configuration from config file
fn merge_codex_config(
    request: &mut DeepLinkImportRequest,
    config: &serde_json::Value,
) -> Result<(), AppError> {
    // Auto-fill API key from auth.OPENAI_API_KEY
    if request.api_key.is_none() || request.api_key.as_ref().unwrap().is_empty() {
        if let Some(api_key) = config
            .get("auth")
            .and_then(|v| v.get("OPENAI_API_KEY"))
            .and_then(|v| v.as_str())
        {
            request.api_key = Some(api_key.to_string());
        }
    }

    // Auto-fill endpoint and model from config string
    if let Some(config_str) = config.get("config").and_then(|v| v.as_str()) {
        // Parse TOML config string to extract base_url and model
        if let Ok(toml_value) = toml::from_str::<toml::Value>(config_str) {
            // Extract base_url from model_providers section
            if request.endpoint.is_none() || request.endpoint.as_ref().unwrap().is_empty() {
                if let Some(base_url) = extract_codex_base_url(&toml_value) {
                    request.endpoint = Some(base_url);
                }
            }

            // Extract model
            if request.model.is_none() {
                if let Some(model) = toml_value.get("model").and_then(|v| v.as_str()) {
                    request.model = Some(model.to_string());
                }
            }
        }
    }

    // Auto-fill homepage from endpoint
    if (request.homepage.is_none() || request.homepage.as_ref().unwrap().is_empty())
        && request.endpoint.is_some()
        && !request.endpoint.as_ref().unwrap().is_empty()
    {
        request.homepage = infer_homepage_from_endpoint(request.endpoint.as_ref().unwrap());
        if request.homepage.is_none() {
            request.homepage = Some("https://openai.com".to_string());
        }
    }

    Ok(())
}

/// Merge Gemini configuration from config file
fn merge_gemini_config(
    request: &mut DeepLinkImportRequest,
    config: &serde_json::Value,
) -> Result<(), AppError> {
    // Gemini uses flat env structure
    if request.api_key.is_none() || request.api_key.as_ref().unwrap().is_empty() {
        if let Some(api_key) = config.get("GEMINI_API_KEY").and_then(|v| v.as_str()) {
            request.api_key = Some(api_key.to_string());
        }
    }

    if request.endpoint.is_none() || request.endpoint.as_ref().unwrap().is_empty() {
        if let Some(base_url) = config.get("GEMINI_BASE_URL").and_then(|v| v.as_str()) {
            request.endpoint = Some(base_url.to_string());
        }
    }

    if request.model.is_none() {
        request.model = config
            .get("GEMINI_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    // Auto-fill homepage from endpoint
    if (request.homepage.is_none() || request.homepage.as_ref().unwrap().is_empty())
        && request.endpoint.is_some()
        && !request.endpoint.as_ref().unwrap().is_empty()
    {
        request.homepage = infer_homepage_from_endpoint(request.endpoint.as_ref().unwrap());
        if request.homepage.is_none() {
            request.homepage = Some("https://ai.google.dev".to_string());
        }
    }

    Ok(())
}

/// Extract base_url from Codex TOML config
fn extract_codex_base_url(toml_value: &toml::Value) -> Option<String> {
    // Try to find base_url in model_providers section
    if let Some(providers) = toml_value.get("model_providers").and_then(|v| v.as_table()) {
        for (_key, provider) in providers.iter() {
            if let Some(base_url) = provider.get("base_url").and_then(|v| v.as_str()) {
                return Some(base_url.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{store::AppState, Database};
    use base64::prelude::*;
    use std::sync::Arc;

    #[test]
    fn test_build_gemini_provider_with_model() {
        use super::super::types::DeepLinkImportRequest;

        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            app: Some("gemini".to_string()),
            name: Some("Test Gemini".to_string()),
            homepage: Some("https://example.com".to_string()),
            endpoint: Some("https://api.example.com".to_string()),
            api_key: Some("test-api-key".to_string()),
            icon: None,
            model: Some("gemini-2.0-flash".to_string()),
            notes: None,
            haiku_model: None,
            sonnet_model: None,
            opus_model: None,
            config: None,
            config_format: None,
            config_url: None,
            apps: None,
            repo: None,
            directory: None,
            branch: None,
            skills_path: None,
            content: None,
            description: None,
            enabled: None,
        };

        let provider = build_provider_from_request(&AppType::Gemini, &request).unwrap();

        // Verify provider basic info
        assert_eq!(provider.name, "Test Gemini");
        assert_eq!(
            provider.website_url,
            Some("https://example.com".to_string())
        );

        // Verify settings_config structure
        let env = provider.settings_config["env"].as_object().unwrap();
        assert_eq!(env["GEMINI_API_KEY"], "test-api-key");
        assert_eq!(env["GOOGLE_GEMINI_BASE_URL"], "https://api.example.com");
        assert_eq!(env["GEMINI_MODEL"], "gemini-2.0-flash");
    }

    #[test]
    fn test_build_gemini_provider_without_model() {
        use super::super::types::DeepLinkImportRequest;

        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            app: Some("gemini".to_string()),
            name: Some("Test Gemini".to_string()),
            homepage: Some("https://example.com".to_string()),
            endpoint: Some("https://api.example.com".to_string()),
            api_key: Some("test-api-key".to_string()),
            icon: None,
            model: None,
            notes: None,
            haiku_model: None,
            sonnet_model: None,
            opus_model: None,
            config: None,
            config_format: None,
            config_url: None,
            apps: None,
            repo: None,
            directory: None,
            branch: None,
            skills_path: None,
            content: None,
            description: None,
            enabled: None,
        };

        let provider = build_provider_from_request(&AppType::Gemini, &request).unwrap();

        // Verify settings_config structure
        let env = provider.settings_config["env"].as_object().unwrap();
        assert_eq!(env["GEMINI_API_KEY"], "test-api-key");
        assert_eq!(env["GOOGLE_GEMINI_BASE_URL"], "https://api.example.com");
        // Model should not be present
        assert!(env.get("GEMINI_MODEL").is_none());
    }

    #[test]
    fn test_parse_and_merge_config_claude() {
        use super::super::types::DeepLinkImportRequest;

        // Prepare Base64 encoded Claude config
        let config_json = r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-ant-xxx","ANTHROPIC_BASE_URL":"https://api.anthropic.com/v1","ANTHROPIC_MODEL":"claude-sonnet-4.5"}}"#;
        let config_b64 = BASE64_STANDARD.encode(config_json.as_bytes());

        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            app: Some("claude".to_string()),
            name: Some("Test".to_string()),
            homepage: None,
            endpoint: None,
            api_key: None,
            icon: None,
            model: None,
            notes: None,
            haiku_model: None,
            sonnet_model: None,
            opus_model: None,
            config: Some(config_b64),
            config_format: Some("json".to_string()),
            config_url: None,
            apps: None,
            repo: None,
            directory: None,
            branch: None,
            skills_path: None,
            content: None,
            description: None,
            enabled: None,
        };

        let merged = parse_and_merge_config(&request).unwrap();

        // Should auto-fill from config
        assert_eq!(merged.api_key, Some("sk-ant-xxx".to_string()));
        assert_eq!(
            merged.endpoint,
            Some("https://api.anthropic.com/v1".to_string())
        );
        assert_eq!(merged.homepage, Some("https://anthropic.com".to_string()));
        assert_eq!(merged.model, Some("claude-sonnet-4.5".to_string()));
    }

    #[test]
    fn test_parse_and_merge_config_url_override() {
        use super::super::types::DeepLinkImportRequest;

        let config_json = r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-old","ANTHROPIC_BASE_URL":"https://api.anthropic.com/v1"}}"#;
        let config_b64 = BASE64_STANDARD.encode(config_json.as_bytes());

        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            app: Some("claude".to_string()),
            name: Some("Test".to_string()),
            homepage: None,
            endpoint: None,
            api_key: Some("sk-new".to_string()), // URL param should override
            icon: None,
            model: None,
            notes: None,
            haiku_model: None,
            sonnet_model: None,
            opus_model: None,
            config: Some(config_b64),
            config_format: Some("json".to_string()),
            config_url: None,
            apps: None,
            repo: None,
            directory: None,
            branch: None,
            skills_path: None,
            content: None,
            description: None,
            enabled: None,
        };

        let merged = parse_and_merge_config(&request).unwrap();

        // URL param should take priority
        assert_eq!(merged.api_key, Some("sk-new".to_string()));
        // Config file value should be used
        assert_eq!(
            merged.endpoint,
            Some("https://api.anthropic.com/v1".to_string())
        );
    }
}
