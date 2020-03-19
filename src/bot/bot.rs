use std::error::Error;

use chrono::Weekday;
use futures::StreamExt;
use log::{error, info, warn};
use num::traits::FromPrimitive;
use telegram_bot::{Api, MessageKind, UpdateKind};

use crate::bot::commands::{help, start, stop, subscribe, subscriptions, unsubscribe};
use crate::db::client::Client as DbClient;
use crate::reddit::client::Client as RedditClient;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyMarkup};

pub async fn init_bot(token: &str, database_url: &str) {
    let db = DbClient::new(&database_url);
    let api = Api::new(&token);
    let reddit_client = RedditClient::new();
    let telegram_client = TelegramClient::new(token.to_string());

    let handle_stuff = |data: String, user_id: String| {
        handle_message(&db, &telegram_client, &reddit_client, data, user_id)
    };

    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        if let Ok(update) = update {
            match update.kind {
                UpdateKind::Message(message) => {
                    if let MessageKind::Text { data, .. } = message.kind {
                        let user_id = message.from.id.to_string();
                        if let Err(e) = handle_stuff(data, user_id).await {
                            error!("error handling message: {}", e);
                        }
                    }
                }
                UpdateKind::CallbackQuery(query) => {
                    if let Some(data) = query.data {
                        let user_id = query.from.id.to_string();
                        if let Err(e) = handle_stuff(data, user_id).await {
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
    db: &DbClient,
    telegram_client: &TelegramClient,
    reddit_client: &RedditClient,
    data: String,
    user_id: String,
) -> Result<(), Box<dyn Error>> {
    info!("received message from: {}, message: {}", user_id, data);

    match data.as_ref() {
        "/start" => start(&telegram_client, &db, &user_id).await?,
        "/stop" => stop(&telegram_client, &db, &user_id).await?,
        "/subscribe" => {
            subscribe(
                &telegram_client,
                &db,
                &reddit_client,
                &user_id,
                None,
                None,
                None,
            )
            .await?
        }
        "/unsubscribe" => unsubscribe(&telegram_client, &db, &user_id, None).await?,
        "/subscriptions" => subscriptions(&telegram_client, &db, &user_id).await?,
        "/help" => help(&telegram_client, &user_id).await?,
        _ => {
            if let Ok(last_command) = db.get_users_last_command(&user_id) {
                if let Some(mut last_command) = last_command {
                    match last_command.command.as_str() {
                        // TODO: encapsulate step logic inside dialog struct
                        "/subscribe" => {
                            match last_command.step {
                                0 => {
                                    // TODO: validate subreddit
                                    // TODO: allow specifying multiple subreddits
                                    // TODO: extract helper function for building inline options
                                    let buttons = (0..7)
                                        .map(|weekday| InlineKeyboardButton {
                                            text: format!("{}", Weekday::from_u8(weekday).unwrap()),
                                            callback_data: format!("{}", weekday).clone(),
                                        })
                                        .collect::<Vec<InlineKeyboardButton>>();
                                    let mut row: Vec<InlineKeyboardButton> = vec![];
                                    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];
                                    let mut buttons_iterator = buttons.into_iter();
                                    while let Some(button) = buttons_iterator.next() {
                                        row.push(button);
                                        if row.len() == 2 {
                                            rows.push(row.clone());
                                            row = vec![];
                                        }
                                    }

                                    if row.len() > 0 {
                                        rows.push(row);
                                    }

                                    let markup = InlineKeyboardMarkup {
                                        inline_keyboard: rows,
                                    };

                                    telegram_client
                                        .send_message(&Message {
                                            chat_id: &user_id,
                                            text: "On which day do you want to receive the posts?",
                                            reply_markup: Some(&ReplyMarkup::InlineKeyboardMarkup(
                                                markup,
                                            )),
                                            ..Default::default()
                                        })
                                        .await?;

                                    last_command.step += 1;
                                    last_command.data = data;
                                    db.insert_or_update_last_command(&last_command).ok();
                                }
                                1 => {
                                    // TODO: extract helper function for building inline options
                                    let buttons = (0..24)
                                        .map(|hour| InlineKeyboardButton {
                                            text: format!("{}:00", hour),
                                            callback_data: format!("{}", hour),
                                        })
                                        .collect::<Vec<InlineKeyboardButton>>();

                                    let mut row: Vec<InlineKeyboardButton> = vec![];
                                    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];
                                    let mut buttons_iterator = buttons.into_iter();
                                    while let Some(button) = buttons_iterator.next() {
                                        row.push(button);
                                        if row.len() == 3 {
                                            rows.push(row.clone());
                                            row = vec![];
                                        }
                                    }

                                    if row.len() > 0 {
                                        rows.push(row);
                                    }

                                    let markup = InlineKeyboardMarkup {
                                        inline_keyboard: rows,
                                    };

                                    telegram_client
                                        .send_message(&Message {
                                            chat_id: &user_id,
                                            text: "At what time? (UTC)",
                                            reply_markup: Some(&ReplyMarkup::InlineKeyboardMarkup(
                                                markup,
                                            )),
                                            ..Default::default()
                                        })
                                        .await?;

                                    last_command.step += 1;
                                    last_command.data = format!("{};{}", last_command.data, data);
                                    db.insert_or_update_last_command(&last_command).ok();
                                }
                                2 => {
                                    let prev_data =
                                        last_command.data.split(";").collect::<Vec<&str>>();
                                    let subreddit = prev_data.get(0).unwrap();
                                    let day = prev_data.get(1).unwrap().parse::<i32>().unwrap_or(0);
                                    let time = data.parse::<i32>().unwrap_or(12);

                                    subscribe(
                                        &telegram_client,
                                        &db,
                                        &reddit_client,
                                        &user_id,
                                        Some(&subreddit),
                                        Some(day),
                                        Some(time),
                                    )
                                    .await?;
                                }
                                _ => {}
                            }
                            return Ok(());
                        }
                        "/unsubscribe" => {
                            if last_command.step == 0 {
                                unsubscribe(&telegram_client, &db, &user_id, Some(&data)).await?;
                                last_command.step += 1;
                                db.insert_or_update_last_command(&last_command).ok();
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                }
            }
            telegram_client
                .send_message(&Message {
                    chat_id: &user_id,
                    text: "Say what?",
                    ..Default::default()
                })
                .await?;
        }
    }
    Ok(())
}
