use std::env;

use dotenv::dotenv;
use reddit_bot::{start, BotError};

#[tokio::main]
async fn main() -> Result<(), BotError> {
    dotenv().ok();
    env_logger::init();

    let token = env::var("TG_TOKEN").expect("missing TG_TOKEN env var");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let author_id = env::var("TG_AUTHOR").expect("missing TG_AUTHOR env var");

    start(token, database_url, author_id).await?;

    Ok(())
}
