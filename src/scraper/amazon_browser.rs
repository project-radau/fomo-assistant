
use anyhow::{Context, bail};
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use scraper::{Html, Selector};
use crate::product::Product;

pub async fn test_browser_scrape(url: &str) -> anyhow::Result<Product> {
    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .with_head()
            .build()
            .map_err(|err| anyhow::anyhow!("Could not configure Chrome: {err}"))?
    )
    .await
    .context("Could not launch Chrome")?;

    tokio::spawn(async move {
        while handler.next().await.is_some() {}
    });

    let page = browser.new_page(url).await?;

    // Warten, bis die Seite geladen ist.
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let html = page.content().await?;
    let document = Html::parse_document(&html);

    if html.contains("/errors_page/validateCaptcha") {
        bail!("Amazon returned a CAPTCHA page");
    }

    let title_selector = Selector::parse("#productTitle").unwrap();
    let price_selector =
        Selector::parse("#apex-pricetopay-accessibility-label").unwrap();

    let title = document
        .select(&title_selector)
        .next()
        .context("Product title not found")?
        .text()
        .collect::<String>()
        .trim()
        .to_string();

    let price_text = document
        .select(&price_selector)
        .next()
        .context("Product price not found")?
        .text()
        .collect::<String>();

    let price = price_text
        .replace("CHF", "")
        .replace("€", "")
        .replace(",", ".")
        .trim()
        .parse::<f64>()
        .context("Could not parse price")?;

    browser.close().await?;

    Ok(Product::new(title, price))
}
