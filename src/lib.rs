#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use crate::bot::bot::init_bot;
pub use crate::bot::error::BotError;
use crate::db::client::DbClient;
use crate::task::task::init_task;

mod bot;
mod db;
mod reddit;
mod task;
mod telegram;

embed_migrations!();

pub async fn start(
    tg_token: String,
    bot_name: String,
    database_url: String,
    author_id: String,
) -> Result<(), BotError> {
    run_migrations(&database_url);
    init_task(tg_token.clone(), database_url.clone());
    init_bot(&tg_token, &bot_name, &database_url, &author_id).await;

    Ok(())
}

fn run_migrations(database_url: &str) {
    let db_client = DbClient::new(database_url);
    embedded_migrations::run(&db_client.conn).expect("Failed to run migrations");
}
