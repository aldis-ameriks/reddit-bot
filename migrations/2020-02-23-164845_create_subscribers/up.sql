CREATE TABLE users
(
    id         varchar(20) PRIMARY KEY NOT NULL,
    created_at varchar(32)             NOT NULL
);

CREATE TABLE users_subscriptions
(
    id           integer PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id      varchar(20)                       NOT NULL,
    subreddit    varchar(255)                      NOT NULL,
    last_sent_at varchar(32),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE UNIQUE INDEX idx_users_subscriptions ON users_subscriptions (user_id, subreddit);
