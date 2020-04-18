use std::thread;
use std::time::Duration;

use chrono::prelude::*;
use chrono::{Datelike, Utc, Weekday};
use log::{debug, error, info};
use num::traits::FromPrimitive;
use tokio::runtime::Runtime;

use crate::db::client::DbClient;
use crate::db::models::Subscription;
use crate::reddit::client::RedditClient;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::Message;
use crate::BotError;

pub fn init_task(token: String, database_url: String) {
    let db = DbClient::new(&database_url);
    let reddit_client = RedditClient::new();
    let telegram_client = TelegramClient::new(token.to_string());

    thread::spawn(move || {
        let result = std::panic::catch_unwind(move || {
            let mut rt = Runtime::new().unwrap();

            rt.block_on(async {
                loop {
                    if let Ok(user_subscriptions) = db.get_subscriptions() {
                        for user_subscription in user_subscriptions {
                            let now = Utc::now();
                            let send_on = Weekday::from_i32(user_subscription.send_on).unwrap();
                            let send_at = user_subscription.send_at as u32;
                            if now.weekday() != send_on || now.hour() < send_at {
                                debug!(
                                    "skipping subscription - now: {}, send_on: {}, send_at: {}",
                                    now, send_on, send_at
                                );
                                continue;
                            }

                            if let Some(date) = &user_subscription.last_sent_at {
                                if let Ok(parsed) = date.parse::<DateTime<Utc>>() {
                                    if parsed.date().eq(&now.date()) {
                                        debug!("already sent today: {:?}", &user_subscription);
                                        continue;
                                    }
                                }
                            }
                            match process_subscription(
                                &db,
                                &telegram_client,
                                &reddit_client,
                                &user_subscription,
                            )
                            .await
                            {
                                Ok(_) => {
                                    info!("processed subscription: {:?}", &user_subscription);
                                }
                                Err(err) => {
                                    error!("failed to process subscription: {}", err);
                                }
                            }
                        }
                    }
                    thread::sleep(Duration::from_secs(10));
                }
            });
        });
        if let Err(_) = result {
            error!("thread panicked, recovering");
            init_task(token, database_url);
        }
    });
}

pub async fn process_subscription(
    db: &DbClient,
    telegram_client: &TelegramClient,
    reddit_client: &RedditClient,
    user_subscription: &Subscription,
) -> Result<(), BotError> {
    let posts = reddit_client
        .fetch_posts(&user_subscription.subreddit)
        .await?;

    let mut message = format!(
        "Weekly popular posts from: \"{}\"\n\n",
        &user_subscription.subreddit
    );

    for post in posts.iter() {
        message.push_str(format!("{}\n", post).as_str());
    }

    telegram_client
        .send_message(&Message {
            chat_id: &user_subscription.user_id,
            text: &message,
            disable_web_page_preview: true,
            ..Default::default()
        })
        .await?;
    db.update_last_sent(user_subscription.id)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use mockito::server_url;
    use serial_test::serial;

    use crate::db::test_helpers::setup_test_db;
    use crate::reddit::test_helpers::mock_reddit_success;
    use crate::telegram::test_helpers::mock_send_message_success;

    use super::*;

    const USER_ID: &str = "123";
    const TOKEN: &str = "token";

    #[tokio::test]
    #[serial]
    async fn process_subscription_success() {
        let url = &server_url();
        let subreddit = "rust";
        let expected_message = Message {
            chat_id: USER_ID,
            text: &format!("Weekly popular posts from: \"rust\"\n\nA half-hour to learn Rust\n{}/r/rust/comments/fbenua/a_halfhour_to_learn_rust/\n\n", url),
            disable_web_page_preview: true,
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &expected_message);
        let _m2 = mock_reddit_success(subreddit);

        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let reddit_client = RedditClient::new_with(url);
        let db_client = setup_test_db();

        let user_subscription = Subscription {
            id: 123,
            user_id: USER_ID.to_string(),
            subreddit: subreddit.to_string(),
            ..Default::default()
        };

        process_subscription(
            &db_client,
            &telegram_client,
            &reddit_client,
            &user_subscription,
        )
        .await
        .unwrap();

        _m.assert();
        _m2.assert();
    }
}
