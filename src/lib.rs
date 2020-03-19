#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::error::Error;

use crate::bot::bot::init_bot;
use crate::task::task::init_task;

mod bot;
mod db;
mod reddit;
mod task;
mod telegram;

pub async fn start(tg_token: String, database_url: String) -> Result<(), Box<dyn Error>> {
    init_task(&tg_token, &database_url);
    init_bot(&tg_token, &database_url).await;

    Ok(())
}
