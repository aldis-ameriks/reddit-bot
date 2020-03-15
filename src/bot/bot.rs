use std::error::Error;

use futures::StreamExt;
use log::{error, info, warn};
use telegram_bot::prelude::*;
use telegram_bot::{Api, MessageKind, UpdateKind, User};

use crate::bot::commands::{help, start, stop, subscribe, subscriptions, unsubscribe};
use crate::db::client::Client as DbClient;
use crate::reddit::client::Client as RedditClient;

pub async fn init_bot(token: &str, database_url: &str) {
    let db = DbClient::new(&database_url);
    let api = Api::new(&token);
    let reddit_client = RedditClient::new();

    let handle_stuff =
        |data: String, from: User| handle_message(data, from, &api, &db, &reddit_client);

    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        if let Ok(update) = update {
            match update.kind {
                UpdateKind::Message(message) => {
                    if let MessageKind::Text { data, .. } = message.kind {
                        if let Err(e) = handle_stuff(data, message.from).await {
                            error!("error handling message: {}", e);
                        }
                    }
                }
                UpdateKind::CallbackQuery(query) => {
                    if let Some(data) = query.data {
                        if let Err(e) = handle_stuff(data, query.from).await {
                            error!("error handling message in callback query: {}", e);
                        }
                    } else {
                        warn!("empty message in callback query");
                    }
                }
                _ => {}
            }
        }
    }
}

async fn handle_message(
    data: String,
    from: User,
    api: &Api,
    db: &DbClient,
    reddit_client: &RedditClient,
) -> Result<(), Box<dyn Error>> {
    info!(
        "received message from: {}({}), message: {}",
        &from.first_name, &from.id, data
    );

    let data = data.split(" ").collect::<Vec<&str>>();
    let command = data.get(0).unwrap_or(&"unknown");
    let payload = data.get(1).cloned();

    match command.as_ref() {
        "/start" => start(&api, &db, &from).await?,
        "/stop" => stop(&api, &db, &from).await?,
        "/subscribe" => subscribe(&api, &db, &reddit_client, &from, payload).await?,
        "/unsubscribe" => unsubscribe(&api, &db, &from, payload).await?,
        "/subscriptions" => subscriptions(&api, &db, &from).await?,
        "/help" => help(&api, &from).await?,
        _ => {
            if let Ok(last_command) = db.get_users_last_command(&from.id.to_string()) {
                if let Some(mut last_command) = last_command {
                    if last_command.command == "/subscribe" && last_command.step == 0 {
                        subscribe(&api, &db, &reddit_client, &from, Some(command)).await?;
                        last_command.step += 1;
                        db.insert_or_update_last_command(&last_command).ok();
                        return Ok(());
                    }
                }
            }
            api.send(from.text("Say what?")).await?;
        }
    }
    Ok(())
}
