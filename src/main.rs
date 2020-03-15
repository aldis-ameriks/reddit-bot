use std::env;

use log::Level;

use dotenv::dotenv;
use reddit_bot::start;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    simple_logger::init_with_level(Level::Info).expect("Failed to init logger");
    let token = env::var("TG_TOKEN").expect("Missing TG_TOKEN env var");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    start(token, database_url).await?;

    Ok(())
}
