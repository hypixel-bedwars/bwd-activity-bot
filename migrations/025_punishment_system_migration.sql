-- Migrate the existing global bans
INSERT INTO modrec (
    user_id,
    moderator_id,
    guild_id,
    action_type,
    ban_expires_at,
    reason,
    created_at
)
SELECT 
    u.id AS user_id,
    0 AS moderator_id, -- system migration
    u.guild_id,
    'ban'::modrec_action,
    u.event_ban_until,
    COALESCE(u.event_ban_reason, 'Migrated from old system'),
    NOW()
FROM users u
WHERE u.event_ban_until IS NOT NULL;

-- Migrate the event bans (disqualifications)
INSERT INTO modrec (
    user_id,
    moderator_id,
    guild_id,
    action_type,
    event_id,
    reason,
    created_at
)
SELECT 
    ep.user_id,
    0 AS moderator_id, -- system migration
    u.guild_id,
    'disqualify'::modrec_action,
    ep.event_id,
    'Migrated from event_participants',
    ep.created_at
FROM event_participants ep
JOIN users u ON u.id = ep.user_id
WHERE ep.disqualified = TRUE;

DROP TABLE event_participants;

ALTER TABLE users
DROP COLUMN event_ban_until,
DROP COLUMN event_ban_reason;