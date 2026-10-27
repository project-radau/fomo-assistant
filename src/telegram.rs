
use std::{time::Duration};

use anyhow::{Context, Result};

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

    let response = reqwest::Client::new()
        .get(url)
        .query(&[("offset", offset)])
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

pub async fn run_bot() -> Result<()> {
    let mut offset: i64 = 0;
    loop {
        match get_updates(offset).await {
            Ok(result) => {
                println!("{}", result["result"][0]["message"]["text"]);
                match result["result"][0]["update_id"].as_i64() {
                    Some(update_id) => {
                        offset = update_id + 1;
                    }
                    None => {}
                }
            }
            Err(err) => {
                eprintln!("{}", err);
            }
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}