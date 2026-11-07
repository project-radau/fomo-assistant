use anyhow::Result;
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{Local, Timelike, Duration as ChronoDuration};

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

                let alert_due = if let CheckStatus::Buy = status {
                    is_alert_due(
                        product.value_in_cents(),
                        database::get_last_alert(conn, product_db.id)?,
                        get_current_time_in_ms(),
                    )
                } else {
                    false
                };

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

pub async fn run_automatic_checks() -> Result<()> {
    let conn = database::open_database()?;

    loop {
        match check_products(&conn).await {
            Ok(results) => {
                if let Err(error) =
                    crate::telegram::send_price_alerts(&conn, &results).await
                {
                    eprintln!("Error sending automatic check results: {error}");
                }
            }
            Err(error) => {
                eprintln!("Automatic price check failed: {error}");
            }
        }

        let now = Local::now();

        // 2 minutes after full hour
        let mut next_check = now
            .with_minute(2)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        if next_check <= now {
            next_check += ChronoDuration::hours(1);
        }

        let wait_duration = (next_check - Local::now()).to_std()?;

        tokio::time::sleep(wait_duration).await;
    }
}

fn is_alert_due(
    current_price: i64,
    last_alert: Option<(i64, i64)>,
    now: i64,
) -> bool {
    if let Some(values) = last_alert {
        let price_decreased = current_price < values.0;
        let seven_days_passed = now - values.1 > 7 * 24 * 60 * 60 * 1000;

        if price_decreased || seven_days_passed {
            return true;
        }
    }
    else {
        return true;
    }

    // Sonst keinen Alarm senden.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_due_without_previous_alert() {
        let result = is_alert_due(10000, None, 1000000);

        assert!(result);
    }

    #[test]
    fn test_alert_due_price_lower() {
        let result = is_alert_due(
            10000,
            Some((12000, 1000000)),
            1000000 + 86400000,
        );

        assert!(result);
    }

    #[test]
    fn test_alert_due_price_not_lower_and_last_alert_less_than_a_week() {
        let result = is_alert_due(
            10000,
            Some((10000, 1000000)),
            1000000 + 86400000,
        );

        assert!(!result);
    }

    #[test]
    fn test_alert_due_price_not_lower_and_last_alert_more_than_a_week() {
        let result = is_alert_due(
            10000,
            Some((10000, 1000000)),
            1000000 + 8 * 86400000,
        );

        assert!(result);
    }

    #[test]
    fn test_alert_due_price_not_lower_and_last_alert_exactly_a_week() {
        let result = is_alert_due(
            10000,
            Some((10000, 1000000)),
            1000000 + 7 * 86400000,
        );

        assert!(!result);
    }
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