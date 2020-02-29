use std::thread;
use std::time::Duration;

use chrono::prelude::*;
use chrono::{Datelike, Utc, Weekday};
use log::{error, info};
use telegram_bot::{Api, ChatId, ChatRef, SendMessage};
use tokio::runtime::Runtime;

use crate::db::DbClient;
use crate::reddit::fetch_posts;

pub fn init_task(token: &str, database_url: &str) {
    let api = Api::new(&token);
    let db = DbClient::new(&database_url);
    thread::spawn(move || {
        let mut rt = Runtime::new().unwrap();

        rt.block_on(async { run_task(api, db).await });
    });
}

async fn run_task(api: Api, db: DbClient) {
    loop {
        let now = Utc::now();
        if now.weekday() != Weekday::Sun || now.hour() < 12 {
            continue;
        }

        info!("processing user subscriptions");

        if let Ok(user_subscriptions) = db.get_subscriptions() {
            for user_subscription in user_subscriptions {
                if let Some(date) = user_subscription.last_sent_at {
                    if let Ok(parsed) = date.parse::<DateTime<Utc>>() {
                        if parsed.date().eq(&now.date()) {
                            continue;
                        }
                    }
                }
                if let Ok(posts) = fetch_posts(&user_subscription.subreddit).await {
                    let mut message = format!(
                        "Weekly popular posts from: \"{}\"\n\n",
                        &user_subscription.subreddit
                    );
                    for post in posts.iter() {
                        message.push_str(&post.format())
                    }

                    if let Ok(_) = api
                        .send(
                            SendMessage::new(
                                ChatRef::Id(ChatId::new(
                                    user_subscription.user_id.parse::<i64>().unwrap(),
                                )),
                                message,
                            )
                            .disable_preview(),
                        )
                        .await
                    {
                        info!("sent reddit posts");
                        if let Err(err) = db.update_last_sent(user_subscription.id) {
                            error!("failed to update last sent date: {}", err);
                        }
                    }
                } else {
                    error!("failed to fetch reddit posts");
                }
            }
        }
        thread::sleep(Duration::from_secs(10));
    }
}
