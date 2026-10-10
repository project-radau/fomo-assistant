use rusqlite::{Connection, Result};
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

#[derive(Debug)]
pub struct PriceEntryDatabase {
    pub id: i32,
    pub product_id: i32,
    pub price: i64,
    pub timestamp: i64
}

pub fn open_database() -> anyhow::Result<Connection> {
    let conn = Connection::open("database.db")?;

    conn.execute(
        "PRAGMA foreign_keys = ON",
        (), // empty list of parameters.
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
            id   INTEGER PRIMARY KEY,
            url TEXT NOT NULL UNIQUE,
            threshold FLOAT NOT NULL
        )",
        (), // empty list of parameters.
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS price_history (
            id INTEGER PRIMARY KEY,
            product_id INTEGER NOT NULL,
            price INTEGER NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (product_id) REFERENCES products(id)
        )", 
        ()
    )?;

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

pub fn add_price_entry(conn: &Connection, product_id: i32, price: i64, timestamp: i64) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "INSERT INTO price_history (product_id, price, timestamp) VALUES (?1, ?2, ?3)",
        (product_id, price, timestamp),
    )
}