use std::env;

use log::{warn, Level};

use dotenv::dotenv;
use reddit_bot::{start, BotError};

#[tokio::main]
async fn main() -> Result<(), BotError> {
    simple_logger::init_with_level(Level::Info).expect("failed to init logger");

    if let Err(err) = dotenv() {
        warn!("failed to load .env file: {}", err);
    }

    let token = env::var("TG_TOKEN").expect("missing TG_TOKEN env var");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    start(token, database_url).await?;

    Ok(())
}
