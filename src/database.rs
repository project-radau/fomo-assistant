use anyhow::Context;
use rusqlite::{Connection, Result, OptionalExtension};
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

    Ok(conn)
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