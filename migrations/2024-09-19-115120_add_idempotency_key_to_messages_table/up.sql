-- Your SQL goes here
ALTER TABLE messages ADD COLUMN idempotency_key VARCHAR(255);
