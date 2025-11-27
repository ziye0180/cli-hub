use crate::error::AppError;
use std::collections::HashMap;
use url::Url;

use super::types::DeepLinkImportRequest;
use super::utils::validate_url;

/// Parse a clihub:// URL into a DeepLinkImportRequest
///
/// Expected format:
/// clihub://v1/import?resource={type}&...
pub fn parse_deeplink_url(url_str: &str) -> Result<DeepLinkImportRequest, AppError> {
    // Parse URL
    let url = Url::parse(url_str)
        .map_err(|e| AppError::InvalidInput(format!("Invalid deep link URL: {e}")))?;

    // Validate scheme
    let scheme = url.scheme();
    if scheme != "clihub" {
        return Err(AppError::InvalidInput(format!(
            "Invalid scheme: expected 'clihub', got '{scheme}'"
        )));
    }

    // Extract version from host
    let version = url
        .host_str()
        .ok_or_else(|| AppError::InvalidInput("Missing version in URL host".to_string()))?
        .to_string();

    // Validate version
    if version != "v1" {
        return Err(AppError::InvalidInput(format!(
            "Unsupported protocol version: {version}"
        )));
    }

    // Extract path (should be "/import")
    let path = url.path();
    if path != "/import" {
        return Err(AppError::InvalidInput(format!(
            "Invalid path: expected '/import', got '{path}'"
        )));
    }

    // Parse query parameters
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    // Extract and validate resource type
    let resource = params
        .get("resource")
        .ok_or_else(|| AppError::InvalidInput("Missing 'resource' parameter".to_string()))?
        .clone();

    // Dispatch to appropriate parser based on resource type
    match resource.as_str() {
        "provider" => parse_provider_deeplink(&params, version, resource),
        "prompt" => parse_prompt_deeplink(&params, version, resource),
        "mcp" => parse_mcp_deeplink(&params, version, resource),
        "skill" => parse_skill_deeplink(&params, version, resource),
        _ => Err(AppError::InvalidInput(format!(
            "Unsupported resource type: {resource}"
        ))),
    }
}

/// Parse provider deep link parameters
fn parse_provider_deeplink(
    params: &HashMap<String, String>,
    version: String,
    resource: String,
) -> Result<DeepLinkImportRequest, AppError> {
    let app = params
        .get("app")
        .ok_or_else(|| AppError::InvalidInput("Missing 'app' parameter".to_string()))?
        .clone();

    // Validate app type
    if app != "claude" && app != "codex" && app != "gemini" {
        return Err(AppError::InvalidInput(format!(
            "Invalid app type: must be 'claude', 'codex', or 'gemini', got '{app}'"
        )));
    }

    let name = params
        .get("name")
        .ok_or_else(|| AppError::InvalidInput("Missing 'name' parameter".to_string()))?
        .clone();

    // Make these optional for config file auto-fill (v3.8+)
    let homepage = params.get("homepage").cloned();
    let endpoint = params.get("endpoint").cloned();
    let api_key = params.get("apiKey").cloned();

    // Validate URLs only if provided
    if let Some(ref hp) = homepage {
        if !hp.is_empty() {
            validate_url(hp, "homepage")?;
        }
    }
    if let Some(ref ep) = endpoint {
        if !ep.is_empty() {
            validate_url(ep, "endpoint")?;
        }
    }

    // Extract optional fields
    let model = params.get("model").cloned();
    let notes = params.get("notes").cloned();
    let haiku_model = params.get("haikuModel").cloned();
    let sonnet_model = params.get("sonnetModel").cloned();
    let opus_model = params.get("opusModel").cloned();
    let icon = params
        .get("icon")
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());
    let config = params.get("config").cloned();
    let config_format = params.get("configFormat").cloned();
    let config_url = params.get("configUrl").cloned();
    let enabled = params.get("enabled").and_then(|v| v.parse::<bool>().ok());

    Ok(DeepLinkImportRequest {
        version,
        resource,
        app: Some(app),
        name: Some(name),
        enabled,
        homepage,
        endpoint,
        api_key,
        icon,
        model,
        notes,
        haiku_model,
        sonnet_model,
        opus_model,
        content: None,
        description: None,
        apps: None,
        repo: None,
        directory: None,
        branch: None,
        skills_path: None,
        config,
        config_format,
        config_url,
    })
}

/// Parse prompt deep link parameters
fn parse_prompt_deeplink(
    params: &HashMap<String, String>,
    version: String,
    resource: String,
) -> Result<DeepLinkImportRequest, AppError> {
    let app = params
        .get("app")
        .ok_or_else(|| AppError::InvalidInput("Missing 'app' parameter for prompt".to_string()))?
        .clone();

    // Validate app type
    if app != "claude" && app != "codex" && app != "gemini" {
        return Err(AppError::InvalidInput(format!(
            "Invalid app type: must be 'claude', 'codex', or 'gemini', got '{app}'"
        )));
    }

    let name = params
        .get("name")
        .ok_or_else(|| AppError::InvalidInput("Missing 'name' parameter for prompt".to_string()))?
        .clone();

    let content = params
        .get("content")
        .ok_or_else(|| {
            AppError::InvalidInput("Missing 'content' parameter for prompt".to_string())
        })?
        .clone();

    let description = params.get("description").cloned();
    let enabled = params.get("enabled").and_then(|v| v.parse::<bool>().ok());

    Ok(DeepLinkImportRequest {
        version,
        resource,
        app: Some(app),
        name: Some(name),
        enabled,
        content: Some(content),
        description,
        icon: None,
        homepage: None,
        endpoint: None,
        api_key: None,
        model: None,
        notes: None,
        haiku_model: None,
        sonnet_model: None,
        opus_model: None,
        apps: None,
        repo: None,
        directory: None,
        branch: None,
        skills_path: None,
        config: None,
        config_format: None,
        config_url: None,
    })
}

/// Parse MCP deep link parameters
fn parse_mcp_deeplink(
    params: &HashMap<String, String>,
    version: String,
    resource: String,
) -> Result<DeepLinkImportRequest, AppError> {
    let apps = params
        .get("apps")
        .ok_or_else(|| AppError::InvalidInput("Missing 'apps' parameter for MCP".to_string()))?
        .clone();

    // Validate apps format
    for app in apps.split(',') {
        let trimmed = app.trim();
        if trimmed != "claude" && trimmed != "codex" && trimmed != "gemini" {
            return Err(AppError::InvalidInput(format!(
                "Invalid app in 'apps': must be 'claude', 'codex', or 'gemini', got '{trimmed}'"
            )));
        }
    }

    let config = params
        .get("config")
        .ok_or_else(|| AppError::InvalidInput("Missing 'config' parameter for MCP".to_string()))?
        .clone();

    let enabled = params.get("enabled").and_then(|v| v.parse::<bool>().ok());

    Ok(DeepLinkImportRequest {
        version,
        resource,
        apps: Some(apps),
        enabled,
        config: Some(config),
        config_format: Some("json".to_string()), // MCP config is always JSON
        app: None,
        name: None,
        icon: None,
        homepage: None,
        endpoint: None,
        api_key: None,
        model: None,
        notes: None,
        haiku_model: None,
        sonnet_model: None,
        opus_model: None,
        content: None,
        description: None,
        repo: None,
        directory: None,
        branch: None,
        skills_path: None,
        config_url: None,
    })
}

/// Parse skill deep link parameters
fn parse_skill_deeplink(
    params: &HashMap<String, String>,
    version: String,
    resource: String,
) -> Result<DeepLinkImportRequest, AppError> {
    let repo = params
        .get("repo")
        .ok_or_else(|| AppError::InvalidInput("Missing 'repo' parameter for skill".to_string()))?
        .clone();

    // Validate repo format (should be "owner/name")
    if !repo.contains('/') || repo.split('/').count() != 2 {
        return Err(AppError::InvalidInput(format!(
            "Invalid repo format: expected 'owner/name', got '{repo}'"
        )));
    }

    let directory = params.get("directory").cloned();

    let branch = params.get("branch").cloned();
    let skills_path = params
        .get("skills_path")
        .or_else(|| params.get("skillsPath"))
        .cloned();

    Ok(DeepLinkImportRequest {
        version,
        resource,
        repo: Some(repo),
        directory,
        branch,
        skills_path,
        icon: None,
        app: Some("claude".to_string()), // Skills are Claude-only
        name: None,
        enabled: None,
        homepage: None,
        endpoint: None,
        api_key: None,
        model: None,
        notes: None,
        haiku_model: None,
        sonnet_model: None,
        opus_model: None,
        content: None,
        description: None,
        apps: None,
        config: None,
        config_format: None,
        config_url: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_claude_deeplink() {
        let url = "clihub://v1/import?resource=provider&app=claude&name=Test%20Provider&homepage=https%3A%2F%2Fexample.com&endpoint=https%3A%2F%2Fapi.example.com&apiKey=sk-test-123&icon=claude";

        let request = parse_deeplink_url(url).unwrap();

        assert_eq!(request.version, "v1");
        assert_eq!(request.resource, "provider");
        assert_eq!(request.app, Some("claude".to_string()));
        assert_eq!(request.name, Some("Test Provider".to_string()));
        assert_eq!(request.homepage, Some("https://example.com".to_string()));
        assert_eq!(
            request.endpoint,
            Some("https://api.example.com".to_string())
        );
        assert_eq!(request.api_key, Some("sk-test-123".to_string()));
        assert_eq!(request.icon, Some("claude".to_string()));
    }

    #[test]
    fn test_parse_deeplink_with_notes() {
        let url = "clihub://v1/import?resource=provider&app=codex&name=Codex&homepage=https%3A%2F%2Fcodex.com&endpoint=https%3A%2F%2Fapi.codex.com&apiKey=key123&notes=Test%20notes";

        let request = parse_deeplink_url(url).unwrap();

        assert_eq!(request.notes, Some("Test notes".to_string()));
    }

    #[test]
    fn test_parse_invalid_scheme() {
        let url = "https://v1/import?resource=provider&app=claude&name=Test";

        let result = parse_deeplink_url(url);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid scheme"));
    }

    #[test]
    fn test_parse_unsupported_version() {
        let url = "clihub://v2/import?resource=provider&app=claude&name=Test";

        let result = parse_deeplink_url(url);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported protocol version"));
    }

    #[test]
    fn test_parse_missing_required_field() {
        // Name is still required even in v3.8+ (only homepage/endpoint/apiKey are optional)
        let url = "clihub://v1/import?resource=provider&app=claude";

        let result = parse_deeplink_url(url);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'name' parameter"));
    }

    #[test]
    fn test_parse_prompt_deeplink() {
        use base64::prelude::*;
        let content = "Hello World";
        let content_b64 = BASE64_STANDARD.encode(content);
        let url = format!(
            "clihub://v1/import?resource=prompt&app=claude&name=test&content={}&description=desc&enabled=true",
            content_b64
        );

        let request = parse_deeplink_url(&url).unwrap();
        assert_eq!(request.resource, "prompt");
        assert_eq!(request.app.unwrap(), "claude");
        assert_eq!(request.name.unwrap(), "test");
        assert_eq!(request.content.unwrap(), content_b64);
        assert_eq!(request.description.unwrap(), "desc");
        assert_eq!(request.enabled.unwrap(), true);
    }

    #[test]
    fn test_parse_mcp_deeplink() {
        use base64::prelude::*;
        let config = r#"{"mcpServers":{"test":{"command":"echo"}}}"#;
        let config_b64 = BASE64_STANDARD.encode(config);
        let url = format!(
            "clihub://v1/import?resource=mcp&apps=claude,codex&config={}&enabled=true",
            config_b64
        );

        let request = parse_deeplink_url(&url).unwrap();
        assert_eq!(request.resource, "mcp");
        assert_eq!(request.apps.unwrap(), "claude,codex");
        assert_eq!(request.config.unwrap(), config_b64);
        assert_eq!(request.enabled.unwrap(), true);
    }

    #[test]
    fn test_parse_skill_deeplink() {
        let url = "clihub://v1/import?resource=skill&repo=owner/repo&directory=skills&branch=dev&skills_path=src";
        let request = parse_deeplink_url(&url).unwrap();

        assert_eq!(request.resource, "skill");
        assert_eq!(request.repo.unwrap(), "owner/repo");
        assert_eq!(request.directory.unwrap(), "skills");
        assert_eq!(request.branch.unwrap(), "dev");
        assert_eq!(request.skills_path.unwrap(), "src");
    }
}
