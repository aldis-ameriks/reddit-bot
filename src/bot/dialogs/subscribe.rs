use crate::bot::dialogs::Dialog;
use crate::db::models::Dialog as DialogEntity;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString};

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

    pub fn handle_current_step(&self) {
        match self.current_step {
            Subscribe::Start => {}
            Subscribe::Subreddit => {}
            Subscribe::Weekday => {}
            Subscribe::Time => {}
        }
    }
}
