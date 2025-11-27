use crate::app_config::MultiAppConfig;
use crate::error::AppError;
use rusqlite::{params, Connection};

use super::{lock_conn, to_json_string, Database};

impl Database {
    /// Migrate data from MultiAppConfig (JSON)
    pub fn migrate_from_json(&self, config: &MultiAppConfig) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Self::migrate_from_json_tx(&tx, config)?;

        tx.commit()
            .map_err(|e| AppError::Database(format!("Commit migration failed: {e}")))?;
        Ok(())
    }

    /// Run migration dry-run in memory for pre-deployment validation (no disk writes)
    pub fn migrate_from_json_dry_run(config: &MultiAppConfig) -> Result<(), AppError> {
        let mut conn =
            Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))?;
        Self::create_tables_on_conn(&conn)?;
        Self::apply_schema_migrations_on_conn(&conn)?;

        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Self::migrate_from_json_tx(&tx, config)?;

        // Explicitly drop transaction without committing (in-memory DB discarded anyway)
        drop(tx);
        Ok(())
    }

    pub(crate) fn migrate_from_json_tx(
        tx: &rusqlite::Transaction<'_>,
        config: &MultiAppConfig,
    ) -> Result<(), AppError> {
        // 1. Migrate Providers
        for (app_key, manager) in &config.apps {
            let app_type = app_key;
            let current_id = &manager.current;

            for (id, provider) in &manager.providers {
                let is_current = if id == current_id { 1 } else { 0 };

                // Handle meta and endpoints
                let mut meta_clone = provider.meta.clone().unwrap_or_default();
                let endpoints = std::mem::take(&mut meta_clone.custom_endpoints);

                tx.execute(
                    "INSERT OR REPLACE INTO providers (
                        id, app_type, name, settings_config, website_url, category,
                        created_at, sort_index, notes, icon, icon_color, meta, is_current
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        id,
                        app_type,
                        provider.name,
                        to_json_string(&provider.settings_config)?,
                        provider.website_url,
                        provider.category,
                        provider.created_at,
                        provider.sort_index,
                        provider.notes,
                        provider.icon,
                        provider.icon_color,
                        to_json_string(&meta_clone)?,
                        is_current,
                    ],
                )
                .map_err(|e| AppError::Database(format!("Migrate provider failed: {e}")))?;

                // Migrate Endpoints
                for (url, endpoint) in endpoints {
                    tx.execute(
                        "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![id, app_type, url, endpoint.added_at],
                    )
                    .map_err(|e| AppError::Database(format!("Migrate endpoint failed: {e}")))?;
                }
            }
        }

        // 2. Migrate MCP Servers
        if let Some(servers) = &config.mcp.servers {
            for (id, server) in servers {
                tx.execute(
                    "INSERT OR REPLACE INTO mcp_servers (
                        id, name, server_config, description, homepage, docs, tags,
                        enabled_claude, enabled_codex, enabled_gemini
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        id,
                        server.name,
                        to_json_string(&server.server)?,
                        server.description,
                        server.homepage,
                        server.docs,
                        to_json_string(&server.tags)?,
                        server.apps.claude,
                        server.apps.codex,
                        server.apps.gemini,
                    ],
                )
                .map_err(|e| AppError::Database(format!("Migrate mcp server failed: {e}")))?;
            }
        }

        // 3. Migrate Prompts
        let migrate_prompts =
            |prompts_map: &std::collections::HashMap<String, crate::prompt::Prompt>,
             app_type: &str|
             -> Result<(), AppError> {
                for (id, prompt) in prompts_map {
                    tx.execute(
                        "INSERT OR REPLACE INTO prompts (
                        id, app_type, name, content, description, enabled, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            id,
                            app_type,
                            prompt.name,
                            prompt.content,
                            prompt.description,
                            prompt.enabled,
                            prompt.created_at,
                            prompt.updated_at,
                        ],
                    )
                    .map_err(|e| AppError::Database(format!("Migrate prompt failed: {e}")))?;
                }
                Ok(())
            };

        migrate_prompts(&config.prompts.claude.prompts, "claude")?;
        migrate_prompts(&config.prompts.codex.prompts, "codex")?;
        migrate_prompts(&config.prompts.gemini.prompts, "gemini")?;

        // 4. Migrate Skills
        for (key, state) in &config.skills.skills {
            tx.execute(
                "INSERT OR REPLACE INTO skills (key, installed, installed_at) VALUES (?1, ?2, ?3)",
                params![key, state.installed, state.installed_at.timestamp()],
            )
            .map_err(|e| AppError::Database(format!("Migrate skill failed: {e}")))?;
        }

        for repo in &config.skills.repos {
            tx.execute(
                "INSERT OR REPLACE INTO skill_repos (owner, name, branch, enabled, skills_path) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![repo.owner, repo.name, repo.branch, repo.enabled, repo.skills_path],
            ).map_err(|e| AppError::Database(format!("Migrate skill repo failed: {e}")))?;
        }

        // 5. Migrate Common Config
        if let Some(snippet) = &config.common_config_snippets.claude {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params!["common_config_claude", snippet],
            )
            .map_err(|e| AppError::Database(format!("Migrate settings failed: {e}")))?;
        }
        if let Some(snippet) = &config.common_config_snippets.codex {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params!["common_config_codex", snippet],
            )
            .map_err(|e| AppError::Database(format!("Migrate settings failed: {e}")))?;
        }
        if let Some(snippet) = &config.common_config_snippets.gemini {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params!["common_config_gemini", snippet],
            )
            .map_err(|e| AppError::Database(format!("Migrate settings failed: {e}")))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::MultiAppConfig;
    use crate::provider::{Provider, ProviderManager};
    use indexmap::IndexMap;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn dry_run_does_not_write_to_disk() {
        // Create minimal valid config for migration
        let mut apps = HashMap::new();
        apps.insert("claude".to_string(), ProviderManager::default());

        let config = MultiAppConfig {
            version: 2,
            apps,
            mcp: Default::default(),
            prompts: Default::default(),
            skills: Default::default(),
            common_config_snippets: Default::default(),
            claude_common_config_snippet: None,
        };

        // Dry-run should succeed without any file I/O errors
        let result = Database::migrate_from_json_dry_run(&config);
        assert!(
            result.is_ok(),
            "Dry-run should succeed with valid config: {result:?}"
        );
    }

    #[test]
    fn dry_run_validates_schema_compatibility() {
        // Create config with actual provider data
        let mut providers = IndexMap::new();
        providers.insert(
            "test-provider".to_string(),
            Provider {
                id: "test-provider".to_string(),
                name: "Test Provider".to_string(),
                settings_config: json!({
                    "anthropicApiKey": "sk-test-123",
                }),
                website_url: None,
                category: None,
                created_at: Some(1234567890),
                sort_index: None,
                notes: None,
                meta: None,
                icon: None,
                icon_color: None,
            },
        );

        let mut manager = ProviderManager::default();
        manager.providers = providers;
        manager.current = "test-provider".to_string();

        let mut apps = HashMap::new();
        apps.insert("claude".to_string(), manager);

        let config = MultiAppConfig {
            version: 2,
            apps,
            mcp: Default::default(),
            prompts: Default::default(),
            skills: Default::default(),
            common_config_snippets: Default::default(),
            claude_common_config_snippet: None,
        };

        // Dry-run should validate the full migration path
        let result = Database::migrate_from_json_dry_run(&config);
        assert!(
            result.is_ok(),
            "Dry-run should succeed with provider data: {result:?}"
        );
    }
}
