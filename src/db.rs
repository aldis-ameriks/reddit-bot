use diesel::prelude::*;
use diesel::result::Error;
use log::{error, info};

use crate::models::{NewSubscription, Subscription, User};
use crate::schema;
use chrono::Utc;

pub struct DbClient(SqliteConnection);

impl DbClient {
    pub fn new(url: &str) -> DbClient {
        let conn = SqliteConnection::establish(url).expect("Error connecting to {}");
        conn.execute("PRAGMA foreign_keys = ON")
            .expect("Failed to enable foreign key support");
        DbClient(conn)
    }

    pub fn create_user(&self, id: &str) -> Result<User, Error> {
        use crate::schema::users;
        let curr = chrono::Utc::now();

        let new_user = User {
            id: id.to_string(),
            created_at: curr.to_rfc3339(),
        };

        info!("creating new user: {:?}", new_user);

        match diesel::insert_into(users::table)
            .values(&new_user)
            .execute(&self.0)
        {
            Ok(_) => Ok(new_user),
            Err(err) => {
                error!("failed to create new user: {}", err);
                Err(err)
            }
        }
    }

    pub fn delete_user(&self, id: &str) -> Result<(), Error> {
        use schema::users::dsl;
        match diesel::delete(dsl::users.filter(dsl::id.eq(id))).execute(&self.0) {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("failed to delete user: {}", err);
                Err(err)
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_users(&self) -> Result<Vec<User>, Error> {
        use schema::users::dsl;
        match dsl::users.load::<User>(&self.0) {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("failed to get users: {}", err);
                Err(err)
            }
        }
    }

    pub fn subscribe(&self, user_id: &str, subreddit: &str) -> Result<(), Error> {
        use schema::users_subscriptions::dsl;

        info!("subscribing user_id: {}, subreddit: {}", user_id, subreddit);

        let new_subscription = NewSubscription { user_id, subreddit };

        match diesel::insert_into(dsl::users_subscriptions)
            .values(&new_subscription)
            .execute(&self.0)
        {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("failed to create new subscription: {}", err);
                Err(err)
            }
        }
    }

    pub fn update_last_sent(&self, id: i32) -> Result<(), Error> {
        use schema::users_subscriptions::dsl;

        info!("updating last sent at id: {}", id);

        match diesel::update(dsl::users_subscriptions.find(id))
            .set(dsl::last_sent_at.eq(Utc::now().to_rfc3339()))
            .execute(&self.0)
        {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("failed to update last sent date: {}", err);
                Err(err)
            }
        }
    }

    pub fn unsubscribe(&self, user_id: &str, subreddit: &str) -> Result<(), Error> {
        info!(
            "unsubscribing user_id: {}, subreddit: {}",
            user_id, subreddit
        );
        use schema::users_subscriptions::dsl;

        match diesel::delete(
            dsl::users_subscriptions
                .filter(dsl::user_id.eq(user_id).and(dsl::subreddit.eq(subreddit))),
        )
        .execute(&self.0)
        {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("failed unsubscribe: {}", err);
                Err(err)
            }
        }
    }

    pub fn get_subscriptions(&self) -> Result<Vec<Subscription>, Error> {
        use schema::users_subscriptions::dsl;
        match dsl::users_subscriptions.load::<Subscription>(&self.0) {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("failed to get subscriptions: {}", err);
                Err(err)
            }
        }
    }

    pub fn get_user_subscriptions(&self, user_id: &str) -> Result<Vec<Subscription>, Error> {
        use schema::users_subscriptions::dsl;
        match dsl::users_subscriptions
            .filter(dsl::user_id.eq(user_id))
            .load::<Subscription>(&self.0)
        {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("failed to get subscriptions: {}", err);
                Err(err)
            }
        }
    }
}
