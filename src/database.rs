use anyhow::Context;
use rusqlite::{Connection, Result, OptionalExtension};
use rusqlite::backup::Backup;
use chrono::{Local, Timelike};
use std::fmt;

#[derive(Debug)]
pub struct ProductDatabase {
    pub id: i32,
    pub url: String,
    pub threshold: f64
}

impl std::fmt::Display for ProductDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Product URL: {}\nThreshold: {}", self.url, self.threshold)
    }
}

impl ProductDatabase {
    pub fn threshold_in_cents(&self) -> i64 {
        (self.threshold * 100.0) as i64
    }
}

pub fn open_database() -> anyhow::Result<Connection> {
    let conn = Connection::open("database.db").context("could not open database")?;

    initialize_database(&conn)?;

    Ok(conn)
}

pub fn initialize_database(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "PRAGMA foreign_keys = ON",
        (), // empty list of parameters.
    ).context("could not enable foreign keys")?;

    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value INTEGER NOT NULL
        );", 
        ()
    ).context("could not create settings table")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
            id   INTEGER PRIMARY KEY,
            url TEXT NOT NULL UNIQUE,
            threshold FLOAT NOT NULL
        )",
        (), // empty list of parameters.
    ).context("could not create products table")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS price_history (
            id INTEGER PRIMARY KEY,
            product_id INTEGER NOT NULL,
            price INTEGER NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (product_id) REFERENCES products(id)
        )", 
        ()
    ).context("could not create price_history table")?;

    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS alert_logs (
            id INTEGER PRIMARY KEY,
            product_id INTEGER NOT NULL,
            price INTEGER NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (product_id) REFERENCES products(id)
        );", 
        ()
    ).context("could not create alert_logs table")?;

    Ok(())
}

pub fn get_products(conn: &Connection) -> Result<Vec<ProductDatabase>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT * FROM products"
    )?;

    let products = stmt.query_map([], |row| {
        Ok(
            ProductDatabase {
                id: row.get(0)?,
                url: row.get(1)?,
                threshold: row.get(2)?
            }
        )
    })?;

    products.collect()
}

pub fn add_product(conn: &Connection, url: &str, threshold: f64) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "INSERT OR IGNORE INTO products (url, threshold) VALUES (?1, ?2)",
        (url, threshold),
    )
}

pub fn remove_product(conn: &mut Connection, id: i32) -> Result<usize, rusqlite::Error> {
    let tx = conn.transaction()?;

    tx.execute(
        "DELETE FROM price_history WHERE product_id = ?1",
        [id],
    )?;

    tx.execute(
        "DELETE FROM alert_logs WHERE product_id = ?1",
        [id],
    )?;

    let ret = tx.execute(
        "DELETE FROM products WHERE id = ?1",
        [id],
    )?;

    tx.commit()?;
    Ok(ret)
}

pub fn add_price_entry(conn: &Connection, product_id: i32, price: i64, timestamp: i64) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "INSERT INTO price_history (product_id, price, timestamp) VALUES (?1, ?2, ?3)",
        (product_id, price, timestamp),
    )
}

pub fn add_alert_log(
        conn: &Connection,
        product_id: i32,
        price: i64,
        timestamp: i64,
    ) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "INSERT INTO alert_logs (product_id, price, timestamp) VALUES (?1, ?2, ?3)",
        (product_id, price, timestamp),
    )
}

pub fn get_last_alert(
        conn: &Connection,
        product_id: i32,
    ) -> Result<Option<(i64, i64)>, rusqlite::Error> {
    conn.query_row(
        "SELECT price, timestamp
         FROM alert_logs
         WHERE product_id = ?1
         ORDER BY timestamp DESC
         LIMIT 1",
        [product_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

pub fn get_telegram_offset(conn: &Connection) -> Result<Option<i64>, rusqlite::Error> {
    conn.query_row(
        "SELECT value
         FROM settings
         WHERE key = ?1
         LIMIT 1",
        ["telegram_offset"],
        |row| Ok(row.get(0)?),
    )
    .optional()
}

pub fn save_telegram_offset(
        conn: &Connection,
        offset: i64,
    ) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        ("telegram_offset", offset),
    )
}

pub fn create_backup(conn: &Connection) -> anyhow::Result<()> {
    std::fs::create_dir_all("backups")?;

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let backup_path = format!("backups/backup_{timestamp}.db");

    let mut destination = Connection::open(&backup_path)?;

    let backup = Backup::new(conn, &mut destination)?;

    backup.run_to_completion(5, std::time::Duration::from_millis(100), None)?;

    cleanup_backups()?;

    Ok(())
}

pub async fn run_backup_scheduler() -> anyhow::Result<()> {
    let conn = open_database()?;

    loop {
        let now = Local::now();

        let mut next_backup = now
            .with_hour(3)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        if next_backup <= now {
            next_backup += chrono::Duration::days(1);
        }

        let wait_duration = (next_backup - now).to_std()?;

        tracing::info!(
            "Next database backup scheduled for {}",
            next_backup.format("%Y-%m-%d %H:%M:%S")
        );

        tokio::time::sleep(wait_duration).await;

        tracing::info!("Starting scheduled database backup");

        match create_backup(&conn) {
            Ok(()) => {
                tracing::info!("Scheduled database backup completed");
            }
            Err(error) => {
                tracing::error!("Scheduled database backup failed: {error:#}");
            }
        }
    }
}

fn list_backups() -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut backups = Vec::new();

    for entry in std::fs::read_dir("backups")? {
        let path = entry?.path();

        if path.extension().is_some_and(|ext| ext == "db") {
            backups.push(path);
        }
    }

    backups.sort();

    Ok(backups)
}

fn backup_age_days(path: &std::path::Path) -> anyhow::Result<i64> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid backup filename"))?;

    let date = filename
        .strip_prefix("backup_")
        .and_then(|name| name.get(..10))
        .ok_or_else(|| anyhow::anyhow!("Invalid backup filename"))?;

    let backup_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")?;
    let today = Local::now().date_naive();

    Ok((today - backup_date).num_days())
}

fn should_keep_backup(age_days: i64) -> bool {
    age_days <= 3
        || age_days % 7 == 0 && age_days <= 28
        || age_days % 30 == 0 && age_days <= 180
}

fn cleanup_backups() -> anyhow::Result<()> {
    for path in list_backups()? {
        let age = backup_age_days(&path)?;

        if !should_keep_backup(age) {
            std::fs::remove_file(&path)?;

            tracing::info!(
                "Deleted old database backup: {}",
                path.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_initialize() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()
            .context("could not open database")?;

        initialize_database(&conn)?;

        Ok(())
    }

    #[test]
    fn test_database_add_product() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()
            .context("could not open database")?;

        initialize_database(&conn)?;

        assert_eq!(add_product(&conn, "abc", 500.0)?, 1);
        assert_eq!(add_product(&conn, "abc", 300.0)?, 0);

        Ok(())
    }
}