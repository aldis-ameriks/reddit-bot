use crate::db::client::Client as DbClient;
use crate::reddit::client::Client as RedditClient;
use crate::task::task::process_subscription;
use telegram_bot::prelude::*;
use telegram_bot::{Api, InlineKeyboardButton, InlineKeyboardMarkup, ReplyMarkup, User};

const HELP_TEXT: &str = r#"
These are the commands I know
/start
/stop
/subscribe <subreddit>
/unsubscribe <subreddit>
/subscriptions
/help
"#;

pub async fn start(
    api: &Api,
    db: &DbClient,
    from: &User,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = db.create_user(&from.id.to_string()) {
        api.send(from.text(HELP_TEXT)).await?;
    }
    Ok(())
}

pub async fn stop(api: &Api, db: &DbClient, from: &User) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = db.delete_user(&from.id.to_string()) {
        api.send(from.text("User and subscriptions deleted"))
            .await?;
    }
    Ok(())
}

pub async fn subscribe(
    api: &Api,
    db: &DbClient,
    reddit_client: &RedditClient,
    from: &User,
    payload: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let None = payload {
        api.send(from.text("Missing subreddit")).await?;
        return Ok(());
    }

    let data = payload.unwrap();

    if !reddit_client.validate_subreddit(&data).await {
        api.send(from.text("Invalid subreddit")).await?;
        return Ok(());
    }

    if let Ok(subscription) = db.subscribe(&from.id.to_string(), &data) {
        api.send(from.text(format!("Subscribed to: {}", &data)))
            .await?;
        process_subscription(&db, &api, &reddit_client, &subscription).await;
    }

    Ok(())
}

pub async fn unsubscribe(
    api: &Api,
    db: &DbClient,
    from: &User,
    data: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let None = data {
        if let Ok(res) = db.get_user_subscriptions(&from.id.to_string()) {
            let buttons = res
                .iter()
                .map(|subscription| {
                    InlineKeyboardButton::callback(
                        subscription.subreddit.to_string(),
                        format!("/unsubscribe {}", subscription.subreddit),
                    )
                })
                .collect::<Vec<InlineKeyboardButton>>();

            let mut markup = InlineKeyboardMarkup::new();
            let mut row: Vec<InlineKeyboardButton> = vec![];
            let mut buttons_iterator = buttons.into_iter();
            while let Some(button) = buttons_iterator.next() {
                row.push(button);
                if row.len() == 2 {
                    markup.add_row(row.clone());
                    row = vec![];
                }
            }

            if row.len() > 0 {
                markup.add_row(row);
            }

            api.send(
                from.text("Select subreddit")
                    .reply_markup(ReplyMarkup::InlineKeyboardMarkup(markup)),
            )
            .await?;
        }
        return Ok(());
    }

    let data = data.unwrap();

    if let Ok(_) = db.unsubscribe(&from.id.to_string(), &data) {
        api.send(from.text(format!("Unsubscribed from: {}", &data)))
            .await?;
    }

    Ok(())
}

pub async fn subscriptions(
    api: &Api,
    db: &DbClient,
    from: &User,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(res) = db.get_user_subscriptions(&from.id.to_string()) {
        let text = res
            .iter()
            .map(|subscription| format!("{}\n", subscription.subreddit))
            .collect::<String>();
        if let 0 = text.len() {
            api.send(from.text("You have no subscriptions")).await?;
        } else {
            api.send(from.text(format!("You are currently subscribed to:\n{}", text)))
                .await?;
        }
    }

    Ok(())
}

pub async fn help(api: &Api, from: &User) -> Result<(), Box<dyn std::error::Error>> {
    api.send(from.text(HELP_TEXT)).await?;
    Ok(())
}
