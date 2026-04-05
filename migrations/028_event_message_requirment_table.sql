-- 1. Table for base event requirements
CREATE TABLE event_message_requirement (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT REFERENCES events (id) ON DELETE CASCADE,
    min_messages INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Table for the unique leaderboard positions
CREATE TABLE event_leaderboard_positions (
    event_id BIGINT REFERENCES events (id) ON DELETE CASCADE,
    position INTEGER,
    -- This enforces uniqueness: no two rows can have the same event_id AND position
    PRIMARY KEY (event_id, position)
);

-- Highly recommended if you search by rank/position across all events
CREATE INDEX idx_event_leaderboard_positions_rank ON event_leaderboard_positions (position);

CREATE INDEX idx_event_message_requirement_event_id ON event_message_requirement (event_id);