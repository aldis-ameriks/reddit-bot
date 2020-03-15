use std::error::Error;

use crate::bot::commands::{help, start, stop, subscribe, subscriptions, unsubscribe};
use crate::db::client::Client as DbClient;
use crate::reddit::client::Client as RedditClient;
use chrono::Weekday;
use futures::StreamExt;
use log::{error, info, warn};
use num::traits::FromPrimitive;
use telegram_bot::prelude::*;
use telegram_bot::{
    Api, InlineKeyboardButton, InlineKeyboardMarkup, MessageKind, ReplyMarkup, UpdateKind, User,
};

pub async fn init_bot(token: &str, database_url: &str) {
    let db = DbClient::new(&database_url);
    let api = Api::new(&token);
    let reddit_client = RedditClient::new();

    let handle_stuff =
        |data: String, from: User| handle_message(data, from, &api, &db, &reddit_client);

    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        if let Ok(update) = update {
            match update.kind {
                UpdateKind::Message(message) => {
                    if let MessageKind::Text { data, .. } = message.kind {
                        if let Err(e) = handle_stuff(data, message.from).await {
                            error!("error handling message: {}", e);
                        }
                    }
                }
                UpdateKind::CallbackQuery(query) => {
                    if let Some(data) = query.data {
                        if let Err(e) = handle_stuff(data, query.from).await {
                            error!("error handling message in callback query: {}", e);
                        }
                    } else {
                        warn!("empty message in callback query");
                    }
                }
                _ => {}
            }
        }
    }
}

async fn handle_message(
    data: String,
    from: User,
    api: &Api,
    db: &DbClient,
    reddit_client: &RedditClient,
) -> Result<(), Box<dyn Error>> {
    info!(
        "received message from: {}({}), message: {}",
        &from.first_name, &from.id, data
    );

    match data.as_ref() {
        "/start" => start(&api, &db, &from).await?,
        "/stop" => stop(&api, &db, &from).await?,
        "/subscribe" => subscribe(&api, &db, &reddit_client, &from, None, None, None).await?,
        "/unsubscribe" => unsubscribe(&api, &db, &from, None).await?,
        "/subscriptions" => subscriptions(&api, &db, &from).await?,
        "/help" => help(&api, &from).await?,
        _ => {
            if let Ok(last_command) = db.get_users_last_command(&from.id.to_string()) {
                if let Some(mut last_command) = last_command {
                    match last_command.command.as_str() {
                        // TODO: encapsulate step logic inside dialog struct
                        "/subscribe" => {
                            match last_command.step {
                                0 => {
                                    // TODO: validate subreddit
                                    // TODO: allow specifying multiple subreddits
                                    // TODO: extract helper function for building inline options
                                    let buttons = (0..7)
                                        .map(|weekday| {
                                            InlineKeyboardButton::callback(
                                                format!("{}", Weekday::from_u8(weekday).unwrap()),
                                                weekday.to_string(),
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
                                        from.text("On which day do you want to receive the posts?")
                                            .reply_markup(ReplyMarkup::InlineKeyboardMarkup(
                                                markup,
                                            )),
                                    )
                                    .await?;

                                    last_command.step += 1;
                                    last_command.data = data;
                                    db.insert_or_update_last_command(&last_command).ok();
                                }
                                1 => {
                                    // TODO: extract helper function for building inline options
                                    let buttons = (0..24)
                                        .map(|hour| {
                                            InlineKeyboardButton::callback(
                                                format!("{}:00", hour),
                                                format!("{}", hour),
                                            )
                                        })
                                        .collect::<Vec<InlineKeyboardButton>>();
                                    let mut markup = InlineKeyboardMarkup::new();
                                    let mut row: Vec<InlineKeyboardButton> = vec![];
                                    let mut buttons_iterator = buttons.into_iter();
                                    while let Some(button) = buttons_iterator.next() {
                                        row.push(button);
                                        if row.len() == 3 {
                                            markup.add_row(row.clone());
                                            row = vec![];
                                        }
                                    }

                                    if row.len() > 0 {
                                        markup.add_row(row);
                                    }
                                    api.send(
                                        from.text("At what time? (UTC)").reply_markup(
                                            ReplyMarkup::InlineKeyboardMarkup(markup),
                                        ),
                                    )
                                    .await?;

                                    last_command.step += 1;
                                    last_command.data = format!("{};{}", last_command.data, data);
                                    db.insert_or_update_last_command(&last_command).ok();
                                }
                                2 => {
                                    let prev_data =
                                        last_command.data.split(";").collect::<Vec<&str>>();
                                    let subreddit = prev_data.get(0).unwrap();
                                    let day = prev_data.get(1).unwrap().parse::<i32>().unwrap_or(0);
                                    let time = data.parse::<i32>().unwrap_or(12);

                                    subscribe(
                                        &api,
                                        &db,
                                        &reddit_client,
                                        &from,
                                        Some(&subreddit),
                                        Some(day),
                                        Some(time),
                                    )
                                    .await?;
                                }
                                _ => {}
                            }
                            return Ok(());
                        }
                        "/unsubscribe" => {
                            if last_command.step == 0 {
                                unsubscribe(&api, &db, &from, Some(&data)).await?;
                                last_command.step += 1;
                                db.insert_or_update_last_command(&last_command).ok();
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                }
            }
            api.send(from.text("Say what?")).await?;
        }
    }
    Ok(())
}
