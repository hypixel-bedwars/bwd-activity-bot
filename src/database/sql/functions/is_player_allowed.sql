CREATE OR REPLACE FUNCTION is_player_allowed(player_id BIGINT, event_id BIGINT)
RETURNS BOOLEAN AS $$
DECLARE
    latest_ban_action TEXT;
    latest_ban_expiry TIMESTAMPTZ;

    latest_event_action TEXT;
BEGIN
    -- Latest ban/unban
    SELECT action_type, ban_expires_at
    INTO latest_ban_action, latest_ban_expiry
    FROM modrec
    WHERE user_id = player_id
      AND action_type IN ('ban', 'unban')
    ORDER BY created_at DESC
    LIMIT 1;

    IF latest_ban_action = 'ban'
       AND (latest_ban_expiry IS NULL or latest_ban_expiry > NOW()) THEN
        RETURN FALSE;
    END IF;

    -- Latest disqualify/undisqualify for this event
    SELECT action_type
    INTO latest_event_action
    FROM modrec
    WHERE user_id = player_id
      AND event_id = is_player_allowed.event_id
      AND action_type IN ('disqualify', 'undisqualify')
    ORDER BY created_at DESC
    LIMIT 1;

    IF latest_event_action = 'disqualify' THEN
        RETURN FALSE;
    END IF;

    -- If both of the above fail it will return a default true
    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;