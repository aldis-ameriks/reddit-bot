use chrono::Weekday;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;
use log::error;
use num::traits::FromPrimitive;

use crate::db::client::DbClient;
use crate::db::models::DialogEntity;
use crate::reddit::client::RedditClient;
use crate::task::task::process_subscription;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyMarkup};

const HELP_TEXT: &str = r#"
You can send me these commands:
/start
/stop
/subscribe
/unsubscribe
/subscriptions
/help
"#;

pub async fn start(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = db.create_user(user_id) {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: HELP_TEXT,
                ..Default::default()
            })
            .await?;
    }
    Ok(())
}

pub async fn stop(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = db.delete_user(user_id) {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: "User and subscriptions deleted",
                ..Default::default()
            })
            .await?;
    }
    Ok(())
}

pub async fn subscribe(
    telegram_client: &TelegramClient,
    db: &DbClient,
    reddit_client: &RedditClient,
    user_id: &str,
    subreddit: Option<&str>,
    send_on: Option<i32>,
    send_at: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    if subreddit.is_none() {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: "Type the name of subreddit you want to subscribe to.",
                ..Default::default()
            })
            .await?;

        let command = DialogEntity {
            user_id: user_id.to_string(),
            command: "/subscribe".to_string(),
            step: "Start".to_string(),
            data: "".to_string(),
        };

        db.insert_or_update_dialog(&command).ok();

        return Ok(());
    }

    if send_on.is_none() || send_at.is_none() {
        return Ok(());
    }

    let send_on = send_on.unwrap();
    let send_at = send_at.unwrap();
    let subreddit = subreddit.unwrap();

    if !reddit_client.validate_subreddit(&subreddit).await {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: "Invalid subreddit",
                ..Default::default()
            })
            .await?;
        // TODO: return error
        return Ok(());
    }

    match db.subscribe(user_id, &subreddit, send_on, send_at) {
        Ok(subscription) => {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: &format!(
                        "Subscribed to: {}. Posts will be sent periodically on {} at around {}:00 UTC time.",
                        &subreddit, Weekday::from_i32(send_on).unwrap(), send_at
                    ),
                    ..Default::default()
                })
                .await?;
            process_subscription(&db, &telegram_client, &reddit_client, &subscription).await;
        }
        Err(err) => {
            error!("err: {}", err);
            if let DatabaseError(DatabaseErrorKind::UniqueViolation, _) = err {
                telegram_client
                    .send_message(&Message {
                        chat_id: user_id,
                        text: &format!("Already subscribed to {}", &subreddit),
                        ..Default::default()
                    })
                    .await?;
            } else {
                telegram_client
                    .send_message(&Message {
                        chat_id: user_id,
                        text: "Something went wrong",
                        ..Default::default()
                    })
                    .await?;
            }
        }
    }
    Ok(())
}

pub async fn unsubscribe(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
    data: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let None = data {
        if let Ok(res) = db.get_user_subscriptions(user_id) {
            let buttons = res
                .iter()
                .map(|subscription| InlineKeyboardButton {
                    text: subscription.subreddit.clone(),
                    callback_data: subscription.subreddit.clone(),
                })
                .collect::<Vec<InlineKeyboardButton>>();

            let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];
            let mut row: Vec<InlineKeyboardButton> = vec![];
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
                    chat_id: user_id,
                    text: "Select subreddit",
                    reply_markup: Some(&ReplyMarkup::InlineKeyboardMarkup(markup)),
                    ..Default::default()
                })
                .await?;

            let command = DialogEntity {
                user_id: user_id.to_string(),
                command: "/unsubscribe".to_string(),
                step: "Start".to_string(),
                data: "".to_string(),
            };
            db.insert_or_update_dialog(&command).ok();
        }
        return Ok(());
    }

    let data = data.unwrap();
    if let Ok(_) = db.unsubscribe(user_id, &data) {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: &format!("Unsubscribed from: {}", &data),
                ..Default::default()
            })
            .await?;
    }

    Ok(())
}

pub async fn subscriptions(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(res) = db.get_user_subscriptions(user_id) {
        let text = res
            .iter()
            .map(|subscription| format!("{}\n", subscription.subreddit))
            .collect::<String>();
        if let 0 = text.len() {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: "You have no subscriptions",
                    ..Default::default()
                })
                .await?;
        } else {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: &format!("You are currently subscribed to:\n{}", text),
                    ..Default::default()
                })
                .await?;
        }
    }

    Ok(())
}

pub async fn help(
    telegram_client: &TelegramClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    telegram_client
        .send_message(&Message {
            chat_id: user_id,
            text: HELP_TEXT,
            ..Default::default()
        })
        .await?;
    Ok(())
}
