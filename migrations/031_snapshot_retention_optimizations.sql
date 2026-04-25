-- Snapshot retention + read/write path optimizations

-- Speed up retention pruning by timestamp while preserving tie-break by id.
CREATE INDEX IF NOT EXISTS idx_hypixel_stats_snapshot_timestamp_id
ON hypixel_stats_snapshot ("timestamp", id);

-- Support sweep ordering for active users first and stale-first refresh logic.
CREATE INDEX IF NOT EXISTS idx_users_active_hypixel_refresh
ON users (last_hypixel_refresh)
WHERE active = TRUE;

-- Speed up first-snapshot lookups (ORDER BY timestamp ASC).
CREATE INDEX IF NOT EXISTS idx_hypixel_user_stat_ts_asc
ON hypixel_stats_snapshot (user_id, stat_name, "timestamp" ASC, id ASC);

-- Speed up first-snapshot lookups for Discord stats as well.
CREATE INDEX IF NOT EXISTS idx_discord_user_stat_ts_asc
ON discord_stats_snapshot (user_id, stat_name, "timestamp" ASC, id ASC);
