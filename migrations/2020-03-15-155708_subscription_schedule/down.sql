PRAGMA foreign_keys= OFF;

CREATE TABLE users_subscriptions2
(
    id           integer PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id      varchar(20)                       NOT NULL,
    subreddit    varchar(255)                      NOT NULL,
    last_sent_at varchar(32),

    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE ON UPDATE CASCADE
);

INSERT INTO users_subscriptions2(id, user_id, subreddit, last_sent_at)
SELECT id, user_id, subreddit, last_sent_at
FROM users_subscriptions;

DROP TABLE users_subscriptions;

ALTER TABLE users_subscriptions2
    RENAME TO users_subscriptions;

CREATE UNIQUE INDEX idx_users_subscriptions ON users_subscriptions (user_id, subreddit);

PRAGMA foreign_keys= ON;
