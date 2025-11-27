use crate::error::AppError;
use crate::prompt::Prompt;
use crate::services::PromptService;
use crate::store::AppState;
use crate::AppType;
use std::str::FromStr;

use super::types::DeepLinkImportRequest;
use super::utils::decode_base64_param;

/// Import a prompt from deep link request
pub fn import_prompt_from_deeplink(
    state: &AppState,
    request: DeepLinkImportRequest,
) -> Result<String, AppError> {
    // Verify this is a prompt request
    if request.resource != "prompt" {
        return Err(AppError::InvalidInput(format!(
            "Expected prompt resource, got '{}'",
            request.resource
        )));
    }

    // Extract required fields
    let app_str = request
        .app
        .as_ref()
        .ok_or_else(|| AppError::InvalidInput("Missing 'app' field for prompt".to_string()))?;

    let name = request
        .name
        .ok_or_else(|| AppError::InvalidInput("Missing 'name' field for prompt".to_string()))?;

    // Parse app type
    let app_type = AppType::from_str(app_str)
        .map_err(|_| AppError::InvalidInput(format!("Invalid app type: {app_str}")))?;

    // Decode content
    let content_b64 = request
        .content
        .as_ref()
        .ok_or_else(|| AppError::InvalidInput("Missing 'content' field for prompt".to_string()))?;

    let content = decode_base64_param("content", content_b64)?;
    let content = String::from_utf8(content)
        .map_err(|e| AppError::InvalidInput(format!("Invalid UTF-8 in content: {e}")))?;

    // Generate ID
    let timestamp = chrono::Utc::now().timestamp_millis();
    let sanitized_name = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase();
    let id = format!("{sanitized_name}-{timestamp}");

    // Check if we should enable this prompt
    let should_enable = request.enabled.unwrap_or(false);

    // Create Prompt (initially disabled)
    let prompt = Prompt {
        id: id.clone(),
        name: name.clone(),
        content,
        description: request.description,
        enabled: false, // Always start as disabled, will be enabled later if needed
        created_at: Some(timestamp),
        updated_at: Some(timestamp),
    };

    // Save using PromptService
    PromptService::upsert_prompt(state, app_type.clone(), &id, prompt)?;

    // If enabled flag is set, enable this prompt (which will disable others)
    if should_enable {
        PromptService::enable_prompt(state, app_type, &id)?;
        log::info!("Successfully imported and enabled prompt '{name}' for {app_str}");
    } else {
        log::info!("Successfully imported prompt '{name}' for {app_str} (disabled)");
    }

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{store::AppState, Database};
    use base64::prelude::*;
    use std::sync::Arc;

    #[test]
    fn test_import_prompt_allows_space_in_base64_content() {
        use super::super::parser::parse_deeplink_url;

        let url = "clihub://v1/import?resource=prompt&app=codex&name=PromptPlus&content=Pj4+";
        let request = parse_deeplink_url(url).unwrap();

        // URL 解码后 content 中的 "+" 会变成空格，确保解码逻辑可以恢复
        assert_eq!(request.content.as_deref(), Some("Pj4 "));

        let db = Arc::new(Database::memory().expect("create memory db"));
        let state = AppState::new(db.clone());

        let prompt_id =
            import_prompt_from_deeplink(&state, request.clone()).expect("import prompt");

        let prompts = state.db.get_prompts("codex").expect("get prompts");
        let prompt = prompts.get(&prompt_id).expect("prompt saved");

        assert_eq!(prompt.content, ">>>");
        assert_eq!(prompt.name, request.name.unwrap());
    }
}
