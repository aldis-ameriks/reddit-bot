use crate::bot::dialogs::{Dialog, Subscribe, Unsubscribe};
use crate::db::client::DbClient;
use crate::reddit::client::RedditClient;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::Message;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;

const HELP_TEXT: &str = r#"
You can send me these commands:
/start
/stop
/subscribe
/unsubscribe
/subscriptions
/help
"#;

const ERROR_TEXT: &str = "Looks like I'm having a technical glitch. Something went wrong.";

pub async fn start(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match db.create_user(user_id) {
        Ok(_) => {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: HELP_TEXT,
                    ..Default::default()
                })
                .await?;
        }
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: HELP_TEXT,
                    ..Default::default()
                })
                .await?;
        }
        Err(_) => {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: ERROR_TEXT,
                    ..Default::default()
                })
                .await?;
        }
    }
    Ok(())
}

pub async fn stop(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = db.delete_user(user_id) {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: "User and subscriptions deleted",
                ..Default::default()
            })
            .await?;
    }
    Ok(())
}

pub async fn subscribe(
    telegram_client: &TelegramClient,
    db: &DbClient,
    reddit_client: &RedditClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Dialog::<Subscribe>::new(user_id.to_string())
        .handle_current_step(&telegram_client, &db, &reddit_client, "")
        .await?;

    Ok(())
}

pub async fn unsubscribe(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Dialog::<Unsubscribe>::new(user_id.to_string())
        .handle_current_step(&telegram_client, &db, "")
        .await?;

    Ok(())
}

pub async fn subscriptions(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(res) = db.get_user_subscriptions(user_id) {
        if res.is_empty() {
            telegram_client
                .send_message(&Message {
                    chat_id: user_id,
                    text: "You have no subscriptions",
                    ..Default::default()
                })
                .await?;
        } else {
            let text = res
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
    }

    Ok(())
}

pub async fn help(
    telegram_client: &TelegramClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
    use super::*;

    use crate::db::test_helpers::{setup_test_db, setup_test_db_with};
    use mockito::{mock, server_url, Matcher};
    use serde_json::json;
    use serial_test::serial;

    const TOKEN: &str = "token";
    const USER_ID: &str = "123";

    #[tokio::test]
    #[serial]
    async fn start_new_user() {
        let url = &server_url();
        let resp = r#"{"ok":true,"result":{"message_id":691,"from":{"id":414141,"is_bot":true,"first_name":"Bot","username":"Bot"},"chat":{"id":123,"first_name":"Name","username":"username","type":"private"},"date":1581200384,"text":"This is a test message"}}"#;
        let message = Message {
            chat_id: USER_ID,
            text: HELP_TEXT,
            ..Default::default()
        };
        let _m = mock("POST", format!("/bot{}/sendMessage", TOKEN).as_str())
            .match_body(Matcher::Json(json!(message)))
            .with_status(200)
            .with_body(resp)
            .with_header("content-type", "application/json")
            .expect(1)
            .create();

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
        let resp = r#"{"ok":true,"result":{"message_id":691,"from":{"id":414141,"is_bot":true,"first_name":"Bot","username":"Bot"},"chat":{"id":123,"first_name":"Name","username":"username","type":"private"},"date":1581200384,"text":"This is a test message"}}"#;
        let message = Message {
            chat_id: USER_ID,
            text: HELP_TEXT,
            ..Default::default()
        };

        let _m = mock("POST", format!("/bot{}/sendMessage", TOKEN).as_str())
            .match_body(Matcher::Json(json!(message)))
            .with_status(200)
            .with_body(resp)
            .with_header("content-type", "application/json")
            .expect(1)
            .create();

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
    async fn start_db_error() {
        let url = &server_url();
        let resp = r#"{"ok":true,"result":{"message_id":691,"from":{"id":414141,"is_bot":true,"first_name":"Bot","username":"Bot"},"chat":{"id":123,"first_name":"Name","username":"username","type":"private"},"date":1581200384,"text":"This is a test message"}}"#;
        let message = Message {
            chat_id: USER_ID,
            text: ERROR_TEXT,
            ..Default::default()
        };

        let _m = mock("POST", format!("/bot{}/sendMessage", TOKEN).as_str())
            .match_body(Matcher::Json(json!(message)))
            .with_status(200)
            .with_body(resp)
            .with_header("content-type", "application/json")
            .expect(1)
            .create();

        let telegram_client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let db_client = setup_test_db_with(false);
        start(&telegram_client, &db_client, USER_ID).await.unwrap();
        _m.assert();
    }
}
