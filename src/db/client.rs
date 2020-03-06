use chrono::Utc;
use diesel::prelude::*;
use diesel::result::Error;
use log::{error, info};

use super::models::{NewSubscription, Subscription, User};
use super::schema;
pub struct Client(SqliteConnection);

impl Client {
    pub fn new(url: &str) -> Client {
        let conn = SqliteConnection::establish(url).expect(&format!("Error connecting to {}", url));
        conn.execute("PRAGMA foreign_keys = ON")
            .expect("Failed to enable foreign key support");
        Client(conn)
    }

    pub fn create_user(&self, id: &str) -> Result<User, Error> {
        use schema::users;
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

    pub fn subscribe(&self, user_id: &str, subreddit: &str) -> Result<Subscription, Error> {
        use schema::users_subscriptions::dsl;

        info!("subscribing user_id: {}, subreddit: {}", user_id, subreddit);

        let new_subscription = NewSubscription { user_id, subreddit };

        match self.0.transaction::<_, Error, _>(|| {
            diesel::insert_into(dsl::users_subscriptions)
                .values(&new_subscription)
                .execute(&self.0)?;

            dsl::users_subscriptions
                .order(dsl::id.desc())
                .first::<Subscription>(&self.0)
        }) {
            Ok(subscription) => Ok(subscription),
            Err(err) => {
                error!("failed to subscribe: {}", err);
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
                error!("failed to unsubscribe: {}", err);
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

#[cfg(test)]
mod test {
    use super::*;
    use diesel_migrations::run_pending_migrations;
    use serial_test::serial;

    const USER_ID: &str = "1";

    pub fn setup() -> Client {
        std::fs::create_dir(".tmp").err();
        std::fs::remove_file(".tmp/test.db").err();
        let client = Client::new("file:.tmp/test.db");
        run_pending_migrations(&client.0).unwrap();
        client
    }

    #[test]
    #[serial]
    fn users() {
        let client = setup();
        let result = client.get_users().unwrap();
        assert_eq!(result.len(), 0);

        client.create_user(USER_ID).unwrap();
        let result = client.get_users().unwrap();
        assert_eq!(result.len(), 1);

        let result = client.create_user(USER_ID).unwrap_err();
        let result = format!("{}", result);
        assert!(result.contains("UNIQUE constraint failed: users.id"));

        client.delete_user(USER_ID).unwrap();
        let result = client.get_users().unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    #[serial]
    fn user_subscriptions() {
        let client = setup();
        client.create_user(USER_ID).unwrap();

        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        client.subscribe(USER_ID, "rust").unwrap();

        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subreddit, "rust");

        let result = client.subscribe(USER_ID, "rust").unwrap_err();
        let result = format!("{}", result);
        assert!(result.contains(
            "UNIQUE constraint failed: users_subscriptions.user_id, users_subscriptions.subreddit"
        ));

        client.subscribe(USER_ID, "Whatcouldgowrong").unwrap();
        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].subreddit, "Whatcouldgowrong");
        assert_eq!(result[1].subreddit, "rust");

        client.unsubscribe(USER_ID, "rust").unwrap();
        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subreddit, "Whatcouldgowrong");

        client.delete_user(USER_ID).unwrap();
        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    #[serial]
    fn subscriptions() {
        const SECOND_USER_ID: &str = "2";

        let client = setup();
        client.create_user(USER_ID).unwrap();
        client.create_user(SECOND_USER_ID).unwrap();

        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        let result = client.get_user_subscriptions(SECOND_USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        client.subscribe(USER_ID, "rust").unwrap();

        let result = client.get_user_subscriptions(SECOND_USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        let result = client.get_subscriptions().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subreddit, "rust");
    }

    #[test]
    #[serial]
    fn update_last_sent() {
        let client = setup();
        client.create_user(USER_ID).unwrap();

        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        client.subscribe(USER_ID, "rust").unwrap();

        let result = client.get_subscriptions().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subreddit, "rust");
        assert_eq!(result[0].last_sent_at, None);

        client.update_last_sent(result[0].id).unwrap();
        let result = client.get_subscriptions().unwrap();
        assert!(result[0].last_sent_at.is_some());
    }
}
