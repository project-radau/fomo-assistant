use rusqlite::{params, Connection, Result};

#[derive(Debug)]
pub struct ProductDatabase {
    id: i32,
    url: String,
    threshold: f64
}

pub fn open_database() -> anyhow::Result<Connection> {
    let conn = Connection::open("database.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
            id   INTEGER PRIMARY KEY,
            url TEXT NOT NULL,
            threshold FLOAT NOT NULL
        )",
        (), // empty list of parameters.
    )?;

    Ok(conn)
}

pub fn add_product(conn: &Connection, url: &str, threshold: f64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO products (url, threshold) VALUES (?1, ?2)",
        (&url, &threshold),
    )?;

    Ok(())
}