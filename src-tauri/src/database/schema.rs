use crate::error::AppError;
use rusqlite::Connection;

use super::{lock_conn, Database};

const SCHEMA_VERSION: i32 = 1;

impl Database {
    pub(super) fn create_tables(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::create_tables_on_conn(&conn)
    }

    pub(crate) fn create_tables_on_conn(conn: &Connection) -> Result<(), AppError> {
        // 1. Providers table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS providers (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                website_url TEXT,
                category TEXT,
                created_at INTEGER,
                sort_index INTEGER,
                notes TEXT,
                icon TEXT,
                icon_color TEXT,
                meta TEXT NOT NULL DEFAULT '{}',
                is_current BOOLEAN NOT NULL DEFAULT 0,
                PRIMARY KEY (id, app_type)
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 2. Provider Endpoints table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS provider_endpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                url TEXT NOT NULL,
                added_at INTEGER,
                FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
            )",
            [],
        ).map_err(|e| AppError::Database(e.to_string()))?;

        // 3. MCP Servers table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                server_config TEXT NOT NULL,
                description TEXT,
                homepage TEXT,
                docs TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 4. Prompts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS prompts (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                description TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at INTEGER,
                updated_at INTEGER,
                PRIMARY KEY (id, app_type)
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 5. Skills table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS skills (
                key TEXT PRIMARY KEY,
                installed BOOLEAN NOT NULL DEFAULT 0,
                installed_at INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 6. Skill Repos table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS skill_repos (
                owner TEXT NOT NULL,
                name TEXT NOT NULL,
                branch TEXT NOT NULL DEFAULT 'main',
                enabled BOOLEAN NOT NULL DEFAULT 1,
                skills_path TEXT,
                PRIMARY KEY (owner, name)
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 7. Settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub(super) fn apply_schema_migrations(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::apply_schema_migrations_on_conn(&conn)
    }

    pub(crate) fn apply_schema_migrations_on_conn(conn: &Connection) -> Result<(), AppError> {
        conn.execute("SAVEPOINT schema_migration;", [])
            .map_err(|e| AppError::Database(format!("Failed to start migration savepoint: {e}")))?;

        let mut version = Self::get_user_version(conn)?;

        if version > SCHEMA_VERSION {
            conn.execute("ROLLBACK TO schema_migration;", []).ok();
            conn.execute("RELEASE schema_migration;", []).ok();
            return Err(AppError::Database(format!(
                "Database version too new ({version}), only supports {SCHEMA_VERSION}, please upgrade app."
            )));
        }

        let result = (|| {
            while version < SCHEMA_VERSION {
                match version {
                    0 => {
                        log::info!("Detected user_version=0, migrating to 1 (add missing columns)");
                        Self::add_column_if_missing(conn, "providers", "category", "TEXT")?;
                        Self::add_column_if_missing(conn, "providers", "created_at", "INTEGER")?;
                        Self::add_column_if_missing(conn, "providers", "sort_index", "INTEGER")?;
                        Self::add_column_if_missing(conn, "providers", "notes", "TEXT")?;
                        Self::add_column_if_missing(conn, "providers", "icon", "TEXT")?;
                        Self::add_column_if_missing(conn, "providers", "icon_color", "TEXT")?;
                        Self::add_column_if_missing(
                            conn,
                            "providers",
                            "meta",
                            "TEXT NOT NULL DEFAULT '{}'",
                        )?;
                        Self::add_column_if_missing(
                            conn,
                            "providers",
                            "is_current",
                            "BOOLEAN NOT NULL DEFAULT 0",
                        )?;

                        Self::add_column_if_missing(
                            conn,
                            "provider_endpoints",
                            "added_at",
                            "INTEGER",
                        )?;

                        Self::add_column_if_missing(conn, "mcp_servers", "description", "TEXT")?;
                        Self::add_column_if_missing(conn, "mcp_servers", "homepage", "TEXT")?;
                        Self::add_column_if_missing(conn, "mcp_servers", "docs", "TEXT")?;
                        Self::add_column_if_missing(
                            conn,
                            "mcp_servers",
                            "tags",
                            "TEXT NOT NULL DEFAULT '[]'",
                        )?;
                        Self::add_column_if_missing(
                            conn,
                            "mcp_servers",
                            "enabled_codex",
                            "BOOLEAN NOT NULL DEFAULT 0",
                        )?;
                        Self::add_column_if_missing(
                            conn,
                            "mcp_servers",
                            "enabled_gemini",
                            "BOOLEAN NOT NULL DEFAULT 0",
                        )?;

                        Self::add_column_if_missing(conn, "prompts", "description", "TEXT")?;
                        Self::add_column_if_missing(
                            conn,
                            "prompts",
                            "enabled",
                            "BOOLEAN NOT NULL DEFAULT 1",
                        )?;
                        Self::add_column_if_missing(conn, "prompts", "created_at", "INTEGER")?;
                        Self::add_column_if_missing(conn, "prompts", "updated_at", "INTEGER")?;

                        Self::add_column_if_missing(
                            conn,
                            "skills",
                            "installed_at",
                            "INTEGER NOT NULL DEFAULT 0",
                        )?;

                        Self::add_column_if_missing(
                            conn,
                            "skill_repos",
                            "branch",
                            "TEXT NOT NULL DEFAULT 'main'",
                        )?;
                        Self::add_column_if_missing(
                            conn,
                            "skill_repos",
                            "enabled",
                            "BOOLEAN NOT NULL DEFAULT 1",
                        )?;
                        Self::add_column_if_missing(conn, "skill_repos", "skills_path", "TEXT")?;

                        Self::set_user_version(conn, SCHEMA_VERSION)?;
                    }
                    _ => {
                        return Err(AppError::Database(format!(
                            "Unknown database version {version}, cannot migrate to {SCHEMA_VERSION}"
                        )));
                    }
                }

                version = Self::get_user_version(conn)?;
            }

            Ok(())
        })();

        match result {
            Ok(_) => {
                conn.execute("RELEASE schema_migration;", []).map_err(|e| {
                    AppError::Database(format!("Failed to commit migration savepoint: {e}"))
                })?;
                Ok(())
            }
            Err(e) => {
                conn.execute("ROLLBACK TO schema_migration;", []).ok();
                conn.execute("RELEASE schema_migration;", []).ok();
                Err(e)
            }
        }
    }
}

// Schema version helpers
impl Database {
    fn get_user_version(conn: &Connection) -> Result<i32, AppError> {
        conn.query_row("PRAGMA user_version;", [], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("Failed to read user_version: {e}")))
    }

    fn set_user_version(conn: &Connection, version: i32) -> Result<(), AppError> {
        if version < 0 {
            return Err(AppError::Database("user_version cannot be negative".to_string()));
        }
        let sql = format!("PRAGMA user_version = {version};");
        conn.execute(&sql, [])
            .map_err(|e| AppError::Database(format!("Failed to write user_version: {e}")))?;
        Ok(())
    }
}

// Column validation helpers
impl Database {
    fn validate_identifier(s: &str, kind: &str) -> Result<(), AppError> {
        if s.is_empty() {
            return Err(AppError::Database(format!("{kind} cannot be empty")));
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AppError::Database(format!(
                "Invalid {kind}: {s}, only alphanumeric and underscore allowed"
            )));
        }
        Ok(())
    }

    fn table_exists(conn: &Connection, table: &str) -> Result<bool, AppError> {
        Self::validate_identifier(table, "table name")?;

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .map_err(|e| AppError::Database(format!("Failed to read table names: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("Failed to query table names: {e}")))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let name: String = row
                .get(0)
                .map_err(|e| AppError::Database(format!("Failed to parse table name: {e}")))?;
            if name.eq_ignore_ascii_case(table) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, AppError> {
        Self::validate_identifier(table, "table name")?;
        Self::validate_identifier(column, "column name")?;

        let sql = format!("PRAGMA table_info(\"{table}\");");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(format!("Failed to read table structure: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("Failed to query table structure: {e}")))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let name: String = row
                .get(1)
                .map_err(|e| AppError::Database(format!("Failed to read column name: {e}")))?;
            if name.eq_ignore_ascii_case(column) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<bool, AppError> {
        Self::validate_identifier(table, "table name")?;
        Self::validate_identifier(column, "column name")?;

        if !Self::table_exists(conn, table)? {
            return Err(AppError::Database(format!(
                "Table {table} does not exist, cannot add column {column}"
            )));
        }
        if Self::has_column(conn, table, column)? {
            return Ok(false);
        }

        let sql = format!("ALTER TABLE \"{table}\" ADD COLUMN \"{column}\" {definition};");
        conn.execute(&sql, [])
            .map_err(|e| AppError::Database(format!("Failed to add column {column} to table {table}: {e}")))?;
        log::info!("Added missing column {column} to table {table}");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    const LEGACY_SCHEMA_SQL: &str = r#"
            CREATE TABLE providers (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                PRIMARY KEY (id, app_type)
            );
            CREATE TABLE provider_endpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                url TEXT NOT NULL
            );
            CREATE TABLE mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                server_config TEXT NOT NULL
            );
            CREATE TABLE prompts (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                PRIMARY KEY (id, app_type)
            );
            CREATE TABLE skills (
                key TEXT PRIMARY KEY,
                installed BOOLEAN NOT NULL DEFAULT 0
            );
            CREATE TABLE skill_repos (
                owner TEXT NOT NULL,
                name TEXT NOT NULL,
                PRIMARY KEY (owner, name)
            );
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT
            );
        "#;

    #[derive(Debug)]
    struct ColumnInfo {
        name: String,
        r#type: String,
        notnull: i64,
        default: Option<String>,
    }

    fn get_column_info(conn: &Connection, table: &str, column: &str) -> ColumnInfo {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info(\"{table}\");"))
            .expect("prepare pragma");
        let mut rows = stmt.query([]).expect("query pragma");
        while let Some(row) = rows.next().expect("read row") {
            let name: String = row.get(1).expect("name");
            if name.eq_ignore_ascii_case(column) {
                return ColumnInfo {
                    name,
                    r#type: row.get::<_, String>(2).expect("type"),
                    notnull: row.get::<_, i64>(3).expect("notnull"),
                    default: row.get::<_, Option<String>>(4).ok().flatten(),
                };
            }
        }
        panic!("column {table}.{column} not found");
    }

    fn normalize_default(default: &Option<String>) -> Option<String> {
        default
            .as_ref()
            .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
    }

    #[test]
    fn migration_sets_user_version_when_missing() {
        let conn = Connection::open_in_memory().expect("open memory db");

        Database::create_tables_on_conn(&conn).expect("create tables");
        assert_eq!(
            Database::get_user_version(&conn).expect("read version before"),
            0
        );

        Database::apply_schema_migrations_on_conn(&conn).expect("apply migration");

        assert_eq!(
            Database::get_user_version(&conn).expect("read version after"),
            SCHEMA_VERSION
        );
    }

    #[test]
    fn migration_rejects_future_version() {
        let conn = Connection::open_in_memory().expect("open memory db");
        Database::create_tables_on_conn(&conn).expect("create tables");
        Database::set_user_version(&conn, SCHEMA_VERSION + 1).expect("set future version");

        let err = Database::apply_schema_migrations_on_conn(&conn)
            .expect_err("should reject higher version");
        assert!(
            err.to_string().contains("version too new") || err.to_string().contains("版本过新"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn migration_adds_missing_columns_for_providers() {
        let conn = Connection::open_in_memory().expect("open memory db");

        conn.execute_batch(LEGACY_SCHEMA_SQL)
            .expect("seed old schema");

        Database::apply_schema_migrations_on_conn(&conn).expect("apply migrations");

        // Verify key new columns are added
        for (table, column) in [
            ("providers", "meta"),
            ("providers", "is_current"),
            ("provider_endpoints", "added_at"),
            ("mcp_servers", "enabled_gemini"),
            ("prompts", "updated_at"),
            ("skills", "installed_at"),
            ("skill_repos", "enabled"),
        ] {
            assert!(
                Database::has_column(&conn, table, column).expect("check column"),
                "{table}.{column} should exist after migration"
            );
        }

        // Verify meta column constraints
        let meta = get_column_info(&conn, "providers", "meta");
        assert_eq!(meta.notnull, 1, "meta should be NOT NULL");
        assert_eq!(
            normalize_default(&meta.default).as_deref(),
            Some("{}"),
            "meta default should be '{{}}'"
        );

        assert_eq!(
            Database::get_user_version(&conn).expect("version after migration"),
            SCHEMA_VERSION
        );
    }

    #[test]
    fn migration_aligns_column_defaults_and_types() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(LEGACY_SCHEMA_SQL)
            .expect("seed old schema");

        Database::apply_schema_migrations_on_conn(&conn).expect("apply migrations");

        let is_current = get_column_info(&conn, "providers", "is_current");
        assert_eq!(is_current.r#type, "BOOLEAN");
        assert_eq!(is_current.notnull, 1);
        assert_eq!(
            normalize_default(&is_current.default).as_deref(),
            Some("0")
        );

        let tags = get_column_info(&conn, "mcp_servers", "tags");
        assert_eq!(tags.r#type, "TEXT");
        assert_eq!(tags.notnull, 1);
        assert_eq!(normalize_default(&tags.default).as_deref(), Some("[]"));

        let enabled = get_column_info(&conn, "prompts", "enabled");
        assert_eq!(enabled.r#type, "BOOLEAN");
        assert_eq!(enabled.notnull, 1);
        assert_eq!(
            normalize_default(&enabled.default).as_deref(),
            Some("1")
        );

        let installed_at = get_column_info(&conn, "skills", "installed_at");
        assert_eq!(installed_at.r#type, "INTEGER");
        assert_eq!(installed_at.notnull, 1);
        assert_eq!(
            normalize_default(&installed_at.default).as_deref(),
            Some("0")
        );

        let branch = get_column_info(&conn, "skill_repos", "branch");
        assert_eq!(branch.r#type, "TEXT");
        assert_eq!(normalize_default(&branch.default).as_deref(), Some("main"));

        let skill_repo_enabled = get_column_info(&conn, "skill_repos", "enabled");
        assert_eq!(skill_repo_enabled.r#type, "BOOLEAN");
        assert_eq!(skill_repo_enabled.notnull, 1);
        assert_eq!(
            normalize_default(&skill_repo_enabled.default).as_deref(),
            Some("1")
        );
    }
}
