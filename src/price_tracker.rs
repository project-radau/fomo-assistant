use anyhow::Result;
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::database;
use crate::scraper;

pub struct CheckResult {
    pub product_name: String,
    pub current_price: Option<f64>,
    pub target_price: f64,
    pub status: CheckStatus,
    pub alert_due: bool,
    pub product_id: i32,
}

pub enum CheckStatus {
    Wait,
    Buy,
    Error,
}

pub async fn check_products(conn: &Connection) -> Result<Vec<CheckResult>> {
    let mut results: Vec<CheckResult> = Vec::new();

    let current_products_db = database::get_products(&conn)?;

    for product_db in current_products_db {
        match scraper::scrape_product(&product_db.url).await {
            Ok(product) => {
                match database::add_price_entry(&conn, product_db.id, product.value_in_cents(), get_current_time_in_ms()) {
                    Ok(_) => {}
                    Err(err) => eprintln!("{}", err)
                }

                let status = if product.value_in_cents() <= product_db.threshold_in_cents() {
                    CheckStatus::Buy
                } else {
                    CheckStatus::Wait
                };

                let mut alert_due = false;

                if let CheckStatus::Buy = status {
                    match database::get_last_alert(&conn, product_db.id)? {
                        Some(last_alert) => {
                            let price_decreased = product.value_in_cents() < last_alert.0;
                            let seven_days_passed = get_current_time_in_ms() - last_alert.1 > 7 * 24 * 60 * 60 * 1000;

                            alert_due = price_decreased || seven_days_passed;
                        }
                        None => {
                            alert_due = true;
                        }
                    }
                }

                results.push(CheckResult {
                    product_name: product.name().to_string(),
                    current_price: Some(product.value()),
                    target_price: product_db.threshold,
                    status: status,
                    alert_due: alert_due,
                    product_id: product_db.id
                });
            }
            Err(err) => {
                eprintln!("{}", err);
                results.push(CheckResult {
                    product_name: product_db.url,
                    current_price: None,
                    target_price: product_db.threshold,
                    status: CheckStatus::Error,
                    alert_due: false,
                    product_id: product_db.id,
                });
            }
        }
    }

    Ok(results)
}

// Source - https://stackoverflow.com/a/44378174
// Posted by Shepmaster, modified by community. See post 'Timeline' for change history
// Retrieved 2026-10-10, License - CC BY-SA 4.0
fn get_current_time_in_ms() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("time should go forward");

    let in_ms = since_the_epoch.as_secs() * 1000 +
    since_the_epoch.subsec_nanos() as u64 / 1_000_000;

    in_ms as i64
}