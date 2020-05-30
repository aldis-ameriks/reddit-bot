use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;
use log::{error, info, warn};
use std::thread;
use std::time::Duration;

use crate::bot::dialogs::{Dialog, Feedback, Subscribe, Unsubscribe};
use crate::bot::error::BotError;
use crate::db::client::DbClient;
use crate::reddit::client::RedditClient;
use crate::task::task::process_subscription;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::Message;

const HELP_TEXT: &str = r#"
You can send me these commands:
/start
/stop
/subscribe
/unsubscribe
/subscriptions
/sendnow
/feedback
/help

Bot is open source and available here https://github.com/aldis-ameriks/reddit-bot. If you encounter any issues feel free to open an issue.
Or you can also send feedback via /feedback command.
"#;

pub async fn start(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), BotError> {
    match db.create_user(user_id) {
        Ok(_) => {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: HELP_TEXT,
                    ..Default::default()
                })
                .await?;
            Ok(())
        }
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: HELP_TEXT,
                    ..Default::default()
                })
                .await?;
            Ok(())
        }
        Err(err) => Err(BotError::DatabaseError(err)),
    }
}

pub async fn stop(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), BotError> {
    db.delete_user(user_id)?;
    telegram_client
        .send_message(&Message {
            chat_id: user_id,
            text: "User and subscriptions deleted",
            ..Default::default()
        })
        .await?;

    Ok(())
}

pub async fn subscribe(
    telegram_client: &TelegramClient,
    db: &DbClient,
    reddit_client: &RedditClient,
    user_id: &str,
) -> Result<(), BotError> {
    match Dialog::<Subscribe>::new(user_id.to_string())
        .handle_current_step(&telegram_client, &db, &reddit_client, "")
        .await
    {
        Ok(_) => Ok(()),
        Err(BotError::DatabaseError(err)) => {
            if let DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) = err {
                warn!("subscribe was initiated without user");
                telegram_client
                    .send_message(&Message {
                        chat_id: user_id,
                        text: "You need to call /start before setting up subscriptions",
                        ..Default::default()
                    })
                    .await?;
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}

pub async fn unsubscribe(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), BotError> {
    Dialog::<Unsubscribe>::new(user_id.to_string())
        .handle_current_step(&telegram_client, &db, "")
        .await
}

pub async fn subscriptions(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), BotError> {
    let subscriptions = db.get_user_subscriptions(user_id)?;
    if subscriptions.is_empty() {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: "You haven't subscribed to anything yet. Subscribe using /subscribe command.",
                ..Default::default()
            })
            .await?;
    } else {
        let text = subscriptions
            .iter()
            .map(|subscription| format!("{}\n", subscription.subreddit))
            .collect::<String>();
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: &format!("You are currently subscribed to:\n{}", text),
                ..Default::default()
            })
            .await?;
    }

    Ok(())
}

pub async fn feedback(
    telegram_client: &TelegramClient,
    db: &DbClient,
    author_id: &str,
    user_id: &str,
) -> Result<(), BotError> {
    match Dialog::<Feedback>::new(user_id.to_string())
        .handle_current_step(&telegram_client, &db, author_id, "")
        .await
    {
        Ok(_) => Ok(()),
        Err(BotError::DatabaseError(err)) => {
            if let DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) = err {
                warn!("feedback was initiated without user");
                telegram_client
                    .send_message(&Message {
                        chat_id: user_id,
                        text: "You need to call /start before interacting with me",
                        ..Default::default()
                    })
                    .await?;
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}

pub async fn send_now(
    telegram_client: &TelegramClient,
    db: &DbClient,
    reddit_client: &RedditClient,
    user_id: &str,
) -> Result<(), BotError> {
    let subscriptions = db.get_user_subscriptions(user_id)?;

    if subscriptions.len() == 0 {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: "You haven't subscribed to anything yet. Subscribe using /subscribe command.",
                ..Default::default()
            })
            .await?;
    }

    for subscription in subscriptions {
        match process_subscription(db, telegram_client, reddit_client, &subscription).await {
            Ok(_) => {
                info!("processed subscription: {:?}", &subscription);
            }
            Err(err) => {
                error!("failed to process subscription: {}", err);
            }
        }
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}

pub async fn help(telegram_client: &TelegramClient, user_id: &str) -> Result<(), BotError> {
    telegram_client
        .send_message(&Message {
            chat_id: user_id,
            text: HELP_TEXT,
            ..Default::default()
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use mockito::server_url;
    use serial_test::serial;

    use crate::db::test_helpers::{setup_test_db, setup_test_db_with};
    use crate::telegram::test_helpers::{mock_send_message_not_called, mock_send_message_success};

    use super::*;
    use crate::reddit::test_helpers::mock_reddit_success;

    const TOKEN: &str = "token";
    const USER_ID: &str = "123";

    #[tokio::test]
    #[serial]
    async fn start_success() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: HELP_TEXT,
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let db_client = setup_test_db();

        start(&telegram_client, &db_client, USER_ID).await.unwrap();
        _m.assert();

        let users = db_client.get_users().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, USER_ID);
    }

    #[tokio::test]
    #[serial]
    async fn start_existing_user() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: HELP_TEXT,
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let db_client = setup_test_db();
        db_client.create_user(USER_ID).unwrap();

        let users = db_client.get_users().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, USER_ID);

        start(&telegram_client, &db_client, USER_ID).await.unwrap();
        _m.assert();

        let users = db_client.get_users().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, USER_ID);
    }

    #[tokio::test]
    #[serial]
    async fn start_error() {
        let url = &server_url();
        let _m = mock_send_message_not_called(TOKEN);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let db_client = setup_test_db_with(false);

        let result = start(&telegram_client, &db_client, USER_ID).await;
        assert!(result.is_err());
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn stop_success() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "User and subscriptions deleted",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let db_client = setup_test_db();
        db_client.create_user(USER_ID).unwrap();
        let users = db_client.get_users().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, USER_ID);

        stop(&telegram_client, &db_client, USER_ID).await.unwrap();

        let users = db_client.get_users().unwrap();
        assert_eq!(users.len(), 0);
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn stop_error() {
        let url = &server_url();
        let _m = mock_send_message_not_called(TOKEN);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let db_client = setup_test_db_with(false);

        let result = stop(&telegram_client, &db_client, USER_ID).await;
        assert!(result.is_err());
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn subscribe_success() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "Type the name of subreddit you want to subscribe to.\nMultiple subreddits are allowed, separated by whitespace.",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let db_client = setup_test_db();
        db_client.create_user(USER_ID).unwrap();
        let reddit_client = RedditClient::new_with(url);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        subscribe(&telegram_client, &db_client, &reddit_client, USER_ID)
            .await
            .unwrap();
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn subscribe_without_user() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "You need to call /start before setting up subscriptions",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let db_client = setup_test_db();
        let reddit_client = RedditClient::new_with(url);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        let users = db_client.get_users().unwrap();
        assert_eq!(users.len(), 0);

        subscribe(&telegram_client, &db_client, &reddit_client, USER_ID)
            .await
            .unwrap();

        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn unsubscribe_without_user() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "You have no subscriptions to unsubscribe from",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let db_client = setup_test_db();
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        let users = db_client.get_users().unwrap();
        assert_eq!(users.len(), 0);

        unsubscribe(&telegram_client, &db_client, USER_ID)
            .await
            .unwrap();

        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn subscriptions_success() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "You are currently subscribed to:\nrust\n",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let db_client = setup_test_db();
        db_client.create_user(USER_ID).unwrap();
        db_client.subscribe(USER_ID, "rust", 1, 1).unwrap();
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        subscriptions(&telegram_client, &db_client, USER_ID)
            .await
            .unwrap();
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn subscriptions_no_subscriptions() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "You haven't subscribed to anything yet. Subscribe using /subscribe command.",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let db_client = setup_test_db();
        db_client.create_user(USER_ID).unwrap();
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        subscriptions(&telegram_client, &db_client, USER_ID)
            .await
            .unwrap();
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn subscriptions_error() {
        let url = &server_url();
        let _m = mock_send_message_not_called(TOKEN);
        let db_client = setup_test_db_with(false);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        let result = subscriptions(&telegram_client, &db_client, USER_ID).await;
        assert!(result.is_err());
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn feedback_success() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "You can write your feedback. If you want the author to get back to you, leave your email.",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let db_client = setup_test_db();
        db_client.create_user(USER_ID).unwrap();
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        feedback(&telegram_client, &db_client, "", USER_ID)
            .await
            .unwrap();

        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn feedback_without_user() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "You need to call /start before interacting with me",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let db_client = setup_test_db();
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        let users = db_client.get_users().unwrap();
        assert_eq!(users.len(), 0);

        feedback(&telegram_client, &db_client, "", USER_ID)
            .await
            .unwrap();
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn send_now_success() {
        let url = &server_url();
        let subreddit = "rust";
        let message = Message {
            chat_id: USER_ID,
            text: &format!("Weekly popular posts from: \"rust\"\n\nA half-hour to learn Rust\n{}/r/rust/comments/fbenua/a_halfhour_to_learn_rust/\n\n", url),
            disable_web_page_preview: true,
            ..Default::default()
        };
        let _m1 = mock_send_message_success(TOKEN, &message);
        let _m2 = mock_reddit_success(subreddit);
        let db_client = setup_test_db();
        db_client.create_user(USER_ID).unwrap();
        db_client.subscribe(USER_ID, subreddit, 1, 1).unwrap();
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let reddit_client = RedditClient::new_with(&url);

        send_now(&telegram_client, &db_client, &reddit_client, USER_ID)
            .await
            .unwrap();
        _m1.assert();
        _m2.assert();
    }

    #[tokio::test]
    #[serial]
    async fn send_now_no_subscriptions() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: "You haven't subscribed to anything yet. Subscribe using /subscribe command.",
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let db_client = setup_test_db();
        db_client.create_user(USER_ID).unwrap();
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let reddit_client = RedditClient::new_with(&url);

        send_now(&telegram_client, &db_client, &reddit_client, USER_ID)
            .await
            .unwrap();
        _m.assert();
    }

    #[tokio::test]
    #[serial]
    async fn help_success() {
        let url = &server_url();
        let message = Message {
            chat_id: USER_ID,
            text: HELP_TEXT,
            ..Default::default()
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        help(&telegram_client, USER_ID).await.unwrap();
        _m.assert();
    }
}
