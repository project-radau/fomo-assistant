
use anyhow::{Context};
use scraper::{Selector};
use crate::product::Product;

pub async fn scrape_product(url: &str) -> anyhow::Result<Product> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/131.0.0.0 Safari/537.36"
        )
        .build()?;

    // 1. Genau einen Request senden
    let response = client
        .get(url)
        .send()
        .await
        .context("Amazon request failed")?;

    // 2. Response-Informationen ausgeben
    println!("HTTP Status: {}", response.status());
    println!("Final URL: {}", response.url());

    for header in [
        "content-type",
        "server",
        "x-amzn-waf-action",
        "x-cache",
        "via",
    ] {
        if let Some(value) = response.headers().get(header) {
            println!("{header}: {value:?}");
        }
    }

    let status = response.status();

    // 3. HTML aus genau dieser Response lesen
    let body = response
        .text()
        .await
        .context("Failed to read Amazon response")?;

    println!("Response size: {} bytes", body.len());

    // 4. Antwort lokal speichern
    std::fs::write("amazon_last_response.html", &body)
        .context("Could not save Amazon response")?;

    println!("HTML saved to amazon_last_response.html");

    if !status.is_success() {
        anyhow::bail!("Amazon returned HTTP status {status}");
    }

    // 5. Prüfen, ob Amazon ein Captcha zurückgegeben hat
    if body.contains("/errors_page/validateCaptcha") {
        anyhow::bail!("Amazon returned a CAPTCHA page");
    }

    // 6. HTML parsen
    let document = scraper::Html::parse_document(&body);

    let selector_product_title =
        Selector::parse("#productTitle").unwrap();

    let selector_price =
        Selector::parse("#apex-pricetopay-accessibility-label").unwrap();

    let product_title: String = document
        .select(&selector_product_title)
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not find Amazon product title"))?
        .text()
        .collect::<String>()
        .trim()
        .to_string();

    let price_string: String = document
        .select(&selector_price)
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not find Amazon product price"))?
        .text()
        .collect::<String>()
        .trim()
        .to_string();

    println!("Amazon product: {product_title}");
    println!("Amazon price: {price_string}");

    let price = parse_price(&price_string)?;

    Ok(Product::new(product_title, price))
}


pub fn test_amazon_html_file(path: &str) -> anyhow::Result<Product> {
    let body = std::fs::read_to_string(path)
        .context("Could not read Amazon HTML file")?;

    println!("Loaded HTML: {} bytes", body.len());

    let document = scraper::Html::parse_document(&body);

    let selector_product_title =
        Selector::parse("#productTitle").unwrap();

    let selector_price =
        Selector::parse("#apex-pricetopay-accessibility-label").unwrap();

    let product_title: String = document
        .select(&selector_product_title)
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not find Amazon product title"))?
        .text()
        .collect::<String>()
        .trim()
        .to_string();

    let price_string: String = document
        .select(&selector_price)
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not find Amazon product price"))?
        .text()
        .collect::<String>()
        .trim()
        .to_string();

    println!("Amazon product: {product_title}");
    println!("Amazon price: {price_string}");

    let price = parse_price(&price_string)?;

    Ok(Product::new(product_title, price))
}


fn parse_price(price: &str) -> anyhow::Result<f64> {
    let cleaned = price
        .replace("CHF", "")
        .replace("€", "")
        .replace("EUR", "")
        .replace(",", ".")
        .trim()
        .to_string();

    cleaned
        .parse::<f64>()
        .context("Could not convert Amazon price to number")
}