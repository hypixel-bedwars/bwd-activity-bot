CREATE INDEX idx_event_xp_event_xp
ON event_xp (event_id, xp_earned);

CREATE INDEX idx_users_active
ON users (id) WHERE active = TRUE;

-- For daily_snapshots
CREATE INDEX idx_hypixel_stats_snapshot_latest
ON hypixel_stats_snapshot (user_id, stat_name, timestamp DESC);

-- modec indexes
CREATE INDEX idx_modrec_ban_lookup
ON modrec (user_id, created_at DESC, id DESC)
WHERE action_type IN ('ban', 'unban');

CREATE INDEX idx_modrec_event_lookup
ON modrec (user_id, event_id, created_at DESC, id DESC)
WHERE action_type IN ('disqualify', 'undisqualify');