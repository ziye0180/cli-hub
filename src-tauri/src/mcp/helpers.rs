use serde_json::{json, Value};
use std::collections::HashMap;

use crate::app_config::{AppType, McpConfig, MultiAppConfig};
use crate::error::AppError;

use super::validation::validate_mcp_entry;

pub fn normalize_server_keys(map: &mut HashMap<String, Value>) -> usize {
    let mut change_count = 0usize;
    let mut renames: Vec<(String, String)> = Vec::new();

    for (key_ref, value) in map.iter_mut() {
        let key = key_ref.clone();
        let Some(obj) = value.as_object_mut() else {
            continue;
        };

        let id_value = obj.get("id").cloned();

        let target_id: String;

        match id_value {
            Some(id_val) => match id_val.as_str() {
                Some(id_str) => {
                    let trimmed = id_str.trim();
                    if trimmed.is_empty() {
                        obj.insert("id".into(), json!(key.clone()));
                        change_count += 1;
                        target_id = key.clone();
                    } else {
                        if trimmed != id_str {
                            obj.insert("id".into(), json!(trimmed));
                            change_count += 1;
                        }
                        target_id = trimmed.to_string();
                    }
                }
                None => {
                    obj.insert("id".into(), json!(key.clone()));
                    change_count += 1;
                    target_id = key.clone();
                }
            },
            None => {
                obj.insert("id".into(), json!(key.clone()));
                change_count += 1;
                target_id = key.clone();
            }
        }

        if target_id != key {
            renames.push((key, target_id));
        }
    }

    for (old_key, new_key) in renames {
        if old_key == new_key {
            continue;
        }
        if map.contains_key(&new_key) {
            log::warn!("MCP 条目 '{old_key}' 的内部 id '{new_key}' 与现有键冲突，回退为原键");
            if let Some(value) = map.get_mut(&old_key) {
                if let Some(obj) = value.as_object_mut() {
                    if obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s != old_key)
                        .unwrap_or(true)
                    {
                        obj.insert("id".into(), json!(old_key.clone()));
                        change_count += 1;
                    }
                }
            }
            continue;
        }
        if let Some(mut value) = map.remove(&old_key) {
            if let Some(obj) = value.as_object_mut() {
                obj.insert("id".into(), json!(new_key.clone()));
            }
            log::info!("MCP 条目键名已自动修复: '{old_key}' -> '{new_key}'");
            map.insert(new_key, value);
            change_count += 1;
        }
    }

    change_count
}

pub fn normalize_servers_for(config: &mut MultiAppConfig, app: &AppType) -> usize {
    let servers = &mut config.mcp_for_mut(app).servers;
    normalize_server_keys(servers)
}

pub fn extract_server_spec(entry: &Value) -> Result<Value, AppError> {
    let obj = entry
        .as_object()
        .ok_or_else(|| AppError::McpValidation("MCP 服务器条目必须为 JSON 对象".into()))?;
    let server = obj
        .get("server")
        .ok_or_else(|| AppError::McpValidation("MCP 服务器条目缺少 server 字段".into()))?;

    if !server.is_object() {
        return Err(AppError::McpValidation(
            "MCP 服务器 server 字段必须为 JSON 对象".into(),
        ));
    }

    Ok(server.clone())
}

/// Returns enabled MCP servers (filtered by enabled==true)
pub fn collect_enabled_servers(cfg: &McpConfig) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for (id, entry) in cfg.servers.iter() {
        let enabled = entry
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !enabled {
            continue;
        }
        match extract_server_spec(entry) {
            Ok(spec) => {
                out.insert(id.clone(), spec);
            }
            Err(err) => {
                log::warn!("跳过无效的 MCP 条目 '{id}': {err}");
            }
        }
    }
    out
}

#[allow(dead_code)] // v3.7.0: Old per-app API, retained for future possible migration
pub fn get_servers_snapshot_for(
    config: &mut MultiAppConfig,
    app: &AppType,
) -> (HashMap<String, Value>, usize) {
    let normalized = normalize_servers_for(config, app);
    let mut snapshot = config.mcp_for(app).servers.clone();
    snapshot.retain(|id, value| {
        let Some(obj) = value.as_object_mut() else {
            log::warn!("跳过无效的 MCP 条目 '{id}': 必须为 JSON 对象");
            return false;
        };

        obj.entry(String::from("id")).or_insert(json!(id));

        match validate_mcp_entry(value) {
            Ok(()) => true,
            Err(err) => {
                log::error!("config.json 中存在无效的 MCP 条目 '{id}': {err}");
                false
            }
        }
    });
    (snapshot, normalized)
}

#[allow(dead_code)] // v3.7.0: Old per-app API, retained for future possible migration
pub fn upsert_in_config_for(
    config: &mut MultiAppConfig,
    app: &AppType,
    id: &str,
    spec: Value,
) -> Result<bool, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::InvalidInput("MCP 服务器 ID 不能为空".into()));
    }
    normalize_servers_for(config, app);
    validate_mcp_entry(&spec)?;

    let mut entry_obj = spec
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::McpValidation("MCP 服务器条目必须为 JSON 对象".into()))?;
    if let Some(existing_id) = entry_obj.get("id") {
        let Some(existing_id_str) = existing_id.as_str() else {
            return Err(AppError::McpValidation("MCP 服务器 id 必须为字符串".into()));
        };
        if existing_id_str != id {
            return Err(AppError::McpValidation(format!(
                "MCP 服务器条目中的 id '{existing_id_str}' 与参数 id '{id}' 不一致"
            )));
        }
    } else {
        entry_obj.insert(String::from("id"), json!(id));
    }

    let value = Value::Object(entry_obj);

    let servers = &mut config.mcp_for_mut(app).servers;
    let before = servers.get(id).cloned();
    servers.insert(id.to_string(), value);

    Ok(before.is_none())
}

#[allow(dead_code)] // v3.7.0: Old per-app API, retained for future possible migration
pub fn delete_in_config_for(
    config: &mut MultiAppConfig,
    app: &AppType,
    id: &str,
) -> Result<bool, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::InvalidInput("MCP 服务器 ID 不能为空".into()));
    }
    normalize_servers_for(config, app);
    let existed = config.mcp_for_mut(app).servers.remove(id).is_some();
    Ok(existed)
}

#[allow(dead_code)] // v3.7.0: Old per-app API, retained for future possible migration
/// Set enabled status (does not perform disk write or file sync)
pub fn set_enabled_flag_for(
    config: &mut MultiAppConfig,
    app: &AppType,
    id: &str,
    enabled: bool,
) -> Result<bool, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::InvalidInput("MCP 服务器 ID 不能为空".into()));
    }
    normalize_servers_for(config, app);
    if let Some(spec) = config.mcp_for_mut(app).servers.get_mut(id) {
        // Write enabled field
        let mut obj = spec
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::McpValidation("MCP 服务器定义必须为 JSON 对象".into()))?;
        obj.insert("enabled".into(), json!(enabled));
        *spec = Value::Object(obj);
    } else {
        // If not exists, return false directly
        return Ok(false);
    }

    Ok(true)
}
