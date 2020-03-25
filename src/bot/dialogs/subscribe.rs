use std::collections::HashMap;

use chrono::Weekday;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;
use log::error;
use num::traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::bot::dialogs::Dialog;
use crate::bot::error::BotError;
use crate::db::client::DbClient;
use crate::reddit::client::RedditClient;
use crate::task::task::process_subscription;
use crate::telegram::client::TelegramClient;
use crate::telegram::helpers::build_inline_keyboard_markup;
use crate::telegram::types::{InlineKeyboardButton, Message, ReplyMarkup};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Display, EnumString)]
pub enum Subscribe {
    Start,
    Subreddit,
    Weekday,
    Time,
}

impl Dialog<Subscribe> {
    pub fn new(user_id: String) -> Self {
        Dialog {
            command: "/subscribe".to_string(),
            user_id: user_id.clone(),
            current_step: Subscribe::Start,
            data: HashMap::new(),
        }
    }

    pub async fn handle_current_step(
        &mut self,
        telegram_client: &TelegramClient,
        db: &DbClient,
        reddit_client: &RedditClient,
        payload: &str,
    ) -> Result<(), BotError> {
        self.data.insert(self.current_step, payload.to_string());

        match self.current_step {
            Subscribe::Start => {
                self.current_step = Subscribe::Subreddit;
                db.insert_or_update_dialog(&self.clone().into())?;
                telegram_client
                    .send_message(&Message {
                        chat_id: &self.user_id,
                        text: "Type the name of subreddit you want to subscribe to",
                        ..Default::default()
                    })
                    .await?;
            }
            Subscribe::Subreddit => {
                let subreddit = self.data.get(&Subscribe::Subreddit).unwrap();
                if !reddit_client.validate_subreddit(&subreddit).await {
                    telegram_client
                        .send_message(&Message {
                            chat_id: &self.user_id,
                            text: "Invalid subreddit, try again",
                            ..Default::default()
                        })
                        .await?;
                    return Ok(());
                }

                let buttons = (0..7)
                    .map(|weekday| InlineKeyboardButton {
                        text: format!("{}", Weekday::from_u8(weekday).unwrap()),
                        callback_data: format!("{}", weekday).clone(),
                    })
                    .collect::<Vec<InlineKeyboardButton>>();

                let markup = build_inline_keyboard_markup(buttons, 2);

                self.current_step = Subscribe::Weekday;
                db.insert_or_update_dialog(&self.clone().into())?;

                telegram_client
                    .send_message(&Message {
                        chat_id: &self.user_id,
                        text: "On which day do you want to receive the posts?",
                        reply_markup: Some(&ReplyMarkup::InlineKeyboardMarkup(markup)),
                        ..Default::default()
                    })
                    .await?;
            }
            Subscribe::Weekday => {
                let buttons = (0..24)
                    .map(|hour| InlineKeyboardButton {
                        text: format!("{}:00", hour),
                        callback_data: format!("{}", hour),
                    })
                    .collect::<Vec<InlineKeyboardButton>>();

                let markup = build_inline_keyboard_markup(buttons, 4);

                self.current_step = Subscribe::Time;
                db.insert_or_update_dialog(&self.clone().into())?;

                telegram_client
                    .send_message(&Message {
                        chat_id: &self.user_id,
                        text: "At what time? (UTC)",
                        reply_markup: Some(&ReplyMarkup::InlineKeyboardMarkup(markup)),
                        ..Default::default()
                    })
                    .await?;
            }
            Subscribe::Time => {
                let subreddit = self.data.get(&Subscribe::Subreddit).unwrap();
                let day = self
                    .data
                    .get(&Subscribe::Weekday)
                    .unwrap()
                    .parse::<i32>()
                    .unwrap_or(0);
                let time = self
                    .data
                    .get(&Subscribe::Time)
                    .unwrap()
                    .parse::<i32>()
                    .unwrap_or(12);

                match db.subscribe(&self.user_id, &subreddit, day, time) {
                    Ok(subscription) => {
                        telegram_client
                            .send_message(&Message {
                                chat_id: &self.user_id,
                                text: &format!(
                                    "Subscribed to: {}. Posts will be sent periodically on {} at around {}:00 UTC time.",
                                    &subreddit, Weekday::from_i32(day).unwrap(), time
                                ),
                                ..Default::default()
                            })
                            .await?;
                        process_subscription(&db, &telegram_client, &reddit_client, &subscription)
                            .await;
                    }
                    Err(err) => {
                        error!("err: {}", err);
                        if let DatabaseError(DatabaseErrorKind::UniqueViolation, _) = err {
                            telegram_client
                                .send_message(&Message {
                                    chat_id: &self.user_id,
                                    text: &format!("Already subscribed to {}", &subreddit),
                                    ..Default::default()
                                })
                                .await?;
                        } else {
                            telegram_client
                                .send_message(&Message {
                                    chat_id: &self.user_id,
                                    text: "Something went wrong",
                                    ..Default::default()
                                })
                                .await?;
                        }
                    }
                }
                db.delete_dialog(&self.user_id)?;
            }
        }
        Ok(())
    }
}
