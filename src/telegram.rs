
use std::{time::Duration};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::database;

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
                                            println!("{text}");
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