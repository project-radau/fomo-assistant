use std::time::{SystemTime, UNIX_EPOCH};
use clap::{Parser, Subcommand};

mod scraper;
mod product;
mod database;

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
    List,
    Check
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = database::open_database()?;

    let args = Cli::parse();

    match args.command {
        Commands::Add { url, threshold } =>  {
            database::add_product(&conn, &url, threshold)?;
        },
        Commands::List => {
            let items = database::get_products(&conn)?;
            for item in items {
                println!("{}", item);
            }
        },
        Commands::Check => {
            let current_products_db = database::get_products(&conn)?;

            for product_db in current_products_db {
                match scraper::scrape_product(&product_db.url).await {
                    Ok(product) => {
                        database::add_price_entry(&conn, product_db.id, product.value_in_cents(), get_current_time_in_ms())?;

                        //message
                        if product.value_in_cents() <= product_db.threshold_in_cents() {
                            println!("BUY BUY BUY");
                            println!("===========");
                            println!("{0} costs {1}", product.name(), product.value());
                            println!("===========");
                            println!("BUY BUY BUY");
                        }

                        println!("{}", product);
                    }
                    Err(error) => {
                        eprintln!("Scraping fehlgeschlagen: {error}");
                    }
                }
            }
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

