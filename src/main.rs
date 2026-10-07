mod scraper;
mod product;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://www.unieuro.it/online/Mixer-dj-e-Controller/DDJ-FLX4-pidPDJ8002294";

    let product = scraper::scrape_product(url).await?;

    println!("{}", product);

    Ok(())
}

