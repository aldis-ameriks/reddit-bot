use super::schema::commands;
use super::schema::users;
use super::schema::users_subscriptions;

#[derive(Debug, Queryable, Insertable)]
#[table_name = "users"]
pub struct User {
    pub id: String,
    pub created_at: String,
}

#[derive(Debug, Queryable)]
pub struct Subscription {
    pub id: i32,
    pub user_id: String,
    pub subreddit: String,
    pub last_sent_at: Option<String>,
}

#[derive(Insertable)]
#[table_name = "users_subscriptions"]
pub struct NewSubscription<'a> {
    pub user_id: &'a str,
    pub subreddit: &'a str,
}

#[derive(Debug, Queryable, Insertable, Clone, PartialEq)]
#[table_name = "commands"]
pub struct Command {
    pub user_id: String,
    pub command: String,
    pub step: i32,
}
