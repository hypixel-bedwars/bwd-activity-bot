-- Migration 019: Event milestones
--
-- Adds per-event XP milestone thresholds so admins can define checkpoints
-- (e.g. 500 XP, 1 000 XP) that participants are tracked against.
--
-- Also adds milestone_message_id to persistent_event_leaderboards so the
-- background updater can keep a live milestone card pinned alongside the
-- leaderboard pages (matching the behaviour of the regular persistent
-- leaderboard).

-- ---------------------------------------------------------------------------
-- event_milestones
-- ---------------------------------------------------------------------------

CREATE TABLE event_milestones (
    id           BIGSERIAL        PRIMARY KEY,
    event_id     BIGINT           NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    xp_threshold DOUBLE PRECISION NOT NULL CHECK (xp_threshold > 0),
    created_at   TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    UNIQUE (event_id, xp_threshold)
);

CREATE INDEX idx_event_milestones_event_id ON event_milestones (event_id);

-- ---------------------------------------------------------------------------
-- persistent_event_leaderboards — add milestone message tracking
-- ---------------------------------------------------------------------------

ALTER TABLE persistent_event_leaderboards
    ADD COLUMN milestone_message_id BIGINT NOT NULL DEFAULT 0;
