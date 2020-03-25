use log::info;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::bot::dialogs::Dialog;
use crate::bot::error::BotError;
use crate::db::client::DbClient;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::Message;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Display, EnumString)]
pub enum Feedback {
    Start,
    Input,
}

impl Dialog<Feedback> {
    pub fn new(user_id: String) -> Self {
        Dialog {
            command: "/feedback".to_string(),
            user_id,
            current_step: Feedback::Start,
            data: HashMap::new(),
        }
    }

    pub async fn handle_current_step(
        &mut self,
        telegram_client: &TelegramClient,
        db: &DbClient,
        author_id: &str,
        payload: &str,
    ) -> Result<(), BotError> {
        self.data.insert(self.current_step, payload.to_string());

        match self.current_step {
            Feedback::Start => {
                self.current_step = Feedback::Input;
                db.insert_or_update_dialog(&self.clone().into())?;

                telegram_client
                    .send_message(&Message {
                        chat_id: &self.user_id,
                        text: "You can write your feedback. If you want the author to get back to you, leave your email.",
                        ..Default::default()
                    })
                    .await?;
            }
            Feedback::Input => {
                let input = self.data.get(&Feedback::Input).unwrap();
                info!("received feedback from user({}): {}", &self.user_id, input);

                telegram_client
                    .send_message(&Message {
                        chat_id: author_id,
                        text: &format!("Received input from user({}):\n{}", &self.user_id, input),
                        ..Default::default()
                    })
                    .await?;

                telegram_client
                    .send_message(&Message {
                        chat_id: &self.user_id,
                        text: "Sent your feedback to the author. Thanks for the input!",
                        ..Default::default()
                    })
                    .await?;
                db.delete_dialog(&self.user_id)?;
            }
        }
        Ok(())
    }
}
