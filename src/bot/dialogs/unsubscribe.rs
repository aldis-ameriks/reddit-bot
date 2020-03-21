use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString};

use crate::bot::dialogs::Dialog;
use crate::db::models::DialogEntity;

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

    pub fn handle_current_step(&self) {
        match self.current_step {
            Unsubscribe::Start => {}
            Unsubscribe::Subreddit => {}
        }
    }
}
