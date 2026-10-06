use crate::product::Product;
use scraper::Selector;


pub async fn scrape_product(url: &str) -> anyhow::Result<Product> {
    let body = reqwest::get(url)
        .await?
        .text()
        .await?;

    let document = scraper::Html::parse_document(&body);
    
    let selector_current_price = Selector::parse(r#".current-price"#).unwrap();
    let selector_product_title = Selector::parse(r#".product-title"#).unwrap();
    
    let current_price_string: String = document
        .select(&selector_current_price)
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not find current price"))?
        .text()
        .collect();

    let current_price = cleanup_current_value(&current_price_string)
        .parse::<f64>()?;

    let product_title: String = document
        .select(&selector_product_title)
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not find product title"))?
        .text()
        .collect();
    
    Ok(Product::new(product_title, current_price))
}


fn cleanup_current_value(s: &str) -> String {
    s
        .replace("€", "")
        .replace(",", ".")
        .trim().to_string()
} 