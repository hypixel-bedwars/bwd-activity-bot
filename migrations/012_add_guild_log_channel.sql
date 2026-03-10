-- Migration: 012_add_guild_log_channel.sql
-- Add a log_channel_id column to the guilds table so the bot can store
-- which channel in a guild should receive automated logs.
--
-- This migration is idempotent (uses IF NOT EXISTS) so it can be run safely
-- against databases that have already been migrated.

ALTER TABLE guilds
    ADD COLUMN IF NOT EXISTS log_channel_id BIGINT;

-- Add an index to speed lookups by log channel if you ever need to locate
-- a guild by its configured logging channel.
CREATE INDEX IF NOT EXISTS idx_guilds_log_channel
    ON guilds (log_channel_id);
