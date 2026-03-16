/// "/unregister" command.
/// Unregisters the user by deleting their row from the database and removing the registered role (if they have it).
use poise::serenity_prelude::{self as serenity, CreateEmbed};
use tracing::{debug, info};

use crate::config::GuildConfig;
use crate::database::queries;
use crate::shared::types::{Context, Error};

/// Unregister your Minecraft account and stop tracking stats and earning XP.
///
/// This is a *soft* unregister: your user row and history remain in the database,
/// but you will be marked inactive so tracking/leaderboards can ignore you.
/// If you register again later, your stats will still be present.
#[poise::command(slash_command, guild_only)]
pub async fn unregister(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in a server")?;

    let guild_id_i64 = guild_id.get() as i64;
    let discord_user_id = ctx.author().id.get() as i64;
    let data = ctx.data();

    let guild_row = queries::get_guild(&data.db, guild_id_i64)
        .await?
        .ok_or("Guild configuration not found. Ask an admin to configure the bot.")?;

    let guild_config: GuildConfig =
        serde_json::from_value(guild_row.config_json.clone()).unwrap_or_default();

    // Soft-unregister: mark inactive instead of deleting (preserves history and avoids FK issues).
    let now = chrono::Utc::now();
    queries::mark_user_inactive(&data.db, discord_user_id, guild_id_i64, &now).await?;

    info!("User {} unregistered (marked inactive)", ctx.author().id);

    if let Some(role_id) = guild_config.registered_role_id {
        let role = serenity::RoleId::new(role_id);

        let role_exists = ctx
            .guild()
            .map(|g| g.roles.contains_key(&role))
            .unwrap_or(false);

        // check role exists in cached guild
        if !role_exists {
            debug!(
                role_id,
                guild_id = guild_id_i64,
                "Registered role not found in guild, skipping role removal"
            );
            let embed = CreateEmbed::default()
                .title("Unregistered")
                .color(0xFFAA00)
                .description(
                    "You have been unregistered, but I couldn't find the registered role in \
                     the server. Please ask an administrator to update the configuration and \
                     remove the role manually if desired.",
                );
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
            return Ok(());
        }

        // remove role from the member
        let member = guild_id.member(ctx.http(), ctx.author().id).await?;

        if member.roles.contains(&role) {
            member.remove_role(ctx.http(), role).await?;
        }
    }

    let embed = CreateEmbed::default()
        .title("Unregistered")
        .color(0x00BFFF)
        .description(
            "You have been successfully unregistered.\n\
             You are now marked as **inactive** and your stats will no longer be tracked.\n\
             If you register again later, your previous stats will still be present.",
        );
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
