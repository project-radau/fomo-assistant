use crate::product::Product;

pub mod amazon;
pub mod unieuro;
pub mod amazon_browser;

pub async fn scrape_product(url: &str) -> anyhow::Result<Product> {
    if url.contains("amazon.") {
        amazon::scrape_product(url).await
    } else if url.contains("unieuro.") {
        unieuro::scrape_product(url).await
    } else {
        anyhow::bail!("Unsupported shop URL: {}", url)
    }
}