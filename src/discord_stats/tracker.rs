use std::cmp::{max, min};

use serenity::all::{FullEvent, GuildId, RoleId};
use sqlx::PgPool;
use tracing::{debug, error};

use chrono::Utc;

use crate::config::GuildConfig;
use crate::database::queries;
use crate::discord_stats::validation::validate_message;
use crate::shared::types::{Data, Error, StatDelta};
use crate::xp::calculator::{XPConfig, calculate_level, calculate_xp_rewards};

/// Handle a Serenity `FullEvent` and record relevant Discord activity.
pub async fn handle_event(event: &FullEvent, data: &Data) -> Result<(), Error> {
    match event {
        FullEvent::Message { new_message } => {
            if new_message.author.bot {
                return Ok(());
            }

            let Some(guild_id) = new_message.guild_id else {
                return Ok(());
            };

            // Count every non-bot guild message as a raw total (before validation).
            increment_stat_by(
                &data.db,
                data,
                new_message.author.id.get() as i64,
                guild_id.get() as i64,
                "total_messages_raw",
                1,
            )
            .await;

            if !validate_message(
                new_message.author.id.get() as i64,
                &new_message.content,
                data,
            ) {
                return Ok(());
            }

            increment_stat_by(
                &data.db,
                data,
                new_message.author.id.get() as i64,
                guild_id.get() as i64,
                "messages_sent",
                1,
            )
            .await;
        }

        FullEvent::ReactionAdd { add_reaction } => {
            let Some(guild_id) = add_reaction.guild_id else {
                return Ok(());
            };

            let Some(user_id) = add_reaction.user_id else {
                return Ok(());
            };

            increment_stat_by(
                &data.db,
                data,
                user_id.get() as i64,
                guild_id.get() as i64,
                "reactions_added",
                1,
            )
            .await;
        }

        FullEvent::VoiceStateUpdate { old, new } => {
            handle_voice_state_update(data, old.as_ref(), new).await;
        }

        FullEvent::GuildMemberRemoval { guild_id, user, .. } => {
            let discord_user_id = user.id.get() as i64;
            let guild_i64 = guild_id.get() as i64;
            let now = chrono::Utc::now();

            // mark them left (soft delete)
            if let Err(e) =
                queries::mark_user_inactive(&data.db, discord_user_id, guild_i64, &now).await
            {
                error!(error = %e, "Failed to mark user left on GuildMemberRemoval");
            }
        }

        FullEvent::GuildMemberAddition { new_member } => {
            let discord_user_id = new_member.user.id.get() as i64;
            let guild_i64 = new_member.guild_id.get() as i64;

            // Only reactivate if the user was previously registered in this guild.
            match queries::get_user_by_discord_id(&data.db, discord_user_id, guild_i64).await {
                Ok(Some(_db_user)) => {
                    // Reactivate them (soft un-delete)
                    if let Err(e) =
                        queries::mark_user_active(&data.db, discord_user_id, guild_i64).await
                    {
                        error!(error = %e, "Failed to mark user active on GuildMemberAddition");
                    }

                    // Optional: restore registered role if configured and if we can fetch the member
                    if let Ok(Some(guild_row)) = queries::get_guild(&data.db, guild_i64).await {
                        let guild_config: crate::config::GuildConfig =
                            serde_json::from_value(guild_row.config_json).unwrap_or_default();

                        if let Some(role_id) = guild_config.registered_role_id {
                            if let Ok(member) = GuildId::new(guild_i64 as u64)
                                .member(&data.http, new_member.user.id)
                                .await
                            {
                                // ignore role-add errors but you can log them if desired
                                let _ = member.add_role(&data.http, RoleId::new(role_id)).await;
                            }
                        }
                    }
                }

                // User never registered for this guild — nothing to do.
                Ok(None) => {}

                // DB error while checking registration
                Err(e) => {
                    error!(error = %e, "Failed checking user registration on GuildMemberAddition");
                }
            }
        }

        _ => {}
    }

    Ok(())
}

/// Fetch guild config with 5-minute caching to reduce database load.
/// This is shared by both increment_stat_by and handle_vc_xp.
async fn get_guild_config_cached(pool: &PgPool, data: &Data, guild_id: i64) -> GuildConfig {
    const GUILD_CONFIG_TTL: std::time::Duration = std::time::Duration::from_secs(300);

    // Check cache
    let cached = data.guild_configs.get(&guild_id).and_then(|entry| {
        let (cfg, cached_at) = entry.value();
        if cached_at.elapsed() < GUILD_CONFIG_TTL {
            Some(cfg.clone())
        } else {
            None
        }
    });

    if let Some(config) = cached {
        return config;
    }

    // Cache miss - fetch from database
    let fetched = match queries::get_guild(pool, guild_id).await {
        Ok(Some(g)) => serde_json::from_value(g.config_json).unwrap_or_default(),
        Ok(None) => GuildConfig::default(),
        Err(e) => {
            error!(error = %e, "failed to fetch guild config");
            GuildConfig::default()
        }
    };

    data.guild_configs
        .insert(guild_id, (fetched.clone(), std::time::Instant::now()));

    fetched
}

/// Handle a voice state transition and record voice_minutes when a user leaves.
async fn handle_voice_state_update(
    data: &Data,
    old: Option<&serenity::all::VoiceState>,
    new: &serenity::all::VoiceState,
) {
    let Some(guild_id) = new.guild_id else {
        return;
    };
    let guild_id_i64 = guild_id.get() as i64;

    let discord_user_id = new.user_id.get() as i64;

    let guild_config = get_guild_config_cached(&data.db, data, guild_id_i64).await;
    let afk_channel_id = guild_config
        .afk_voice_channel_id
        .map(serenity::all::ChannelId::new);

    let old_channel_id = old.as_ref().and_then(|v| v.channel_id);
    let new_channel_id = new.channel_id;

    let was_trackable_voice = old_channel_id.is_some() && old_channel_id != afk_channel_id;
    let is_trackable_voice = new_channel_id.is_some() && new_channel_id != afk_channel_id;

    match (was_trackable_voice, is_trackable_voice) {
        (false, true) => {
            queries::add_vc_session(&data.db, discord_user_id, guild_id_i64, Utc::now())
                .await
                .unwrap_or_else(|e| {
                    error!(error = %e, "Failed to record VC join");
                });
            debug!(discord_user_id, "Voice session started.");
        }

        (true, false) => {
            let now = Utc::now();

            let join_time = queries::end_vc_session(&data.db, discord_user_id, guild_id_i64, now)
                .await
                .unwrap_or_else(|e| {
                    error!(error = %e, "Failed to record VC leave");
                    None
                });

            let Some(join_time) = join_time else {
                return;
            };

            let duration = now.signed_duration_since(join_time);

            if duration.num_seconds() < 60 {
                // Ignore very short sessions to reduce noise from accidental joins, brief disconnects, etc.
                return;
            }

            // Call your XP logic
            handle_vc_xp(&data.db, data, discord_user_id, guild_id_i64, duration).await;
        }

        // User remained in trackable voice (including moving between normal voice channels).
        (true, true) => {}

        // User remained outside trackable voice (not in VC, or in AFK VC).
        (false, false) => {}
    }
}

fn calculate_integral_xp(total_minutes: f64, c_initial: f64) -> f64 {
    let x_limit = (c_initial - 0.5) / 0.0375;

    let f_integral = |t: f64| -> f64 {
        if t <= x_limit {
            c_initial * t - 0.01875 * (t * t)
        } else {
            // XP earned up to the limit + flat 0.5/min thereafter
            let xp_at_limit = c_initial * x_limit - 0.01875 * (x_limit * x_limit);
            xp_at_limit + 0.5 * (t - x_limit)
        }
    };

    f_integral(total_minutes)
}

pub async fn handle_vc_xp(
    pool: &PgPool,
    data: &Data,
    discord_user_id: i64,
    guild_id: i64,
    session_duration: chrono::Duration,
) {
    let session_mins = session_duration.num_minutes() as f64;
    if session_mins < 1.0 {
        return;
    }

    let now = Utc::now();
    let today = now.date_naive();

    let cache_key = (discord_user_id, guild_id, today);
    let prior_minutes_today = if let Some(cached) = data.vc_daily_minutes.get(&cache_key) {
        *cached.value()
    } else {
        let sessions_today =
            queries::get_vc_sessions_user_for_day(pool, discord_user_id, guild_id, now)
                .await
                .unwrap_or_default();

        let mut total_minutes: f64 = 0.0;
        let day_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let day_end = (today + chrono::Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        for s in sessions_today {
            let start = max(s.join_time, day_start);
            let end = min(s.leave_time.unwrap_or(now), day_end);
            let duration = end.signed_duration_since(start);

            if duration.num_seconds() > 0 {
                total_minutes += duration.num_minutes() as f64;
            }
        }

        total_minutes
    };

    // Update cache with new session included
    let updated_minutes = prior_minutes_today + session_mins;
    data.vc_daily_minutes.insert(cache_key, updated_minutes);

    let guild_config = get_guild_config_cached(pool, data, guild_id).await;
    let xp_config = XPConfig::new(guild_config.xp_config.clone());
    let total_xp_earned = if let Some(c_initial) = xp_config.rewards.get("voice_minutes").copied() {
        calculate_integral_xp(updated_minutes, c_initial)
            - calculate_integral_xp(prior_minutes_today, c_initial)
    } else {
        0.0
    };

    increment_vc_stat_with_custom_xp(
        pool,
        data,
        discord_user_id,
        guild_id,
        session_mins as i64,
        total_xp_earned,
    )
    .await;
}

/// Record command usage (called from command hook)
pub async fn record_command_usage(
    pool: &PgPool,
    data: &Data,
    discord_user_id: i64,
    guild_id: i64,
) -> Result<(), Error> {
    increment_stat_by(pool, data, discord_user_id, guild_id, "commands_used", 1).await;
    Ok(())
}

/// Increment a Discord stat by `by` units and immediately apply XP + event XP.
async fn increment_stat_by(
    pool: &PgPool,
    data: &Data,
    discord_user_id: i64,
    guild_id: i64,
    stat_name: &str,
    by: i64,
) {
    if by <= 0 {
        return;
    }

    let now = Utc::now();

    // ----------------------------------------------------
    // Lookup user
    // ----------------------------------------------------
    let user = match queries::get_user_by_discord_id(pool, discord_user_id, guild_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return;
        }
        Err(e) => {
            error!(error = %e, "failed to fetch user");
            return;
        }
    };

    // ----------------------------------------------------
    // Get current stat value (outside transaction — pool-only query)
    // ----------------------------------------------------
    let current = match queries::get_latest_discord_snapshot(pool, user.id, stat_name).await {
        Ok(Some(s)) => s.stat_value,
        Ok(None) => 0,
        Err(e) => {
            error!(error = %e, "failed to fetch snapshot");
            return;
        }
    };

    let new_value = current + by;

    // ----------------------------------------------------
    // Insert snapshot (pool-only query — must happen before transaction
    // so the cumulative value is recorded unconditionally, even if the
    // XP pipeline below aborts early due to no rewards being configured)
    // ----------------------------------------------------
    if let Err(e) = queries::insert_discord_snapshot(pool, user.id, stat_name, new_value, now).await
    {
        error!(error = %e, "failed to insert snapshot");
        return;
    }

    match queries::is_user_globally_banned(pool, user.id).await {
        Ok(true) => {
            debug!(
                user_id = user.id,
                "Skipping XP + event XP — user is globally banned."
            );
            return;
        }
        Ok(false) => {}
        Err(e) => {
            error!(error = %e, "failed to check modrec global ban status");
            return;
        }
    }

    // ----------------------------------------------------
    // Build stat delta and check whether it yields any XP before opening
    // a transaction — avoids a no-op transaction for untracked stats
    // ----------------------------------------------------
    let delta = StatDelta::new(user.id, stat_name.to_string(), current, new_value);

    if delta.difference <= 0 {
        return;
    }

    let guild_config = get_guild_config_cached(pool, data, guild_id).await;

    let xp_config = XPConfig::new(guild_config.xp_config.clone());

    // ----------------------------------------------------
    // Calculate XP rewards up-front so we can skip the transaction
    // entirely when the stat is not configured for XP
    // ----------------------------------------------------
    let rewards = calculate_xp_rewards(&[delta.clone()], &xp_config);

    let total_xp: f64 = rewards.iter().map(|r| r.xp_earned).sum();

    // ----------------------------------------------------
    // Begin transaction — wraps stat_delta + xp_events + xp + level
    // ----------------------------------------------------
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!(error = %e, "failed to start transaction");
            return;
        }
    };

    // ----------------------------------------------------
    // Insert stat_delta row
    // ----------------------------------------------------
    let delta_id = match queries::insert_stat_delta_in_tx(
        &mut tx,
        user.id,
        stat_name,
        current,
        new_value,
        delta.difference,
        "discord",
        &now,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            error!(error = %e, "failed inserting stat delta");
            return;
        }
    };

    // ----------------------------------------------------
    // Insert one xp_event row per reward
    // ----------------------------------------------------
    for reward in &rewards {
        if let Err(e) = queries::insert_xp_event_in_tx(
            &mut tx,
            user.id,
            &reward.stat_name,
            delta_id,
            reward.units as i32,
            reward.xp_per_unit,
            reward.xp_earned,
            &now,
        )
        .await
        {
            error!(error = %e, "failed inserting xp event");
            return;
        }
    }

    // ----------------------------------------------------
    // Upsert XP and recalculate level inside the transaction — makes the XP
    // increment atomic with stat_delta + xp_events so a crash between commit
    // and increment_xp can no longer leave a user with missing XP.
    // ----------------------------------------------------
    if total_xp > 0.0 {
        if let Err(e) = sqlx::query(
            "INSERT INTO xp (user_id, total_xp, last_updated)
             VALUES ($1, $2, $3)
             ON CONFLICT(user_id) DO UPDATE SET
                 total_xp = xp.total_xp + excluded.total_xp,
                 last_updated = excluded.last_updated",
        )
        .bind(user.id)
        .bind(total_xp)
        .bind(&now)
        .execute(&mut *tx)
        .await
        {
            error!(error = %e, "failed upserting xp in transaction");
            return;
        }

        let xp_row = match sqlx::query_as::<_, crate::database::models::DbXP>(
            "SELECT * FROM xp WHERE user_id = $1",
        )
        .bind(user.id)
        .fetch_optional(&mut *tx)
        .await
        {
            Ok(Some(row)) => row,
            Ok(None) => {
                error!(
                    user_id = user.id,
                    "xp row missing after upsert in transaction"
                );
                return;
            }
            Err(e) => {
                error!(error = %e, "failed fetching xp row in transaction");
                return;
            }
        };

        let new_level = calculate_level(
            xp_row.total_xp,
            data.config.base_level_xp,
            data.config.level_exponent,
        );

        if new_level != xp_row.level {
            if let Err(e) =
                sqlx::query("UPDATE xp SET level = $1, last_updated = $2 WHERE user_id = $3")
                    .bind(new_level)
                    .bind(&now)
                    .bind(user.id)
                    .execute(&mut *tx)
                    .await
            {
                error!(error = %e, "failed updating level in transaction");
                return;
            }
        }
    }

    // ----------------------------------------------------
    // Commit transaction — stat_delta + xp_events + xp + level are now durable
    // ----------------------------------------------------
    if let Err(e) = tx.commit().await {
        error!(error = %e, "transaction commit failed");
        return;
    }

    // ----------------------------------------------------
    // Award event XP for this delta (post-commit, pool-only).
    // Event XP cannot run inside the TX because award_event_xp_for_delta
    // uses the pool directly; its own level update is handled below.
    // ----------------------------------------------------
    let event_xp = match queries::award_event_xp_for_delta(
        pool,
        guild_id,
        user.id,
        stat_name,
        delta_id,
        delta.difference,
        &now,
    )
    .await
    {
        Ok(xp) => xp,
        Err(e) => {
            error!(error = %e, "failed to award event XP");
            0.0
        }
    };

    // ----------------------------------------------------
    // Event XP tracking (separate from global XP)
    // ----------------------------------------------------
    // Event XP is tracked separately in the event_xp table for event-specific leaderboards.
    // It should NOT be added to the user's global total_xp to avoid double-counting.
    // The regular XP (from guild multipliers) was already added inside the transaction above.
    // Event leaderboards query the event_xp table directly via get_event_leaderboard().
    if event_xp > 0.0 {
        debug!(
            user_id = user.id,
            event_xp,
            "Discord activity: Event XP awarded and recorded in event_xp table (not added to global total)."
        );
    }

    debug!(
        user_id = user.id,
        stat_name,
        new_value,
        xp_awarded = total_xp,
        event_xp_awarded = event_xp,
        "Discord stat processed"
    );
}

async fn increment_vc_stat_with_custom_xp(
    pool: &PgPool,
    data: &Data,
    discord_user_id: i64,
    guild_id: i64,
    minutes: i64,
    xp_earned: f64,
) {
    if minutes <= 0 {
        return;
    }

    let now = Utc::now();
    let stat_name = "voice_minutes";

    let user = match queries::get_user_by_discord_id(pool, discord_user_id, guild_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return;
        }
        Err(e) => {
            error!(error = %e, "failed to fetch user for VC XP");
            return;
        }
    };

    let current = match queries::get_latest_discord_snapshot(pool, user.id, stat_name).await {
        Ok(Some(s)) => s.stat_value as i64,
        Ok(None) => 0,
        Err(e) => {
            error!(error = %e, "failed to fetch voice_minutes snapshot");
            return;
        }
    };

    let new_value = current + minutes;

    if let Err(e) = queries::insert_discord_snapshot(pool, user.id, stat_name, new_value, now).await
    {
        error!(error = %e, "failed to insert voice_minutes snapshot");
        return;
    }

    match queries::is_user_globally_banned(pool, user.id).await {
        Ok(true) => {
            debug!(
                user_id = user.id,
                "Skipping VC XP — user is globally banned."
            );
            return;
        }
        Ok(false) => {}
        Err(e) => {
            error!(error = %e, "failed to check global ban status");
            return;
        }
    }

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!(error = %e, "failed to start VC XP transaction");
            return;
        }
    };

    let delta_id = match queries::insert_stat_delta_in_tx(
        &mut tx, user.id, stat_name, current, new_value, minutes, "discord", &now,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            error!(error = %e, "failed inserting VC stat delta");
            return;
        }
    };

    let xp_per_unit = if minutes > 0 {
        xp_earned / (minutes as f64)
    } else {
        0.0
    };

    if xp_earned > 0.0 {
        if let Err(e) = queries::insert_xp_event_in_tx(
            &mut tx,
            user.id,
            stat_name,
            delta_id,
            minutes as i32,
            xp_per_unit,
            xp_earned,
            &now,
        )
        .await
        {
            error!(error = %e, "failed inserting VC xp event");
            return;
        }
    }

    if xp_earned > 0.0 {
        if let Err(e) = sqlx::query(
            "INSERT INTO xp (user_id, total_xp, last_updated)
             VALUES ($1, $2, $3)
             ON CONFLICT(user_id) DO UPDATE SET
                 total_xp = xp.total_xp + excluded.total_xp,
                 last_updated = excluded.last_updated",
        )
        .bind(user.id)
        .bind(xp_earned)
        .bind(&now)
        .execute(&mut *tx)
        .await
        {
            error!(error = %e, "failed upserting VC xp");
            return;
        }

        // Fetch updated XP total
        let xp_row = match sqlx::query_as::<_, crate::database::models::DbXP>(
            "SELECT * FROM xp WHERE user_id = $1",
        )
        .bind(user.id)
        .fetch_optional(&mut *tx)
        .await
        {
            Ok(Some(row)) => row,
            Ok(None) => {
                error!(user_id = user.id, "xp row missing after VC upsert");
                return;
            }
            Err(e) => {
                error!(error = %e, "failed fetching xp row for VC");
                return;
            }
        };

        // Recalculate level
        let new_level = calculate_level(
            xp_row.total_xp,
            data.config.base_level_xp,
            data.config.level_exponent,
        );

        if new_level != xp_row.level {
            if let Err(e) =
                sqlx::query("UPDATE xp SET level = $1, last_updated = $2 WHERE user_id = $3")
                    .bind(new_level)
                    .bind(&now)
                    .bind(user.id)
                    .execute(&mut *tx)
                    .await
            {
                error!(error = %e, "failed updating level for VC");
                return;
            }
        }
    }

    if let Err(e) = tx.commit().await {
        error!(error = %e, "VC XP transaction commit failed");
        return;
    }

    let event_xp = if xp_earned > 0.0 {
        match queries::award_event_xp_for_delta(
            pool, guild_id, user.id, stat_name, delta_id, minutes, &now,
        )
        .await
        {
            Ok(xp) => xp,
            Err(e) => {
                error!(error = %e, "failed to award VC event XP");
                0.0
            }
        }
    } else {
        0.0
    };

    debug!(
        user_id = user.id,
        minutes,
        new_value,
        xp_awarded = xp_earned,
        event_xp_awarded = event_xp,
        "Voice channel activity processed"
    );
}
