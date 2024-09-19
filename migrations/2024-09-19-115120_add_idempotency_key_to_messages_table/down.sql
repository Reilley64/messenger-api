-- This file should undo anything in `up.sql`
ALTER TABLE messages DROP COLUMN idempotency_key;
