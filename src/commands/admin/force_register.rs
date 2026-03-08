/// `/force_register` command.
///
/// Allows an admin to forcibly register a Discord user to a Minecraft account,
/// bypassing Hypixel Discord verification. Use only if the normal registration
/// process is failing for legitimate users.
use tracing::{error, info};

use poise::serenity_prelude::{self as serenity, CreateEmbed};

use crate::config::GuildConfig;
use crate::database::queries;
use crate::shared::types::{Context, Error};
use crate::commands::registration::register::fetch_and_cache_head_texture;

/// Forcibly register a user, bypassing Hypixel Discord verification.
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn force_register(
    ctx: Context<'_>,
    #[description = "Discord user to register"] user: serenity::User,
    #[description = "Minecraft username"] minecraft_username: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in a server")?;

    let guild_id_i64 = guild_id.get() as i64;
    let discord_user_id = user.id.get() as i64;

    queries::upsert_guild(&ctx.data().db, guild_id_i64).await?;

    let guild_row = queries::get_guild(&ctx.data().db, guild_id_i64).await?;
    let guild_config: GuildConfig = guild_row
        .as_ref()
        .map(|g| serde_json::from_value(g.config_json.clone()).unwrap_or_default())
        .unwrap_or_default();

    let profile = ctx
        .data()
        .hypixel
        .resolve_username(&minecraft_username)
        .await
        .map_err(|e| format!("Could not resolve Minecraft username: {e}"))?;

    let player_data = ctx
        .data()
        .hypixel
        .fetch_player(&profile.id)
        .await
        .map_err(|e| format!("Could not fetch Hypixel player data: {e}"))?;

    let role_id = match guild_config.registered_role_id {
        Some(id) => id,
        None => {
            let embed = CreateEmbed::default()
                .title("Registration Failed")
                .color(0xFF4444)
                .description("Registration is not configured on this server. An administrator must set a registered role first.");
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
            return Ok(());
        }
    };

    if let Some(existing_user) =
        queries::get_user_by_discord_id(&ctx.data().db, discord_user_id, guild_id_i64).await?
    {
        let embed = CreateEmbed::default()
            .title("Already Registered")
            .color(0xFFAA00)
            .description(format!(
                "User is already registered as **{}** (UUID `{}`). If you want to change the linked Minecraft account, please unregister first with `/unregister`.",
                existing_user.minecraft_uuid, existing_user.minecraft_uuid
            ));
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let role = serenity::RoleId::new(role_id);
    let member = guild_id.member(&ctx.serenity_context().http, user.id).await?;

    if let Err(e) = member.add_role(&ctx.serenity_context().http, role).await {
        error!(
            guild_id = guild_id_i64,
            discord_user_id,
            role_id,
            error = %e,
            "Failed to assign registered role"
        );

        let embed = CreateEmbed::default()
            .title("Registration Failed")
            .color(0xFF4444)
            .description("I couldn't assign the registered role. Please ensure I have **Manage Roles** permission and my role is above the registered role.");
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let now = chrono::Utc::now();

    let db_user = queries::register_user(
        &ctx.data().db,
        discord_user_id,
        profile.id,
        &profile.name,
        guild_id_i64,
        now,
    )
    .await?;

    queries::update_user_hypixel_rank(
        &ctx.data().db,
        db_user.id,
        player_data.rank.as_db_str(),
        player_data.rank_plus_color.as_deref(),
    ).await?;

    // Insert stat snapshots as in normal registration
    let bw = &player_data.bedwars;
    for (stat_name, value) in &bw.stats {
        queries::insert_hypixel_snapshot(&ctx.data().db, db_user.id, stat_name, *value, now).await?;
    }
    for stat_name in &["messages_sent", "reactions_added", "commands_used"] {
        queries::insert_discord_snapshot(&ctx.data().db, db_user.id, stat_name, 0.0, now).await?;
    }

    let _ = fetch_and_cache_head_texture(&ctx.data().db, db_user.id, &profile.id).await;

    info!(
        discord_user_id,
        minecraft_uuid = %profile.id,
        minecraft_name = %profile.name,
        "User forcibly registered by admin"
    );

    let embed = CreateEmbed::default()
        .title("Force Registration Successful")
        .color(0x00BFFF)
        .description(format!(
            "User <@{}> has been forcibly registered as **{}** (UUID `{}`).",
            user.id, profile.name, profile.id
        ));
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
