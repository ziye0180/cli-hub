use serde_json::Value;
use std::collections::HashMap;

use crate::app_config::{McpApps, McpServer, MultiAppConfig};
use crate::error::AppError;

use super::super::helpers::collect_enabled_servers;
use super::super::validation::validate_server_spec;

/// Project enabled==true items from config.json to ~/.gemini/settings.json
pub fn sync_enabled_to_gemini(config: &MultiAppConfig) -> Result<(), AppError> {
    let enabled = collect_enabled_servers(&config.mcp.gemini);
    crate::gemini_mcp::set_mcp_servers_map(&enabled)
}

/// Import mcpServers from ~/.gemini/settings.json to unified structure (v3.7.0+)
/// Existing servers will enable Gemini app, without overwriting other fields and app states
pub fn import_from_gemini(config: &mut MultiAppConfig) -> Result<usize, AppError> {
    let text_opt = crate::gemini_mcp::read_mcp_json()?;
    let Some(text) = text_opt else { return Ok(0) };

    let v: Value = serde_json::from_str(&text)
        .map_err(|e| AppError::McpValidation(format!("解析 ~/.gemini/settings.json 失败: {e}")))?;
    let Some(map) = v.get("mcpServers").and_then(|x| x.as_object()) else {
        return Ok(0);
    };

    // Ensure new structure exists
    let servers = config.mcp.servers.get_or_insert_with(HashMap::new);

    let mut changed = 0;
    let mut errors = Vec::new();

    for (id, spec) in map.iter() {
        // Validation: single item failure does not abort, collect errors and continue processing
        if let Err(e) = validate_server_spec(spec) {
            log::warn!("跳过无效 MCP 服务器 '{id}': {e}");
            errors.push(format!("{id}: {e}"));
            continue;
        }

        if let Some(existing) = servers.get_mut(id) {
            // Already exists: only enable Gemini app
            if !existing.apps.gemini {
                existing.apps.gemini = true;
                changed += 1;
                log::info!("MCP 服务器 '{id}' 已启用 Gemini 应用");
            }
        } else {
            // New server: default to enable Gemini only
            servers.insert(
                id.clone(),
                McpServer {
                    id: id.clone(),
                    name: id.clone(),
                    server: spec.clone(),
                    apps: McpApps {
                        claude: false,
                        codex: false,
                        gemini: true,
                    },
                    description: None,
                    homepage: None,
                    docs: None,
                    tags: Vec::new(),
                },
            );
            changed += 1;
            log::info!("导入新 MCP 服务器 '{id}'");
        }
    }

    if !errors.is_empty() {
        log::warn!("导入完成，但有 {} 项失败: {:?}", errors.len(), errors);
    }

    Ok(changed)
}

/// Sync single MCP server to Gemini live config
pub fn sync_single_server_to_gemini(
    _config: &MultiAppConfig,
    id: &str,
    server_spec: &Value,
) -> Result<(), AppError> {
    // Read existing MCP config
    let current = crate::gemini_mcp::read_mcp_servers_map()?;

    // Create new HashMap, containing existing servers + current server to sync
    let mut updated = current;
    updated.insert(id.to_string(), server_spec.clone());

    // Write back
    crate::gemini_mcp::set_mcp_servers_map(&updated)
}

/// Remove single MCP server from Gemini live config
pub fn remove_server_from_gemini(id: &str) -> Result<(), AppError> {
    // Read existing MCP config
    let mut current = crate::gemini_mcp::read_mcp_servers_map()?;

    // Remove specified server
    current.remove(id);

    // Write back
    crate::gemini_mcp::set_mcp_servers_map(&current)
}
