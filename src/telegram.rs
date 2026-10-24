
use anyhow::{Context, Result};

pub async fn send_message(message: &str) -> Result<()> {
    let token = std::env::var("FOMO_TELEGRAM_BOT_TOKEN")
        .context("Telegram-Bot-Token fehlt")?;

    let chat_id = std::env::var("FOMO_TELEGRAM_CHAT_ID")
        .context("Telegram-Chat-ID fehlt")?;

    let url = format!("https://api.telegram.org/bot{token}/sendMessage");

    let response = reqwest::Client::new()
        .post(url)
        .form(&[
            ("chat_id", chat_id.as_str()),
            ("text", message),
        ])
        .send()
        .await
        .context("Telegram-Request fehlgeschlagen")?;

    let response = response.error_for_status()?;

    let body: serde_json::Value = response.json().await?;

    if body["ok"] != true {
        anyhow::bail!("Telegram konnte die Nachricht nicht senden: {body}");
    }

    Ok(())
}
