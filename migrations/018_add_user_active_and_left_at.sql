-- Add an 'active' flag and 'left_at' timestamp to users so we can hide
-- users who left without deleting their history.
ALTER TABLE users
    ADD COLUMN active BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN left_at TIMESTAMPTZ NULL;
