#[macro_use]
extern crate diesel;

use log::{error, info};
use telegram_bot::{Api, ChatId, ChatRef, SendMessage};

use crate::db::client::Client as DbClient;
use crate::db::models::Subscription;
use crate::reddit::client::Client as RedditClient;

pub mod bot;
mod db;
mod reddit;
pub mod task;

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
