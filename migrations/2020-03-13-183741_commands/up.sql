CREATE TABLE commands
(
    user_id      varchar(20) PRIMARY KEY NOT NULL REFERENCES users (id) ON DELETE CASCADE ON UPDATE CASCADE,
    command      varchar(32)             NOT NULL,
    created_at   varchar(32)             NOT NULL,
    updated_at   varchar(32)             NOT NULL,
    current_step varchar(32)             NOT NULL,
    data         varchar(1000)
)
