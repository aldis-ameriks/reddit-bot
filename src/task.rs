use std::thread;
use std::time::Duration;

use chrono::prelude::*;
use chrono::{Datelike, Utc, Weekday};
use log::info;
use telegram_bot::Api;
use tokio::runtime::Runtime;

use crate::db::client::Client as DbClient;
use crate::process_subscription;
use crate::reddit::client::Client as RedditClient;

pub fn init_task(token: &str, database_url: &str) {
    let api = Api::new(&token);
    let db = DbClient::new(&database_url);
    let reddit_client = RedditClient::new();

    thread::spawn(move || {
        let mut rt = Runtime::new().unwrap();

        rt.block_on(async {
            loop {
                let now = Utc::now();
                // TODO: allow configuring per user and/or per user subscription
                if now.weekday() != Weekday::Sun || now.hour() < 12 {
                    continue;
                }

                info!("processing user subscriptions");

                if let Ok(user_subscriptions) = db.get_subscriptions() {
                    for user_subscription in user_subscriptions {
                        if let Some(date) = &user_subscription.last_sent_at {
                            if let Ok(parsed) = date.parse::<DateTime<Utc>>() {
                                if parsed.date().eq(&now.date()) {
                                    continue;
                                }
                            }
                        }
                        process_subscription(&db, &api, &reddit_client, &user_subscription).await;
                    }
                }
                thread::sleep(Duration::from_secs(10));
            }
        });
    });
}
