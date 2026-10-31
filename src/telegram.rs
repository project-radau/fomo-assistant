
use std::{time::Duration};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::{database, price_tracker::{self, CheckStatus}};

pub async fn send_message(message: &str) -> Result<()> {
    let token = std::env::var("FOMO_TELEGRAM_BOT_TOKEN")
        .context("telegram bot token missing")?;

    let chat_id = std::env::var("FOMO_TELEGRAM_CHAT_ID")
        .context("telegram chat id missing")?;

    let url = format!("https://api.telegram.org/bot{token}/sendMessage");

    let response = reqwest::Client::new()
        .post(url)
        .form(&[
            ("chat_id", chat_id.as_str()),
            ("text", message),
        ])
        .send()
        .await
        .context("telegram request failed")?;

    let response = response.error_for_status()?;

    let body: serde_json::Value = response.json().await?;

    if body["ok"] != true {
        anyhow::bail!("telegram couldnt send message: {body}");
    }

    Ok(())
}

pub async fn get_updates(offset: i64) -> Result<serde_json::Value> {
    let token = std::env::var("FOMO_TELEGRAM_BOT_TOKEN")
        .context("telegram bot token missing")?;

    let url = format!("https://api.telegram.org/bot{token}/getUpdates");

    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(35))
        .build()
        .context("could not build client")?
        .get(url)
        .query(&[("offset", offset), ("timeout", 30)])
        .send()
        .await
        .context("telegram updates could not execute")?;

    let response = response.error_for_status()?;
    let body: serde_json::Value = response.json().await?;

    if body["ok"] != true {
        anyhow::bail!("telegram failed to provide updates: {body}");
    }

    Ok(body)
}

pub async fn run_bot(conn: &Connection) -> Result<()> {

    let chat_id: i64 = std::env::var("FOMO_TELEGRAM_CHAT_ID")
        .context("telegram chat id missing")?
        .parse()
        .context("telegram chat id must be a valid integer")?;

    let mut offset = database::get_telegram_offset(&conn)?.unwrap_or(0);
    
    loop {
        match get_updates(offset).await {
            Ok(result) => {
                if let Some(updates) = result["result"].as_array() {
                    for update in updates {
                        match update["update_id"].as_i64() {
                            Some(update_id) => {
                                if let Some(chat_id_json) = update["message"]["chat"]["id"].as_i64() {
                                    if chat_id == chat_id_json {
                                        if let Some(text) = update["message"]["text"].as_str() {
                                            process_telegram_message(conn, text).await?;
                                        }
                                        offset = update_id + 1;
                                        database::save_telegram_offset(&conn, offset)?;
                                    }
                                    else
                                    {
                                        eprintln!("unknown chat id");
                                        offset = update_id + 1;
                                        database::save_telegram_offset(&conn, offset)?;
                                    }
                                }
                                else {
                                    eprintln!("could not read chat id")
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("{}", err);
            }
        }
    }
}

async fn process_telegram_message(conn: &Connection, message: &str) -> Result<()> {
    if message.eq("/list") {
        process_telegram_list(conn).await?;
    }
    else if message.starts_with("/add ") {
        process_telegram_add(conn, message).await?;
    }
    else if message.eq("/check") {
        process_telegram_check(conn).await?;
    }
    else {
        send_message("command not found").await?;
    }
    
    Ok(())
}

async fn process_telegram_list(conn: &Connection) -> Result<()> {
    let current_products_db = database::get_products(&conn)?;
    let mut message = String::new();
    for product_db in current_products_db {
        message.push_str(&format!(
            "Product #{}\nURL: {}\nTarget price: {:.2}\n\n",
            product_db.id,
            product_db.url,
            product_db.threshold
        ));
    }

    if message.is_empty() {
        send_message("current list is empty").await?;
    }
    else {
        send_message(&message).await?;
    }

    Ok(())
}

async fn process_telegram_add(conn: &Connection, message: &str) -> Result<()> {
    let message_splits: Vec<&str> = message.split_whitespace().collect();

    if message_splits.len() != 3 {
        anyhow::bail!("Invalid command format. Usage: /add URL PRICE")
    }
    else {
        let url = message_splits[1];
        let threshold = message_splits[2]
            .parse::<f64>()
            .context("format error: threshold has to be a number")?;

        let result = database::add_product(&conn, url, threshold)?;
        if result != 0 {
            send_message("Product successfully added").await?;
        }
        else {
            send_message("Product already exists").await?;
        }
    }

    Ok(())
}

async fn process_telegram_check(conn: &Connection) -> Result<()> {
    let results = price_tracker::check_products(conn).await?;

    let mut message = String::new();

    for result in results {
        let should_send_alert = result.alert_due
            && matches!(&result.status, CheckStatus::Buy);

        let status_text = match result.status {
            CheckStatus::Buy => "BUY",
            CheckStatus::Wait => "WAIT",
            CheckStatus::Error => "ERROR",
        };

        let current_price = match result.current_price {
                Some(price) => format!("{price:.2}"),
                None => "-".to_string(),
            };

        message.push_str(&format!(
            "Product: {}\nCurrent Price: {}\nTarget price: {:.2}\nStatus: {}\n\n",
            &result.product_name,
            &current_price,
            &result.target_price,
            &status_text
        ));

        if should_send_alert {

            if let Some(price) = result.current_price {
                let message = format!(
                    "Price alert!\n{}\nCurrent price: {:.2}\nTarget price: {:.2}",
                    result.product_name,
                    price,
                    result.target_price
                );

                send_message(&message).await?;

                database::add_alert_log(
                    &conn,
                    result.product_id,
                    (price * 100.0) as i64,
                    get_current_time_in_ms(),
                )?;
            }
            
        }
    }

    if message.is_empty() {
        send_message("current list is empty").await?;
    }
    else {
        send_message(&message).await?;
    }

    Ok(())
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