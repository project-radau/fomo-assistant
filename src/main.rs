use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use clap::{Parser, Subcommand};
use comfy_table::*;

use crate::price_tracker::CheckStatus;

mod scraper;
mod product;
mod database;
mod telegram;
mod price_tracker;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run,
    Add {
        url: String,
        #[arg(short, long)]
        threshold: f64,
    },
    Remove,
    List,
    Check
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let mut conn = database::open_database()?;
    let mut table = Table::new();
    let args = Cli::parse();

    match args.command {
        Commands::Run => {
            telegram::run_bot(&mut conn).await?;
        },
        Commands::Add { url, threshold } =>  {
            database::add_product(&conn, &url, threshold)?;
        },
        Commands::Remove => {
            let current_products_db = database::get_products(&conn)?;

            table
                .set_header(vec!["ID", "Product", "Target Price"]);

            for product_db in current_products_db {
                table.add_row(vec![
                    Cell::new(product_db.id), 
                    Cell::new(&product_db.url), 
                    Cell::new(&format!("{:.2}", product_db.threshold))
                ]);
            }

            println!("{table}");

            println!("please insert the id:");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match input.trim().parse::<i32>() {
                Ok(value) => {
                    match database::remove_product(&mut conn, value) {
                        Ok(result) => {
                            if result > 0 {
                                println!("product deleted.");
                            }
                            else {
                                eprintln!("product not found");
                            }
                        }
                        Err(err) => {
                            eprintln!("{}", err)
                        }
                    }
                }
                Err(_) => {
                    eprintln!("Invalid ID. Please enter a number.");
                }
            }
        }
        Commands::List => {
            let current_products_db = database::get_products(&conn)?;

            table
                .set_header(vec!["ID", "Product", "Target Price"]);

            for product_db in current_products_db {
                table.add_row(vec![
                    Cell::new(product_db.id), 
                    Cell::new(&product_db.url), 
                    Cell::new(&format!("{:.2}", product_db.threshold))
                ]);
            }

            println!("{table}");
        },
        Commands::Check => {
            let results = price_tracker::check_products(&conn).await?;

            table
                .set_header(vec!["Product", "Current Price", "Target Price", "Status"]);

            for result in results {
                let should_send_alert = result.alert_due
                    && matches!(&result.status, CheckStatus::Buy);

                let (status_text, status_color) = match result.status {
                    CheckStatus::Buy => ("BUY", Color::Green),
                    CheckStatus::Wait => ("WAIT", Color::Yellow),
                    CheckStatus::Error => ("ERROR", Color::Red),
                };

                let current_price = match result.current_price {
                    Some(price) => format!("{price:.2}"),
                    None => "-".to_string(),
                };

                table.add_row(vec![
                    Cell::new(&result.product_name),
                    Cell::new(&current_price),
                    Cell::new(&format!("{:.2}", result.target_price)),
                    Cell::new(status_text).fg(status_color),
                ]);

                if should_send_alert {

                    if let Some(price) = result.current_price {
                        let message = format!(
                            "Price alert!\n{}\nCurrent price: {:.2}\nTarget price: {:.2}",
                            result.product_name,
                            price,
                            result.target_price
                        );

                        telegram::send_message(&message).await?;

                        database::add_alert_log(
                            &conn,
                            result.product_id,
                            (price * 100.0) as i64,
                            get_current_time_in_ms(),
                        )?;
                    }
                    
                }
            }

            println!("{table}");
        }
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