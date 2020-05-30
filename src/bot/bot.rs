use futures::StreamExt;
use log::{error, info, warn};
use telegram_bot::{Api, MessageKind, UpdateKind};

use crate::bot::commands::{
    feedback, help, send_now, start, stop, subscribe, subscriptions, unsubscribe,
};
use crate::bot::dialogs::{Dialog, Feedback, Subscribe, Unsubscribe};
use crate::bot::error::BotError;
use crate::db::client::DbClient;
use crate::reddit::client::RedditClient;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::Message;

const ERROR_TEXT: &str = r#"
Looks like I'm having a technical glitch. Something went wrong.
If the issues persist, open an issue on github (https://github.com/aldis-ameriks/reddit-bot) or you can also send feedback via /feedback command.
"#;

pub async fn init_bot(token: &str, database_url: &str, author_id: &str) {
    let db = DbClient::new(&database_url);
    let api = Api::new(&token);
    let reddit_client = RedditClient::new();
    let telegram_client = TelegramClient::new(token.to_string());

    let handle_stuff = |data: String, user_id: String| {
        handle_message(
            &db,
            &telegram_client,
            &reddit_client,
            author_id,
            data,
            user_id,
        )
    };

    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        if let Ok(update) = update {
            match update.kind {
                UpdateKind::Message(message) => {
                    if let MessageKind::Text { data, .. } = message.kind {
                        let user_id = message.from.id.to_string();
                        if let Err(e) = handle_stuff(data, user_id.clone()).await {
                            error!("error handling message: {}", e);
                            telegram_client
                                .send_message(&Message {
                                    chat_id: &user_id,
                                    text: ERROR_TEXT,
                                    ..Default::default()
                                })
                                .await
                                .ok();
                        }
                    }
                }
                UpdateKind::CallbackQuery(query) => {
                    if let Some(data) = query.data {
                        let user_id = query.from.id.to_string();
                        if let Err(e) = handle_stuff(data, user_id.clone()).await {
                            error!("error handling message in callback query: {}", e);
                            telegram_client
                                .send_message(&Message {
                                    chat_id: &user_id,
                                    text: ERROR_TEXT,
                                    ..Default::default()
                                })
                                .await
                                .ok();
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
    db: &DbClient,
    telegram_client: &TelegramClient,
    reddit_client: &RedditClient,
    author_id: &str,
    payload: String,
    user_id: String,
) -> Result<(), BotError> {
    info!("received message from: {}, message: {}", user_id, payload);

    // TODO: Extract commands as enum
    match payload.as_ref() {
        "/start" => start(&telegram_client, &db, &user_id).await?,
        "/stop" => stop(&telegram_client, &db, &user_id).await?,
        "/subscribe" => subscribe(&telegram_client, &db, &reddit_client, &user_id).await?,
        "/unsubscribe" => unsubscribe(&telegram_client, &db, &user_id).await?,
        "/subscriptions" => subscriptions(&telegram_client, &db, &user_id).await?,
        "/feedback" => feedback(&telegram_client, &db, author_id, &user_id).await?,
        "/sendnow" => send_now(&telegram_client, &db, &reddit_client, &user_id).await?,
        "/help" => help(&telegram_client, &user_id).await?,
        _ => {
            if let Ok(dialog) = db.get_users_dialog(&user_id) {
                match dialog.command.as_str() {
                    "/subscribe" => {
                        let mut dialog: Dialog<Subscribe> = Dialog::from(dialog);
                        dialog
                            .handle_current_step(&telegram_client, &db, &reddit_client, &payload)
                            .await?;
                        return Ok(());
                    }
                    "/unsubscribe" => {
                        let mut dialog: Dialog<Unsubscribe> = Dialog::from(dialog);
                        dialog
                            .handle_current_step(&telegram_client, &db, &payload)
                            .await?;
                        return Ok(());
                    }
                    "/feedback" => {
                        let mut dialog: Dialog<Feedback> = Dialog::from(dialog);
                        dialog
                            .handle_current_step(&telegram_client, &db, author_id, &payload)
                            .await?;
                        return Ok(());
                    }
                    _ => {}
                }
            }

            telegram_client
                .send_message(&Message {
                    chat_id: &user_id,
                    text: "I didn't get that. Use /help to see list of available commands.",
                    ..Default::default()
                })
                .await?;
        }
    }
    Ok(())
}
