use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use sqlx::{Connection, SqliteConnection, SqlitePool};

pub struct ErrorDb {
    // db: SqliteConnection,
    db: sqlx::Pool<sqlx::Sqlite>,
}

#[derive(Debug)]
pub struct ErrorEntry {
    pub id: i64,
    pub printer_id: String,
    pub message: String,
}

impl ErrorDb {
    pub async fn init() -> Result<Self> {
        let path = "errors.db";

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            // .max_connections(5)
            // .connect(path)
            .filename(path)
            .create_if_missing(true);

        let conn = SqlitePool::connect_with(options).await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS error_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    printer_id TEXT NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    message TEXT
)"#,
        )
        .execute(&conn)
        .await?;

        // sqlx::query(
        //     "CREATE TABLE IF NOT EXISTS users (
        //         id INTEGER PRIMARY KEY AUTOINCREMENT,
        //         name TEXT NOT NULL,
        //         email TEXT NOT NULL UNIQUE
        //     )"
        // )
        // .execute(&pool)
        // .await?;

        Ok(Self { db: conn })
    }

    pub async fn insert(&self, printer_id: &str, message: &str) -> Result<()> {
        sqlx::query(r#"INSERT INTO error_log (printer_id, message) VALUES (?, ?)"#)
            .bind(printer_id)
            .bind(message)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
