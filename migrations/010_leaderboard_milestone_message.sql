-- Migration 010: Add milestone_message_id to persistent_leaderboards table.
--
-- milestone_message_id — Discord message ID of the separate milestone card
--                        message posted alongside the leaderboard pages.
--                        0 means no milestone message has been sent yet
--                        (matches the existing convention used by
--                        status_message_id).
--                        Updated by leaderboard_create and the background
--                        leaderboard updater whenever milestones change.

ALTER TABLE persistent_leaderboards
    ADD COLUMN milestone_message_id BIGINT NOT NULL DEFAULT 0;
