-- Your SQL goes here
CREATE TABLE user_push_subscriptions
(
    id         BIGINT NOT NULL PRIMARY KEY,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    user_id    BIGINT NOT NULL REFERENCES users(id),
    endpoint   TEXT NOT NULL,
    p256dh     TEXT NOT NULL,
    auth       TEXT NOT NULL
);
