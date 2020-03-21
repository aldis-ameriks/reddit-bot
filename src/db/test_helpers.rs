use crate::db::client::DbClient;
use diesel_migrations::run_pending_migrations;

#[allow(dead_code)]
pub fn setup_test_db() -> DbClient {
    setup_test_db_with(true)
}

#[allow(dead_code)]
pub fn setup_test_db_with(run_migrations: bool) -> DbClient {
    std::fs::create_dir(".tmp").err();
    std::fs::remove_file(".tmp/test.db").err();
    let client = DbClient::new("file:.tmp/test.db");
    if run_migrations {
        run_pending_migrations(&client.conn).unwrap();
    }
    client
}
