-- Migration 021: Add ON DELETE CASCADE to all user_id foreign keys

-- hypixel_stats_snapshot.user_id
ALTER TABLE hypixel_stats_snapshot
    DROP CONSTRAINT IF EXISTS hypixel_stats_snapshot_user_id_fkey,
    ADD CONSTRAINT hypixel_stats_snapshot_user_id_fkey
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- discord_stats_snapshot.user_id
ALTER TABLE discord_stats_snapshot
    DROP CONSTRAINT IF EXISTS discord_stats_snapshot_user_id_fkey,
    ADD CONSTRAINT discord_stats_snapshot_user_id_fkey
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- xp.user_id
ALTER TABLE xp
    DROP CONSTRAINT IF EXISTS xp_user_id_fkey,
    ADD CONSTRAINT xp_user_id_fkey
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- sweep_cursor.user_id
ALTER TABLE sweep_cursor
    DROP CONSTRAINT IF EXISTS sweep_cursor_user_id_fkey,
    ADD CONSTRAINT sweep_cursor_user_id_fkey
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- stat_deltas.user_id
ALTER TABLE stat_deltas
    DROP CONSTRAINT IF EXISTS stat_deltas_user_id_fkey,
    ADD CONSTRAINT stat_deltas_user_id_fkey
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- xp_events.user_id
ALTER TABLE xp_events
    DROP CONSTRAINT IF EXISTS xp_events_user_id_fkey,
    ADD CONSTRAINT xp_events_user_id_fkey
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- daily_snapshots.user_id
ALTER TABLE daily_snapshots
    DROP CONSTRAINT IF EXISTS fk_daily_snapshots_user,
    ADD CONSTRAINT fk_daily_snapshots_user
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- event_xp.user_id
ALTER TABLE event_xp
    DROP CONSTRAINT IF EXISTS event_xp_user_id_fkey,
    ADD CONSTRAINT event_xp_user_id_fkey
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
