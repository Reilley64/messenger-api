-- Your SQL goes here
CREATE TABLE message_requests
(
    id             BIGINT PRIMARY KEY           NOT NULL,
    created_at     TIMESTAMP WITHOUT TIME ZONE  NOT NULL,
    updated_at     TIMESTAMP WITHOUT TIME ZONE  NOT NULL,
    source_id      BIGINT REFERENCES users (id) NOT NULL,
    destination_id BIGINT REFERENCES users (id) NOT NULL,
    approved_at    TIMESTAMP WITHOUT TIME ZONE
);

CREATE INDEX message_requests_mm_id_destination_id_idx ON message_requests (id, destination_id);

CREATE INDEX message_requests_mm_source_id_destination_id_idx ON message_requests (source_id, destination_id);
