-- Drop existing tables
DROP TABLE IF EXISTS event_leaderboard_positions;
DROP TABLE IF EXISTS event_message_requirement;

-- Create new unified table
CREATE TABLE event_message_requirements (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    min_messages INTEGER NOT NULL CHECK (min_messages > 0),
    positions INTEGER[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT positions_not_empty CHECK (array_length(positions, 1) > 0)
);

CREATE INDEX idx_event_message_requirements_event_id 
    ON event_message_requirements(event_id);

CREATE INDEX idx_event_message_requirements_positions 
    ON event_message_requirements USING GIN (positions);