use scraper::{Html, Selector};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://www.unieuro.it/online/Mixer-dj-e-Controller/DDJ-FLX4-pidPDJ8002294";
    let body = reqwest::get(url)
        .await?
        .text()
        .await?;

    std::fs::write("output.html", &body)?;

    let document = scraper::Html::parse_document(&body);

    let selector_current_price = Selector::parse(r#".current-price"#).unwrap();

    let current_price_string: String;
    match document.select(&selector_current_price).next() {
        Some(value) => current_price_string = value.text().collect(),
        None => panic!("could not find whole value")
    }

    let current_price_string = cleanup_current_value(current_price_string);

    let current_price: f64 = current_price_string.parse::<f64>()?;

    println!("{}", current_price);

    Ok(())
}

fn cleanup_current_value(s: String) -> String {
    s
        .replace("€", "")
        .replace(",", ".")
        .trim().to_string()
} 