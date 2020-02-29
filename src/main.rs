#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use dotenv::dotenv;
use futures::StreamExt;
use log::Level;
use log::{error, info};
use std::env;
use telegram_bot::*;

use db::DbClient;
use reddit::fetch_posts;

mod db;
mod models;
mod reddit;
mod schema;
mod telegram;

const HELP_TEXT: &str = r#"
These are the commands I know
/start
/stop
/subscribe <subreddit>
/unsubscribe <subreddit>
/subscriptions
/help
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().unwrap();
    simple_logger::init_with_level(Level::Info).expect("Failed to init logger");
    let token = env::var("TG_TOKEN").expect("Missing TG_TOKEN env var");
    let chat_id = env::var("TG_CHAT_ID").expect("Missing TG_CHAT_ID env var");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = DbClient::new(&database_url);

    // let subreddits = ["rust", "arduino", "Whatcouldgowrong"];
    // for subreddit in subreddits.iter() {
    //     let posts = fetch_posts(subreddit).await?;
    //     for post in posts.iter() {
    //         telegram_client.send_message(&post.format()).await?;
    //     }
    // }

    let api = Api::new(&token);

    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = update?;
        if let UpdateKind::Message(message) = update.kind {
            if let MessageKind::Text { ref data, .. } = message.kind {
                println!("<{}>: {}", &message.from.first_name, data);

                let data = data.split(" ").collect::<Vec<&str>>();
                let command = data.get(0).unwrap_or(&"unknown");
                let payload = data.get(1).cloned();

                match command.as_ref() {
                    "/start" => start(&api, &message, &db).await?,
                    "/stop" => stop(&api, &message, &db).await?,
                    "/subscribe" => subscribe(&api, &message, payload, &db).await?,
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
    message: &Message,
    payload: Option<&str>,
    db: &DbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    if let None = payload {
        api.send(message.from.text("Missing subreddit")).await?;
        return Ok(());
    }

    let payload = payload.unwrap();

    if !validate_subreddit(&payload).await {
        api.send(message.from.text("Invalid subreddit")).await?;
        return Ok(());
    }

    if let Ok(_) = db.subscribe(&message.from.id.to_string(), &payload) {
        api.send(message.from.text(format!("Subscribed to: {}", &payload)))
            .await?;
    }

    Ok(())
}

async fn unsubscribe(
    api: &Api,
    message: &Message,
    payload: Option<&str>,
    db: &DbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(value) = payload {
        if let Ok(_) = db.unsubscribe(&message.from.id.to_string(), &value) {
            api.send(message.from.text(format!("Unsubscribed from: {}", &value))).await?;
        }
    } else {
        api.send(message.from.text("Missing subreddit")).await?;
    }

    Ok(())
}

async fn subscriptions(
    api: &Api,
    message: &Message,
    db: &DbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(res) = db.get_subscriptions() {
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
