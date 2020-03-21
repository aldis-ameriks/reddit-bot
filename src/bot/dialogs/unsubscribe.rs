use std::collections::HashMap;
use std::error::Error;

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::bot::dialogs::Dialog;
use crate::db::client::DbClient;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyMarkup};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Display, EnumString)]
pub enum Unsubscribe {
    Start,
    Subreddit,
}

impl Dialog<Unsubscribe> {
    pub fn new(user_id: String) -> Self {
        Dialog {
            command: "/unsubscribe".to_string(),
            user_id,
            current_step: Unsubscribe::Start,
            data: HashMap::new(),
        }
    }

    pub async fn handle_current_step(
        &mut self,
        telegram_client: &TelegramClient,
        db: &DbClient,
        payload: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.data.insert(self.current_step, payload.to_string());

        match self.current_step {
            Unsubscribe::Start => {
                if let Ok(res) = db.get_user_subscriptions(&self.user_id) {
                    if res.is_empty() {
                        telegram_client
                            .send_message(&Message {
                                chat_id: &self.user_id,
                                text: "You have no subscriptions to unsubscribe from",
                                ..Default::default()
                            })
                            .await?;
                        return Ok(());
                    }

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
                            chat_id: &self.user_id,
                            text: "Select subreddit",
                            reply_markup: Some(&ReplyMarkup::InlineKeyboardMarkup(markup)),
                            ..Default::default()
                        })
                        .await?;

                    self.current_step = Unsubscribe::Subreddit;
                    db.insert_or_update_dialog(&self.clone().into()).ok();
                }
            }
            Unsubscribe::Subreddit => {
                let subreddit = self.data.get(&Unsubscribe::Subreddit).unwrap();
                if let Ok(_) = db.unsubscribe(&self.user_id, &subreddit) {
                    telegram_client
                        .send_message(&Message {
                            chat_id: &self.user_id,
                            text: &format!("Unsubscribed from: {}", &payload),
                            ..Default::default()
                        })
                        .await?;
                }
            }
        }
        Ok(())
    }
}
