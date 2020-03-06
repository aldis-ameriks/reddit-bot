use std::error::Error;

use futures::StreamExt;
use log::info;
use telegram_bot::prelude::*;
use telegram_bot::{Api, Message, MessageKind, UpdateKind};

use crate::db::client::Client as DbClient;
use crate::process_subscription;
use crate::reddit::client::Client as RedditClient;

const HELP_TEXT: &str = r#"
These are the commands I know
/start
/stop
/subscribe <subreddit>
/unsubscribe <subreddit>
/subscriptions
/help
"#;

pub async fn init_bot(token: &str, database_url: &str) -> Result<(), Box<dyn Error>> {
    let db = DbClient::new(&database_url);
    let api = Api::new(&token);
    let reddit_client = RedditClient::new();

    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = update?;
        if let UpdateKind::Message(message) = update.kind {
            if let MessageKind::Text { ref data, .. } = message.kind {
                info!(
                    "received message from: {}({}), message: {}",
                    &message.from.first_name, &message.from.id, data
                );

                let data = data.split(" ").collect::<Vec<&str>>();
                let command = data.get(0).unwrap_or(&"unknown");
                let payload = data.get(1).cloned();

                match command.as_ref() {
                    "/start" => start(&api, &message, &db).await?,
                    "/stop" => stop(&api, &message, &db).await?,
                    "/subscribe" => subscribe(&api, &reddit_client, &message, payload, &db).await?,
                    "/unsubscribe" => unsubscribe(&api, &message, payload, &db).await?,
                    "/subscriptions" => subscriptions(&api, &message, &db).await?,
                    "/help" => help(&api, &message).await?,
                    _ => {
                        api.send(message.from.text("Say what?")).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn start(
    api: &Api,
    message: &Message,
    db: &DbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = db.create_user(&message.from.id.to_string()) {
        api.send(message.from.text(HELP_TEXT)).await?;
    }
    Ok(())
}

async fn stop(
    api: &Api,
    message: &Message,
    db: &DbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = db.delete_user(&message.from.id.to_string()) {
        api.send(message.from.text("User and subscriptions deleted"))
            .await?;
    }
    Ok(())
}

async fn subscribe(
    api: &Api,
    reddit_client: &RedditClient,
    message: &Message,
    payload: Option<&str>,
    db: &DbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    if let None = payload {
        api.send(message.from.text("Missing subreddit")).await?;
        return Ok(());
    }

    let payload = payload.unwrap();

    if !reddit_client.validate_subreddit(&payload).await {
        api.send(message.from.text("Invalid subreddit")).await?;
        return Ok(());
    }

    if let Ok(subscription) = db.subscribe(&message.from.id.to_string(), &payload) {
        api.send(message.from.text(format!("Subscribed to: {}", &payload)))
            .await?;
        process_subscription(&db, &api, &reddit_client, &subscription).await;
    }

    Ok(())
}

async fn unsubscribe(
    api: &Api,
    message: &Message,
    payload: Option<&str>,
    db: &DbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    if let None = payload {
        api.send(message.from.text("Missing subreddit")).await?;
        return Ok(());
    }

    let payload = payload.unwrap();

    if let Ok(_) = db.unsubscribe(&message.from.id.to_string(), &payload) {
        api.send(
            message
                .from
                .text(format!("Unsubscribed from: {}", &payload)),
        )
        .await?;
    }

    Ok(())
}

async fn subscriptions(
    api: &Api,
    message: &Message,
    db: &DbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(res) = db.get_user_subscriptions(&message.from.id.to_string()) {
        let text = res
            .iter()
            .map(|subscription| format!("{}\n", subscription.subreddit))
            .collect::<String>();
        if let 0 = text.len() {
            api.send(message.from.text("You have no subscriptions"))
                .await?;
        } else {
            api.send(
                message
                    .from
                    .text(format!("You are currently subscribed to:\n{}", text)),
            )
            .await?;
        }
    }

    Ok(())
}

async fn help(api: &Api, message: &Message) -> Result<(), Box<dyn std::error::Error>> {
    api.send(message.from.text(HELP_TEXT)).await?;
    Ok(())
}
