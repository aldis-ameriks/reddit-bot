use std::collections::HashMap;

use chrono::Weekday;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;
use log::error;
use num::traits::FromPrimitive;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::iter::FromIterator;
use strum_macros::{Display, EnumString};

use crate::bot::dialogs::Dialog;
use crate::bot::error::BotError;
use crate::db::client::DbClient;
use crate::reddit::client::RedditClient;
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

fn parse_subreddits(subreddits: &str) -> Vec<String> {
    let result = subreddits
        .replace("r/", "")
        .replace("\n", " ")
        .trim()
        .to_string();
    let re = Regex::new(r"\s\s+").unwrap();
    let result = re.replace_all(&result, " ").to_string();
    let mut result = Vec::from_iter(result.split(' ').map(String::from));
    result.sort();
    result.dedup();
    result
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
                        text: "Type the name of subreddit you want to subscribe to.\nMultiple subreddits are allowed, separated by whitespace or newline.",
                        ..Default::default()
                    })
                    .await?;
            }
            Subscribe::Subreddit => {
                let subreddits = self.data.get(&Subscribe::Subreddit).unwrap();
                let subreddits = parse_subreddits(subreddits);

                for subreddit in subreddits {
                    if !reddit_client.validate_subreddit(&subreddit).await {
                        telegram_client
                            .send_message(&Message {
                                chat_id: &self.user_id,
                                text: &format!("Invalid subreddit - {}, try again", subreddit),
                                ..Default::default()
                            })
                            .await?;
                        return Ok(());
                    }
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
                let subreddits = self
                    .data
                    .get(&Subscribe::Subreddit)
                    .unwrap()
                    .replace("r/", "");
                let subreddits: Vec<&str> = subreddits.split(" ").collect();
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

                for subreddit in &subreddits {
                    match db.subscribe(&self.user_id, &subreddit, day, time) {
                        Ok(_) => {
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
                }

                telegram_client
                    .send_message(&Message {
                        chat_id: &self.user_id,
                        text:
                            "You can use /sendnow to get posts now from all of your subscriptions.",
                        ..Default::default()
                    })
                    .await?;
                db.delete_dialog(&self.user_id)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::bot::dialogs::subscribe::parse_subreddits;

    #[test]
    fn test_parse_subreddits() {
        let input = "aaa bbb ccc";
        let result = parse_subreddits(input);
        assert_eq!(result, ["aaa", "bbb", "ccc"]);

        let input = "ccc xxx aaa bbb";
        let result = parse_subreddits(input);
        assert_eq!(result, ["aaa", "bbb", "ccc", "xxx"]);

        let input = "aaa bbb bbb ccc bbb";
        let result = parse_subreddits(input);
        assert_eq!(result, ["aaa", "bbb", "ccc"]);

        let input = "\n\n  \n aaa\n\n bbb\n  bbb\n\n \n  ccc bbb\n \n";
        let result = parse_subreddits(input);
        assert_eq!(result, ["aaa", "bbb", "ccc"]);

        let input = "aaa\nbbb\nccc\n";
        let result = parse_subreddits(input);
        assert_eq!(result, ["aaa", "bbb", "ccc"]);

        let input = "aaa \nbbb \nccc \n";
        let result = parse_subreddits(input);
        assert_eq!(result, ["aaa", "bbb", "ccc"]);

        let input = "\n\n  \n r/aaa\n\n r/bbb\n  bbb\n\n \n  r/ccc bbb\n \n";
        let result = parse_subreddits(input);
        assert_eq!(result, ["aaa", "bbb", "ccc"]);
    }
}
