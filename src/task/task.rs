use std::thread;
use std::time::Duration;

use chrono::prelude::*;
use chrono::{Datelike, Utc, Weekday};
use log::{error, info};
use num::traits::FromPrimitive;
use telegram_bot::{Api, ChatId, ChatRef, SendMessage};
use tokio::runtime::Runtime;

use crate::db::client::Client as DbClient;
use crate::db::models::Subscription;
use crate::reddit::client::Client as RedditClient;

pub fn init_task(token: &str, database_url: &str) {
    let api = Api::new(&token);
    let db = DbClient::new(&database_url);
    let reddit_client = RedditClient::new();

    thread::spawn(move || {
        let mut rt = Runtime::new().unwrap();

        rt.block_on(async {
            loop {
                if let Ok(user_subscriptions) = db.get_subscriptions() {
                    for user_subscription in user_subscriptions {
                        let now = Utc::now();
                        if now.weekday() != Weekday::from_i32(user_subscription.send_on).unwrap()
                            || now.hour() < user_subscription.send_at as u32
                        {
                            continue;
                        }

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

pub async fn process_subscription(
    db: &DbClient,
    api: &Api,
    reddit_client: &RedditClient,
    user_subscription: &Subscription,
) {
    if let Ok(posts) = reddit_client
        .fetch_posts(&user_subscription.subreddit)
        .await
    {
        let mut message = format!(
            "Weekly popular posts from: \"{}\"\n\n",
            &user_subscription.subreddit
        );
        for post in posts.iter() {
            message.push_str(format!("{}\n", post).as_str());
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
