-- Your SQL goes here
CREATE TABLE messages
(
    id         BIGINT NOT NULL PRIMARY KEY,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    group_id   BIGINT NOT NULL REFERENCES groups(id),
    source_id  BIGINT NOT NULL REFERENCES users(id)
);

CREATE TABLE message_content
(
    message_id BIGINT NOT NULL,
    user_id    BIGINT NOT NULL,
    content    TEXT NOT NULL,
    CONSTRAINT pk_message_content PRIMARY KEY (message_id, user_id)
);
