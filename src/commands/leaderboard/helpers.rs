/// Shared leaderboard generation logic.
///
/// Provides helpers that query the database, fetch avatars, and render
/// leaderboard page images and the standalone milestone card.
/// Used by both the `/leaderboard` user command and the persistent
/// leaderboard background updater.
use sqlx::PgPool;

use crate::cards::leaderboard_card::{
    self, LeaderboardCardParams, LeaderboardRow, MilestoneCardParams, MilestoneEntry,
};
use crate::database::queries;

/// Players per leaderboard page (fixed).
pub const PAGE_SIZE: i64 = 10;

/// Generate a leaderboard PNG for a specific page of a guild.
///
/// Returns `(png_bytes, total_pages)`.
pub async fn generate_leaderboard_page(
    pool: &PgPool,
    guild_id: i64,
    page: u32,
) -> Result<(Vec<u8>, u32), Box<dyn std::error::Error + Send + Sync>> {
    let total_users = queries::count_users_in_guild(pool, guild_id).await?;
    let total_pages = ((total_users as f64) / PAGE_SIZE as f64).ceil().max(1.0) as u32;

    let clamped_page = page.clamp(1, total_pages);
    let offset = ((clamped_page - 1) as i64) * PAGE_SIZE;

    let entries = queries::get_leaderboard(pool, guild_id, offset, PAGE_SIZE).await?;

    // Fetch avatars concurrently.
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let avatar_futures: Vec<_> = entries
        .iter()
        .map(|entry| {
            let url = format!("https://minotar.net/avatar/{}/{}", entry.minecraft_uuid, 80);
            let client = http.clone();
            async move {
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        resp.bytes().await.ok().map(|b| b.to_vec())
                    }
                    _ => None,
                }
            }
        })
        .collect();

    let avatars = futures::future::join_all(avatar_futures).await;

    let rows: Vec<LeaderboardRow> = entries
        .iter()
        .zip(avatars.into_iter())
        .enumerate()
        .map(|(i, (entry, avatar))| {
            let rank = offset as u32 + i as u32 + 1;
            let username = entry
                .minecraft_username
                .clone()
                .unwrap_or_else(|| format!("User#{}", entry.discord_user_id));
            LeaderboardRow {
                rank,
                username,
                level: entry.level,
                total_xp: entry.total_xp,
                avatar_bytes: avatar,
                hypixel_rank: entry.hypixel_rank.clone(),
                hypixel_rank_plus_color: entry.hypixel_rank_plus_color.clone(),
            }
        })
        .collect();

    let params = LeaderboardCardParams {
        rows,
        page: clamped_page,
        total_pages,
    };

    let png_bytes = leaderboard_card::render(&params);
    Ok((png_bytes, total_pages))
}

/// Generate a standalone milestone card PNG for a guild.
///
/// Returns the PNG bytes. Non-fatal errors (e.g. empty milestone list) still
/// produce a valid card with an appropriate empty-state message.
pub async fn generate_milestone_card(
    pool: &PgPool,
    guild_id: i64,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let milestone_data = queries::get_milestones_with_counts(pool, guild_id)
        .await
        .unwrap_or_default();

    let milestones: Vec<MilestoneEntry> = milestone_data
        .into_iter()
        .map(|m| MilestoneEntry {
            level: m.level,
            user_count: m.user_count,
        })
        .collect();

    let total_users = queries::count_users_in_guild(pool, guild_id).await?;

    let params = MilestoneCardParams {
        milestones,
        total_users,
    };

    Ok(leaderboard_card::render_milestone_card(&params))
}
