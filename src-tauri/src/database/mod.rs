use crate::error::AppError;
use rusqlite::Connection;
use std::sync::Mutex;

mod backup;
mod migration;
mod schema;
pub mod dao;

/// Safe JSON serialization helper
pub(crate) fn to_json_string<T: serde::Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|e| AppError::Config(format!("JSON serialization failed: {e}")))
}

/// Safe Mutex lock helper - used across the database module
macro_rules! lock_conn {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|e| AppError::Database(format!("Mutex lock failed: {}", e)))?
    };
}

pub(crate) use lock_conn;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Initialize database connection and create tables
    pub fn init() -> Result<Self, AppError> {
        let db_path = crate::config::get_app_config_dir().join("cli-hub.db");

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let conn = Connection::open(&db_path).map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;
        db.apply_schema_migrations()?;

        Ok(db)
    }

    /// Create in-memory database (for testing)
    pub fn memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;

        Ok(db)
    }

    /// Create snapshot to avoid long-time lock
    pub(crate) fn snapshot_to_memory(&self) -> Result<Connection, AppError> {
        use rusqlite::backup::Backup;
        let conn = lock_conn!(self.conn);
        let mut snapshot =
            Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))?;

        {
            let backup =
                Backup::new(&conn, &mut snapshot).map_err(|e| AppError::Database(e.to_string()))?;
            backup
                .step(-1)
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(snapshot)
    }

    /// Check if database is empty for first import
    pub fn is_empty_for_first_import(&self) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);

        let mcp_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mcp_servers", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;

        let prompt_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;

        let skill_repo_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_repos", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;

        let provider_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(mcp_count == 0 && prompt_count == 0 && skill_repo_count == 0 && provider_count == 0)
    }
}
