use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use clap::{Parser, Subcommand};
use comfy_table::*;

mod scraper;
mod product;
mod database;
mod telegram;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
            let current_products_db = database::get_products(&conn)?;

            table
                .set_header(vec!["Product", "Current Price", "Target Price", "Status"]);

            for product_db in current_products_db {
                match scraper::scrape_product(&product_db.url).await {
                    Ok(product) => {
                        database::add_price_entry(&conn, product_db.id, product.value_in_cents(), get_current_time_in_ms())?;

                        let mut status = Cell::new("WAIT").fg(Color::Yellow);

                        if product.value_in_cents() <= product_db.threshold_in_cents() {
                            status = Cell::new("BUY").fg(Color::Green);

                            match database::get_last_alert(&conn, product_db.id) {
                                Ok(result_last_alert) => {
                                    match result_last_alert {
                                        Some(last_alert) => {
                                            if last_alert.0 > product.value_in_cents() || get_current_time_in_ms() - last_alert.1 > (1000 * 60 * 60 * 24 * 7) {
                                                telegram::send_message(&format!(
                                                    "{}\nAktueller Preis: {:.2}\nZielpreis: {:.2}",
                                                    product.name(),
                                                    product.value(),
                                                    product_db.threshold
                                                )).await?;
                                                database::add_alert_log(&conn, product_db.id, product.value_in_cents(), get_current_time_in_ms())?;
                                            }
                                        }
                                        None => {
                                            telegram::send_message(&format!(
                                                "{}\nAktueller Preis: {:.2}\nZielpreis: {:.2}",
                                                product.name(),
                                                product.value(),
                                                product_db.threshold
                                            )).await?;
                                            database::add_alert_log(&conn, product_db.id, product.value_in_cents(), get_current_time_in_ms())?;
                                        }
                                    }
                                }
                                Err(err) => {
                                    eprintln!("{}", err);
                                    continue;
                                }
                            }
                        }

                        table.add_row(vec![
                            Cell::new(product.name()), 
                            Cell::new(&format!("{:.2}", product.value())), 
                            Cell::new(&format!("{:.2}", product_db.threshold)), 
                            status]);
                    }
                    Err(_) => {
                        table.add_row(vec![
                            Cell::new(product_db.url), 
                            Cell::new("-"), 
                            Cell::new(&format!("{:.2}", product_db.threshold)), 
                            Cell::new("ERROR").fg(Color::Red)]);
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