/// `/register` command.
///
/// Links a Discord user to their Minecraft account by resolving the username
/// to a UUID via the Mojang API, storing the mapping in the database, and
/// assigning the guild's configured registered role.
use time::OffsetDateTime;
use tracing::{debug, info, error};

use poise::serenity_prelude::{self as serenity, CreateEmbed};
use sqlx::SqlitePool;

use crate::config::GuildConfig;
use crate::database::queries;
use crate::shared::types::{Context, Data, Error};
use base64::{engine::general_purpose, Engine as _};

/// Core registration logic, shared between `/register` and the Register button.
///
/// Resolves `minecraft_username` via Mojang, verifies ownership via Hypixel
/// social links, assigns the guild's registered role, inserts the user record,
/// and seeds baseline stat snapshots.
///
/// Returns a user-facing message string describing either success or the reason
/// for failure. The caller is responsible for sending that message as a reply.
pub async fn perform_registration(
    serenity_ctx: &serenity::Context,
    data: &Data,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    user_tag: &str,
    minecraft_username: &str,
) -> Result<(String, Option<(i64, String)>), Error> {
    let guild_id_i64 = guild_id.get() as i64;
    let discord_user_id = user_id.get() as i64;

    queries::upsert_guild(&data.db, guild_id_i64).await?;

    let guild_row = queries::get_guild(&data.db, guild_id_i64).await?;
    let guild_config: GuildConfig = guild_row
        .as_ref()
        .map(|g| serde_json::from_str(&g.config_json).unwrap_or_default())
        .unwrap_or_default();

    let profile = data
        .hypixel
        .resolve_username(minecraft_username)
        .await
        .map_err(|e| format!("Could not resolve Minecraft username: {e}"))?;

    let player_data = data
        .hypixel
        .fetch_player(&profile.id)
        .await
        .map_err(|e| format!("Could not fetch Hypixel player data: {e}"))?;

    match player_data.social_links.get("DISCORD") {
        Some(linked) => {
            if linked != user_tag {
                return Ok((
                    format!(
                        "Ownership verification failed.\n\n\
                    Hypixel account **{}** is linked to Discord `{}` but you are `{}`.\n\
                    Please update your Hypixel social link to match your Discord.",
                        profile.name, linked, user_tag
                    ),
                    None,
                ));
            }
        }
        None => {
            return Ok((
                "Ownership verification failed.\n\n\
                 Your Hypixel account must have a **Discord social link** set.\n\
                 Please link your Discord in Hypixel:\n\
                 `/socials discord <your discord>`"
                    .to_string(),
                None,
            ));
        }
    }

    let role_id = match guild_config.registered_role_id {
        Some(id) => id,
        None => {
            debug!(
                "Guild {} attempted registration but has no registered role configured.",
                guild_id_i64
            );
            return Ok((
                "Registration is not configured on this server. \
                An administrator must set a registered role first."
                    .to_string(),
                None,
            ));
        }
    };

    if let Some(existing_user) =
        queries::get_user_by_discord_id(&data.db, discord_user_id, guild_id_i64).await?
    {
        debug!(
            "User {} attempted to register but is already registered as {} in guild {}.",
            discord_user_id, existing_user.minecraft_uuid, guild_id_i64
        );
        return Ok((
            format!(
                "You are already registered as **{}** (UUID `{}`). \
                If you want to change your linked Minecraft account, please unregister first with `/unregister`.",
                existing_user.minecraft_uuid, existing_user.minecraft_uuid
            ),
            None,
        ));
    }

    let role = serenity::RoleId::new(role_id);
    let member = guild_id.member(&serenity_ctx.http, user_id).await?;

    if let Err(e) = member.add_role(&serenity_ctx.http, role).await {
        error!(
            "Failed to assign registered role to user {} in guild {}: {}",
            discord_user_id, guild_id_i64, e
        );
        return Ok((
            "I couldn't assign the registered role. \
            Please ensure I have **Manage Roles** permission and my role is above the registered role."
                .to_string(),
            None,
        ));
    }

    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());

    let db_user = queries::register_user(
        &data.db,
        discord_user_id,
        &profile.id,
        &profile.name,
        guild_id_i64,
        &now,
    )
    .await?;

    let bw = &player_data.bedwars;
    for (stat_name, value) in &bw.stats {
        queries::insert_hypixel_snapshot(&data.db, db_user.id, stat_name, *value, &now).await?;
    }

    for stat_name in &["messages_sent", "reactions_added", "commands_used"] {
        queries::insert_discord_snapshot(&data.db, db_user.id, stat_name, 0.0, &now).await?;
    }

    queries::upsert_xp(&data.db, db_user.id, 0.0, &now).await?;

    info!(
        discord_user_id,
        minecraft_uuid = %profile.id,
        minecraft_name = %profile.name,
        "User registered."
    );

    Ok((
        format!(
            "You have been registered as **{}** (UUID `{}`). \
            You can now start earning XP and tracking your stats!",
            profile.name, profile.id
        ),
        Some((db_user.id, profile.id)),
    ))
}

pub async fn fetch_and_cache_head_texture(
    pool: &SqlitePool,
    user_id: i64,
    uuid: &str,
) -> Option<String> {
    // Construct the URL you want to fetch. Minotar is convenient:
    // let url = format!("https://minotar.net/helm/{}/64.png", uuid);
    // If you have a different API for textures, use that URL.

    let url = format!("https://minotar.net/avatar/{}/128", uuid);

    let resp = match reqwest::get(&url).await {
        Ok(r) => r,
        Err(_) => return None,
    };
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(_) => return None,
    };
    // Convert to base64 and build a data URL
    let b64 = general_purpose::STANDARD.encode(&bytes);
    let data_url = format!("data:image/png;base64,{}", b64);

    // store in DB
    let updated_at = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .ok();
    if let Some(ts) = updated_at {
        let _ = queries::set_user_head_texture(pool, user_id, &data_url, &ts).await;
    }

    Some(data_url)
}

/// Register your Minecraft account to start tracking stats and earning XP.
#[poise::command(slash_command, guild_only)]
pub async fn register(
    ctx: Context<'_>,
    #[description = "Your Minecraft username"] minecraft_username: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in a server")?;

    let (msg, user_data) = perform_registration(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        ctx.author().id,
        &ctx.author().tag(),
        &minecraft_username,
    )
    .await?;

    if let Some((db_user_id, uuid)) = user_data {
        let _ = fetch_and_cache_head_texture(&ctx.data().db, db_user_id, &uuid).await;
    }

    // Detect success by looking for the phrase we set in the success branch.
    let success = msg.contains("You have been registered");
    let embed = CreateEmbed::default()
        .title(if success {
            "Registration Successful"
        } else {
            "Registration Failed"
        })
        .color(if success { 0x00BFFF } else { 0xFF4444 })
        .description(msg);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
