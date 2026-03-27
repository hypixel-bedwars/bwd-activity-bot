BEGIN;

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

-- Validate counts (scoped to THIS migration only)
DO $$
DECLARE
    old_bans INT;
    new_bans INT;
    old_dq INT;
    new_dq INT;
BEGIN
    SELECT COUNT(*) INTO old_bans 
    FROM users 
    WHERE event_ban_until IS NOT NULL;

    SELECT COUNT(*) INTO new_bans 
    FROM modrec 
    WHERE action_type = 'ban'
      AND reason = 'Migrated from old system';

    SELECT COUNT(*) INTO old_dq 
    FROM event_participants 
    WHERE disqualified = TRUE;

    SELECT COUNT(*) INTO new_dq 
    FROM modrec 
    WHERE action_type = 'disqualify'
      AND reason = 'Migrated from event_participants';

    IF old_bans != new_bans THEN
        RAISE EXCEPTION 'Ban mismatch: % vs %', old_bans, new_bans;
    END IF;

    IF old_dq != new_dq THEN
        RAISE EXCEPTION 'DQ mismatch: % vs %', old_dq, new_dq;
    END IF;
END$$;

-- Only delete AFTER validation passes
DROP TABLE event_participants;

ALTER TABLE users
DROP COLUMN event_ban_until,
DROP COLUMN event_ban_reason;
COMMIT;