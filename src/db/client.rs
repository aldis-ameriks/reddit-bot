use chrono::Utc;
use diesel::prelude::*;
use diesel::result::Error;
use log::{error, info};

use crate::db::models::DialogEntity;

use super::models::{NewSubscription, Subscription, User};
use super::schema;

embed_migrations!();

pub struct DbClient {
    pub conn: SqliteConnection,
}

impl DbClient {
    pub fn new(url: &str) -> DbClient {
        let conn = SqliteConnection::establish(url).expect(&format!("Error connecting to {}", url));
        conn.execute("PRAGMA foreign_keys = ON")
            .expect("Failed to enable foreign key support");

        // TODO: run migration on applications startup
        embedded_migrations::run(&conn).unwrap();
        DbClient { conn }
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
            .execute(&self.conn)
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
        match diesel::delete(dsl::users.filter(dsl::id.eq(id))).execute(&self.conn) {
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
        match dsl::users.load::<User>(&self.conn) {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("failed to get users: {}", err);
                Err(err)
            }
        }
    }

    pub fn subscribe(
        &self,
        user_id: &str,
        subreddit: &str,
        send_on: i32,
        send_at: i32,
    ) -> Result<Subscription, Error> {
        use schema::users_subscriptions::dsl;

        info!("subscribing user_id: {}, subreddit: {}", user_id, subreddit);

        let new_subscription = NewSubscription {
            user_id,
            subreddit,
            send_on,
            send_at,
            last_sent_at: Some(Utc::now().to_rfc3339()),
        };

        match self.conn.transaction::<_, Error, _>(|| {
            diesel::insert_into(dsl::users_subscriptions)
                .values(&new_subscription)
                .execute(&self.conn)?;

            dsl::users_subscriptions
                .order(dsl::id.desc())
                .first::<Subscription>(&self.conn)
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
            .execute(&self.conn)
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
        .execute(&self.conn)
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
        match dsl::users_subscriptions.load::<Subscription>(&self.conn) {
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
            .load::<Subscription>(&self.conn)
        {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("failed to get subscriptions: {}", err);
                Err(err)
            }
        }
    }

    pub fn get_users_dialog(&self, user_id: &str) -> Result<DialogEntity, Error> {
        use schema::dialogs::dsl;
        match dsl::dialogs
            .filter(dsl::user_id.eq(user_id))
            .first::<DialogEntity>(&self.conn)
        {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("failed to get users dialog: {}", err);
                Err(err)
            }
        }
    }

    pub fn insert_or_update_dialog(&self, dialog: &DialogEntity) -> Result<(), Error> {
        use schema::dialogs::dsl;
        info!("inserting or updating dialog: {:?}", dialog);

        match diesel::replace_into(dsl::dialogs)
            .values(vec![dialog])
            .execute(&self.conn)
        {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("failed to insert or update dialog: {}", err);
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use serial_test::serial;

    use super::*;
    use crate::db::test_helpers::setup_test_db;

    const USER_ID: &str = "1";

    #[test]
    #[serial]
    fn users() {
        let client = setup_test_db();
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
        let client = setup_test_db();
        client.create_user(USER_ID).unwrap();

        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        client.subscribe(USER_ID, "rust", 0, 12).unwrap();

        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subreddit, "rust");

        let result = client.subscribe(USER_ID, "rust", 0, 12).unwrap_err();
        let result = format!("{}", result);
        assert!(result.contains(
            "UNIQUE constraint failed: users_subscriptions.user_id, users_subscriptions.subreddit"
        ));

        client
            .subscribe(USER_ID, "Whatcouldgowrong", 0, 12)
            .unwrap();
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

        let client = setup_test_db();
        client.create_user(USER_ID).unwrap();
        client.create_user(SECOND_USER_ID).unwrap();

        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        let result = client.get_user_subscriptions(SECOND_USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        client.subscribe(USER_ID, "rust", 0, 12).unwrap();

        let result = client.get_user_subscriptions(SECOND_USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        let result = client.get_subscriptions().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subreddit, "rust");
    }

    #[test]
    #[serial]
    fn update_last_sent() {
        let client = setup_test_db();
        client.create_user(USER_ID).unwrap();

        let result = client.get_user_subscriptions(USER_ID).unwrap();
        assert_eq!(result.len(), 0);

        client.subscribe(USER_ID, "rust", 0, 12).unwrap();

        let result = client.get_subscriptions().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subreddit, "rust");
        assert_eq!(result[0].send_on, 0);
        assert_eq!(result[0].send_at, 12);

        client.update_last_sent(result[0].id).unwrap();
        let result = client.get_subscriptions().unwrap();
        assert!(result[0].last_sent_at.is_some());
    }

    #[test]
    #[serial]
    fn dialogs() {
        let client = setup_test_db();
        client.create_user(USER_ID).unwrap();

        let result = client.get_users_dialog(USER_ID);
        assert!(result.is_err());

        let dialog = DialogEntity {
            user_id: USER_ID.to_string(),
            command: "/subscribe".to_string(),
            step: "One".to_string(),
            data: "".to_string(),
        };

        client.insert_or_update_dialog(&dialog).unwrap();
        let result = client.get_users_dialog(USER_ID).unwrap();
        assert_eq!(result, dialog);

        let dialog2 = DialogEntity {
            step: "Two".to_string(),
            ..dialog
        };
        client.insert_or_update_dialog(&dialog2).unwrap();
        let result = client.get_users_dialog(USER_ID).unwrap();
        assert_eq!(result, dialog2);
    }
}
