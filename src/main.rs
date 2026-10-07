mod scraper;
mod product;
mod database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = database::open_database()?;

    let result = database::add_product(&connection, "https://www.unieuro.it/online/Mixer-dj-e-Controller/DDJ-FLX4-pidPDJ8002294", 200.0);

    Ok(())
}

