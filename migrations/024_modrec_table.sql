-- action can only be 'ban' or 'disqualify'
-- Ban = User is banned from participating in the guild's events until the ban expires (if applicable). 
-- Disqualify = User is disqualified from the current event but can participate in future events.
CREATE TYPE modrec_action AS ENUM ('ban', 'disqualify', 'unban', 'undisqualify');

-- Create a new table to store moderator records
CREATE TABLE
    modrec (
        id SERIAL PRIMARY KEY,
        user_id BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
        moderator_id BIGINT NOT NULL,
        guild_id BIGINT NOT NULL REFERENCES guilds (guild_id) ON DELETE CASCADE,
        action_type modrec_action NOT NULL,
        -- Ban expiration and Event id for ban and disqualify actions respectively
        ban_expires_at TIMESTAMPTZ,
        event_id BIGINT REFERENCES events (id) ON DELETE CASCADE,
        reason TEXT default 'No reason provided',
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW ()
    );

-- Ban need to have a timestamp for when the ban expires, while disqualify does not need an expiration timestamp.
-- In case of disqualify the expiration timestamp should be the same as the timestamp of the event the user is disqualified from.
ALTER TABLE modrec ADD CONSTRAINT modrec_action_rules CHECK (
    -- BAN → must have expiration, no event
    (
        action_type = 'ban'
        AND event_id IS NULL
    )
    OR
    -- DISQUALIFY → must have event, no ban expiration
    (
        action_type = 'disqualify'
        AND event_id IS NOT NULL
        AND ban_expires_at IS NULL
    )
    OR
    -- UNBAN → no expiration, no event
    (
        action_type = 'unban'
        AND ban_expires_at IS NULL
        AND event_id IS NULL
    )
    OR
    -- UNDISQUALIFY → must have event
    (
        action_type = 'undisqualify'
        AND event_id IS NOT NULL
        AND ban_expires_at IS NULL
    )
);

-- Add a index on user_id and action_type for faster lookups of a user's modrec history
CREATE INDEX idx_modrec_user_id ON modrec (user_id, action_type);

CREATE INDEX idx_modrec_user_created_desc ON modrec (user_id, created_at DESC);

CREATE INDEX idx_modrec_user_event_created_desc ON modrec (user_id, event_id, created_at DESC);