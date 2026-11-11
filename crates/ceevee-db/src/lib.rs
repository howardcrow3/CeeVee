//! SQLite schema, migrations and connection setup (docs/TECHNICAL_PLAN.md, section 4).
//!
//! WAL mode, foreign keys on, numbered migrations embedded in the binary, and a
//! backup copy of the file written before any pending migration runs.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use ceevee_core::{Error, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

fn db_err(e: impl std::fmt::Display) -> Error {
    Error::Database(e.to_string())
}

/// Handle to the CeeVee database.
#[derive(Debug, Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Open (creating if needed) the database file, back it up if migrations are
    /// pending, then migrate to the latest schema.
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let pending = Self::has_pending_migrations(path).await?;
        if pending && path.exists() {
            backup_file(path)?;
        }
        let pool = connect(path.to_string_lossy().as_ref(), 4).await?;
        MIGRATOR.run(&pool).await.map_err(db_err)?;
        Ok(Self { pool })
    }

    /// In-memory database for tests. Single connection so all queries share it.
    pub async fn open_in_memory() -> Result<Self> {
        let pool = connect(":memory:", 1).await?;
        MIGRATOR.run(&pool).await.map_err(db_err)?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Latest applied migration version, or 0 on an empty database.
    pub async fn schema_version(&self) -> Result<i64> {
        let row: (Option<i64>,) = sqlx::query_as("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(row.0.unwrap_or(0))
    }

    async fn has_pending_migrations(path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false); // fresh file: nothing to lose
        }
        let pool = connect(path.to_string_lossy().as_ref(), 1).await?;
        let applied: Vec<(i64,)> = sqlx::query_as("SELECT version FROM _sqlx_migrations")
            .fetch_all(&pool)
            .await
            .unwrap_or_default();
        pool.close().await;
        Ok(MIGRATOR
            .iter()
            .any(|m| !applied.iter().any(|(v,)| *v == m.version)))
    }
}

async fn connect(target: &str, max: u32) -> Result<SqlitePool> {
    let opts = if target == ":memory:" {
        SqliteConnectOptions::from_str("sqlite::memory:").map_err(db_err)?
    } else {
        SqliteConnectOptions::new()
            .filename(target)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
    }
    .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(max)
        .connect_with(opts)
        .await
        .map_err(db_err)
}

/// Copy `path` to `<name>.bak-<unix seconds>` next to it.
fn backup_file(path: &Path) -> Result<PathBuf> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bak-{secs}"));
    let dest = path.with_file_name(name);
    std::fs::copy(path, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests;

impl Db {
    /// Read a value from the `setting` table.
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        sqlx::query_scalar("SELECT value FROM setting WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)
    }

    /// Insert or replace a value in the `setting` table.
    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO setting(key, value) VALUES (?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }
}
