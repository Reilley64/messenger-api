-- Your SQL goes here
CREATE TABLE users
(
    id           BIGINT PRIMARY KEY          NOT NULL,
    created_at   TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated_at   TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    sub          VARCHAR(255)                NOT NULL,
    email        VARCHAR(255)                NOT NULL,
    phone_number VARCHAR(255)                NOT NULL,
    first_name   VARCHAR(255)                NOT NULL,
    last_name    VARCHAR(255)                NOT NULL,
    display_name VARCHAR(255),
    public_key   VARCHAR(392)                NOT NULL
);

ALTER TABLE users
    ADD CONSTRAINT uc_users_sub UNIQUE (sub);

ALTER TABLE users
    ADD CONSTRAINT uc_users_email UNIQUE (email);

ALTER TABLE users
    ADD CONSTRAINT uc_users_phone_number UNIQUE (phone_number);
