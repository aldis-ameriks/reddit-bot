DROP TABLE dialogs;

CREATE TABLE commands
(
    user_id varchar(20) PRIMARY KEY NOT NULL REFERENCES users (id) ON DELETE CASCADE ON UPDATE CASCADE,
    command varchar(32)             NOT NULL,
    step    integer                 NOT NULL,
    data    varchar(1000)           NOT NULL
)
