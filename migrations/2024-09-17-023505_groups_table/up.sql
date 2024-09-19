-- Your SQL goes here
CREATE TABLE groups
(
    id                 BIGINT PRIMARY KEY          NOT NULL,
    created_at         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated_at         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    name               VARCHAR(255),
    message_request_id BIGINT REFERENCES message_requests (id)
);

CREATE TABLE group_users
(
    id         BIGINT PRIMARY KEY          NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    group_id   BIGINT                      NOT NULL REFERENCES groups (id),
    user_id    BIGINT                      NOT NULL REFERENCES users (id),
    is_admin   BOOLEAN                     NOT NULL,
    nickname   VARCHAR(255)
);
