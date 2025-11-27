use serde_json::{json, Value};
use std::collections::HashMap;

use crate::app_config::{McpApps, McpServer, MultiAppConfig};
use crate::error::AppError;

use super::super::helpers::collect_enabled_servers;
use super::super::toml_convert::json_server_to_toml_table;
use super::super::validation::validate_server_spec;

/// Import MCP from ~/.codex/config.toml to unified structure (v3.7.0+)
///
/// Format support:
/// - Correct format: [mcp_servers.*] (Codex official standard)
/// - Incorrect format: [mcp.servers.*] (fault-tolerant reading, for migration of incorrectly written config)
///
/// Existing servers will enable Codex app, without overwriting other fields and app states
pub fn import_from_codex(config: &mut MultiAppConfig) -> Result<usize, AppError> {
    let text = crate::codex_config::read_and_validate_codex_config_text()?;
    if text.trim().is_empty() {
        return Ok(0);
    }

    let root: toml::Table = toml::from_str(&text)
        .map_err(|e| AppError::McpValidation(format!("解析 ~/.codex/config.toml 失败: {e}")))?;

    // Ensure new structure exists
    let servers = config.mcp.servers.get_or_insert_with(HashMap::new);

    let mut changed_total = 0usize;

    // helper: process a group of servers table
    let mut import_servers_tbl = |servers_tbl: &toml::value::Table| {
        let mut changed = 0usize;
        for (id, entry_val) in servers_tbl.iter() {
            let Some(entry_tbl) = entry_val.as_table() else {
                continue;
            };

            // type defaults to stdio
            let typ = entry_tbl
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("stdio");

            // Build JSON spec
            let mut spec = serde_json::Map::new();
            spec.insert("type".into(), json!(typ));

            // Core fields (fields that need to be manually handled)
            let core_fields = match typ {
                "stdio" => vec!["type", "command", "args", "env", "cwd"],
                "http" | "sse" => vec!["type", "url", "http_headers"],
                _ => vec!["type"],
            };

            // 1. Handle core fields (strongly typed)
            match typ {
                "stdio" => {
                    if let Some(cmd) = entry_tbl.get("command").and_then(|v| v.as_str()) {
                        spec.insert("command".into(), json!(cmd));
                    }
                    if let Some(args) = entry_tbl.get("args").and_then(|v| v.as_array()) {
                        let arr = args
                            .iter()
                            .filter_map(|x| x.as_str())
                            .map(|s| json!(s))
                            .collect::<Vec<_>>();
                        if !arr.is_empty() {
                            spec.insert("args".into(), serde_json::Value::Array(arr));
                        }
                    }
                    if let Some(cwd) = entry_tbl.get("cwd").and_then(|v| v.as_str()) {
                        if !cwd.trim().is_empty() {
                            spec.insert("cwd".into(), json!(cwd));
                        }
                    }
                    if let Some(env_tbl) = entry_tbl.get("env").and_then(|v| v.as_table()) {
                        let mut env_json = serde_json::Map::new();
                        for (k, v) in env_tbl.iter() {
                            if let Some(sv) = v.as_str() {
                                env_json.insert(k.clone(), json!(sv));
                            }
                        }
                        if !env_json.is_empty() {
                            spec.insert("env".into(), serde_json::Value::Object(env_json));
                        }
                    }
                }
                "http" | "sse" => {
                    if let Some(url) = entry_tbl.get("url").and_then(|v| v.as_str()) {
                        spec.insert("url".into(), json!(url));
                    }
                    // Read from http_headers (correct Codex format) or headers (legacy) with priority to http_headers
                    let headers_tbl = entry_tbl
                        .get("http_headers")
                        .and_then(|v| v.as_table())
                        .or_else(|| entry_tbl.get("headers").and_then(|v| v.as_table()));

                    if let Some(headers_tbl) = headers_tbl {
                        let mut headers_json = serde_json::Map::new();
                        for (k, v) in headers_tbl.iter() {
                            if let Some(sv) = v.as_str() {
                                headers_json.insert(k.clone(), json!(sv));
                            }
                        }
                        if !headers_json.is_empty() {
                            spec.insert("headers".into(), serde_json::Value::Object(headers_json));
                        }
                    }
                }
                _ => {
                    log::warn!("跳过未知类型 '{typ}' 的 Codex MCP 项 '{id}'");
                    return changed;
                }
            }

            // 2. Handle extended fields and other unknown fields (generic TOML → JSON conversion)
            for (key, toml_val) in entry_tbl.iter() {
                // Skip already processed core fields
                if core_fields.contains(&key.as_str()) {
                    continue;
                }

                // Generic TOML value to JSON value conversion
                let json_val = match toml_val {
                    toml::Value::String(s) => Some(json!(s)),
                    toml::Value::Integer(i) => Some(json!(i)),
                    toml::Value::Float(f) => Some(json!(f)),
                    toml::Value::Boolean(b) => Some(json!(b)),
                    toml::Value::Array(arr) => {
                        // Only support simple type arrays
                        let json_arr: Vec<serde_json::Value> = arr
                            .iter()
                            .filter_map(|item| match item {
                                toml::Value::String(s) => Some(json!(s)),
                                toml::Value::Integer(i) => Some(json!(i)),
                                toml::Value::Float(f) => Some(json!(f)),
                                toml::Value::Boolean(b) => Some(json!(b)),
                                _ => None,
                            })
                            .collect();
                        if !json_arr.is_empty() {
                            Some(serde_json::Value::Array(json_arr))
                        } else {
                            log::debug!("跳过复杂数组字段 '{key}' (TOML → JSON)");
                            None
                        }
                    }
                    toml::Value::Table(tbl) => {
                        // Shallow table to JSON object (only support string values)
                        let mut json_obj = serde_json::Map::new();
                        for (k, v) in tbl.iter() {
                            if let Some(s) = v.as_str() {
                                json_obj.insert(k.clone(), json!(s));
                            }
                        }
                        if !json_obj.is_empty() {
                            Some(serde_json::Value::Object(json_obj))
                        } else {
                            log::debug!("跳过复杂对象字段 '{key}' (TOML → JSON)");
                            None
                        }
                    }
                    toml::Value::Datetime(_) => {
                        log::debug!("跳过日期时间字段 '{key}' (TOML → JSON)");
                        None
                    }
                };

                if let Some(val) = json_val {
                    spec.insert(key.clone(), val);
                    log::debug!("导入扩展字段 '{key}' = {toml_val:?}");
                }
            }

            let spec_v = serde_json::Value::Object(spec);

            // Validation: single item failure continues processing
            if let Err(e) = validate_server_spec(&spec_v) {
                log::warn!("跳过无效 Codex MCP 项 '{id}': {e}");
                continue;
            }

            if let Some(existing) = servers.get_mut(id) {
                // Already exists: only enable Codex app
                if !existing.apps.codex {
                    existing.apps.codex = true;
                    changed += 1;
                    log::info!("MCP 服务器 '{id}' 已启用 Codex 应用");
                }
            } else {
                // New server: default to enable Codex only
                servers.insert(
                    id.clone(),
                    McpServer {
                        id: id.clone(),
                        name: id.clone(),
                        server: spec_v,
                        apps: McpApps {
                            claude: false,
                            codex: true,
                            gemini: false,
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
        changed
    };

    // 1) Handle mcp.servers
    if let Some(mcp_val) = root.get("mcp") {
        if let Some(mcp_tbl) = mcp_val.as_table() {
            if let Some(servers_val) = mcp_tbl.get("servers") {
                if let Some(servers_tbl) = servers_val.as_table() {
                    changed_total += import_servers_tbl(servers_tbl);
                }
            }
        }
    }

    // 2) Handle mcp_servers
    if let Some(servers_val) = root.get("mcp_servers") {
        if let Some(servers_tbl) = servers_val.as_table() {
            changed_total += import_servers_tbl(servers_tbl);
        }
    }

    Ok(changed_total)
}

/// Write enabled==true items from config.json to ~/.codex/config.toml in TOML format
///
/// Format strategy:
/// - Only correct format: [mcp_servers] top-level table (Codex official standard)
/// - Auto-clean incorrect format: [mcp.servers] (if exists)
/// - Read existing config.toml; if syntax invalid, report error, do not attempt to overwrite
/// - Only update `mcp_servers` table, preserve other keys
/// - Only write enabled items; clean mcp_servers table when no enabled items
pub fn sync_enabled_to_codex(config: &MultiAppConfig) -> Result<(), AppError> {
    use toml_edit::{Item, Table};

    // 1) Collect enabled items (Codex dimension)
    let enabled = collect_enabled_servers(&config.mcp.codex);

    // 2) Read existing config.toml text; keep error return for invalid TOML (do not overwrite file)
    let base_text = crate::codex_config::read_and_validate_codex_config_text()?;

    // 3) Use toml_edit to parse (allow empty file)
    let mut doc = if base_text.trim().is_empty() {
        toml_edit::DocumentMut::default()
    } else {
        base_text
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| AppError::McpValidation(format!("解析 config.toml 失败: {e}")))?
    };

    // 4) Clean possibly existing incorrect format [mcp.servers]
    if let Some(mcp_item) = doc.get_mut("mcp") {
        if let Some(tbl) = mcp_item.as_table_like_mut() {
            if tbl.contains_key("servers") {
                log::warn!("检测到错误的 MCP 格式 [mcp.servers]，正在清理并迁移到 [mcp_servers]");
                tbl.remove("servers");
            }
        }
    }

    // 5) Build target servers table (stable key order)
    if enabled.is_empty() {
        // No enabled items: remove mcp_servers table
        doc.as_table_mut().remove("mcp_servers");
    } else {
        // Build servers table
        let mut servers_tbl = Table::new();
        let mut ids: Vec<_> = enabled.keys().cloned().collect();
        ids.sort();
        for id in ids {
            let spec = enabled.get(&id).expect("spec must exist");
            // Reuse generic conversion function (already includes extended field support)
            match json_server_to_toml_table(spec) {
                Ok(table) => {
                    servers_tbl[&id[..]] = Item::Table(table);
                }
                Err(err) => {
                    log::error!("跳过无效的 MCP 服务器 '{id}': {err}");
                }
            }
        }
        // Use only correct format: [mcp_servers]
        doc["mcp_servers"] = Item::Table(servers_tbl);
    }

    // 6) Write back (only change TOML, do not touch auth.json); toml_edit will try to preserve comments/whitespace/order in unchanged areas
    let new_text = doc.to_string();
    let path = crate::codex_config::get_codex_config_path();
    crate::config::write_text_file(&path, &new_text)?;
    Ok(())
}

/// Sync single MCP server to Codex live config
/// Always use Codex official format [mcp_servers], and clean possibly existing incorrect format [mcp.servers]
pub fn sync_single_server_to_codex(
    _config: &MultiAppConfig,
    id: &str,
    server_spec: &Value,
) -> Result<(), AppError> {
    use toml_edit::Item;

    // Read existing config.toml
    let config_path = crate::codex_config::get_codex_config_path();

    let mut doc = if config_path.exists() {
        let content =
            std::fs::read_to_string(&config_path).map_err(|e| AppError::io(&config_path, e))?;
        content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| AppError::McpValidation(format!("解析 Codex config.toml 失败: {e}")))?
    } else {
        toml_edit::DocumentMut::new()
    };

    // Clean possibly existing incorrect format [mcp.servers]
    if let Some(mcp_item) = doc.get_mut("mcp") {
        if let Some(tbl) = mcp_item.as_table_like_mut() {
            if tbl.contains_key("servers") {
                log::warn!("检测到错误的 MCP 格式 [mcp.servers]，正在清理并迁移到 [mcp_servers]");
                tbl.remove("servers");
            }
        }
    }

    // Ensure [mcp_servers] table exists
    if !doc.contains_key("mcp_servers") {
        doc["mcp_servers"] = toml_edit::table();
    }

    // Convert JSON server spec to TOML table
    let toml_table = json_server_to_toml_table(server_spec)?;

    // Use only correct format: [mcp_servers]
    doc["mcp_servers"][id] = Item::Table(toml_table);

    // Write back file
    std::fs::write(&config_path, doc.to_string()).map_err(|e| AppError::io(&config_path, e))?;

    Ok(())
}

/// Remove single MCP server from Codex live config
/// Delete from correct [mcp_servers] table, and clean data that may exist in incorrect location [mcp.servers]
pub fn remove_server_from_codex(id: &str) -> Result<(), AppError> {
    let config_path = crate::codex_config::get_codex_config_path();

    if !config_path.exists() {
        return Ok(()); // File does not exist, no need to delete
    }

    let content =
        std::fs::read_to_string(&config_path).map_err(|e| AppError::io(&config_path, e))?;

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| AppError::McpValidation(format!("解析 Codex config.toml 失败: {e}")))?;

    // Delete from correct location: [mcp_servers]
    if let Some(mcp_servers) = doc.get_mut("mcp_servers").and_then(|s| s.as_table_mut()) {
        mcp_servers.remove(id);
    }

    // Also clean data that may exist in incorrect location: [mcp.servers] (if exists)
    if let Some(mcp_table) = doc.get_mut("mcp").and_then(|t| t.as_table_mut()) {
        if let Some(servers) = mcp_table.get_mut("servers").and_then(|s| s.as_table_mut()) {
            if servers.remove(id).is_some() {
                log::warn!("从错误的 MCP 格式 [mcp.servers] 中清理了服务器 '{id}'");
            }
        }
    }

    // Write back file
    std::fs::write(&config_path, doc.to_string()).map_err(|e| AppError::io(&config_path, e))?;

    Ok(())
}
