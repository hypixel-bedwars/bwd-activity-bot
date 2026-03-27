-- Register event participation eligibility function.
--
-- Important: files under src/database/sql are NOT auto-executed by sqlx::migrate!
-- in this project. Keeping this function in a numbered migration guarantees it
-- is present in every environment.

CREATE OR REPLACE FUNCTION is_player_allowed(
    p_player_id BIGINT,
    p_event_id BIGINT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
AS $$
DECLARE
    latest_ban_action modrec_action;
    latest_ban_expiry TIMESTAMPTZ;
    latest_event_action modrec_action;
BEGIN
    -- Latest global ban/unban action for this user.
    SELECT m.action_type, m.ban_expires_at
      INTO latest_ban_action, latest_ban_expiry
      FROM modrec m
     WHERE m.user_id = p_player_id
       AND m.action_type IN ('ban', 'unban')
     ORDER BY m.created_at DESC, m.id DESC
     LIMIT 1;

    IF latest_ban_action = 'ban'
       AND (latest_ban_expiry IS NULL OR latest_ban_expiry > NOW()) THEN
        RETURN FALSE;
    END IF;

    -- Latest event-specific disqualify/undisqualify action.
    SELECT m.action_type
      INTO latest_event_action
      FROM modrec m
     WHERE m.user_id = p_player_id
       AND m.event_id = p_event_id
       AND m.action_type IN ('disqualify', 'undisqualify')
     ORDER BY m.created_at DESC, m.id DESC
     LIMIT 1;

    IF latest_event_action = 'disqualify' THEN
        RETURN FALSE;
    END IF;

    RETURN TRUE;
END;
$$;
