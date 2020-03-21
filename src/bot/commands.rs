use crate::bot::dialogs::{Dialog, Subscribe, Unsubscribe};
use crate::db::client::DbClient;
use crate::reddit::client::RedditClient;
use crate::telegram::client::TelegramClient;
use crate::telegram::types::Message;

const HELP_TEXT: &str = r#"
You can send me these commands:
/start
/stop
/subscribe
/unsubscribe
/subscriptions
/help
"#;

pub async fn start(
    telegram_client: &TelegramClient,
    db: &DbClient,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = db.create_user(user_id) {
        telegram_client
            .send_message(&Message {
                chat_id: user_id,
                text: HELP_TEXT,
                ..Default::default()
            })
            .await?;
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
