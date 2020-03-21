#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::error::Error;

use crate::bot::bot::init_bot;
use crate::db::client::DbClient;
use crate::task::task::init_task;

mod bot;
mod db;
mod reddit;
mod task;
mod telegram;

embed_migrations!();

pub async fn start(tg_token: String, database_url: String) -> Result<(), Box<dyn Error>> {
    run_migrations(&database_url);
    init_task(&tg_token, &database_url);
    init_bot(&tg_token, &database_url).await;

    Ok(())
}

fn run_migrations(database_url: &str) {
    let db_client = DbClient::new(database_url);
    embedded_migrations::run(&db_client.conn).expect("Failed to run migrations");
}
