/// `/set-afk-vc` command — admin only.
///
/// Sets the guild AFK voice channel. Time spent in this channel is excluded
/// from `voice_minutes` tracking.
use poise::serenity_prelude::{self as serenity, CreateEmbed};
use tracing::{debug, info};

use crate::commands::logger::logger::{LogType, logger};
use crate::config::GuildConfig;
use crate::database::queries;
use crate::shared::types::{Context, Error};

/// Set the voice channel that should be treated as AFK (not counted for VC minutes).
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    rename = "set-afk-vc",
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set_afk_vc(
    ctx: Context<'_>,
    #[description = "Voice channel to exclude from VC minutes"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in a server")?;
    let guild_id_i64 = guild_id.get() as i64;
    let data = ctx.data();

    if channel.kind != serenity::ChannelType::Voice && channel.kind != serenity::ChannelType::Stage
    {
        ctx.say("Please select a voice channel.").await?;
        return Ok(());
    }

    debug!(
        "Invoked /set-afk-vc for guild {} with channel {} ({})",
        guild_id, channel.name, channel.id
    );

    queries::upsert_guild(&data.db, guild_id_i64).await?;

    let guild_row = queries::get_guild(&data.db, guild_id_i64).await?;
    let mut guild_config: GuildConfig = guild_row
        .as_ref()
        .and_then(|g| serde_json::from_value(g.config_json.clone()).ok())
        .unwrap_or_default();

    guild_config.afk_voice_channel_id = Some(channel.id.get());

    let config_json = serde_json::to_value(&guild_config)?;
    queries::update_guild_config(&data.db, guild_id_i64, config_json).await?;
    data.guild_configs
        .insert(guild_id_i64, (guild_config, std::time::Instant::now()));

    let embed = CreateEmbed::default()
        .title("AFK VC Updated")
        .color(0x00BFFF)
        .description(format!(
            "AFK voice channel set to <#{}>. Time spent there will not count toward `voice_minutes`.",
            channel.id
        ));
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    info!(
        "Updated AFK VC for guild {} to {} ({})",
        guild_id, channel.name, channel.id
    );

    logger(
        ctx.serenity_context(),
        data,
        guild_id,
        LogType::Info,
        format!(
            "{} set AFK VC channel to {} ({})",
            ctx.author().name,
            channel.name,
            channel.id
        ),
    )
    .await?;

    Ok(())
}

/// Clear the AFK voice channel so all voice channels count toward VC minutes.
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    rename = "clear-afk-vc",
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn clear_afk_vc(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in a server")?;
    let guild_id_i64 = guild_id.get() as i64;
    let data = ctx.data();

    queries::upsert_guild(&data.db, guild_id_i64).await?;

    let guild_row = queries::get_guild(&data.db, guild_id_i64).await?;
    let mut guild_config: GuildConfig = guild_row
        .as_ref()
        .and_then(|g| serde_json::from_value(g.config_json.clone()).ok())
        .unwrap_or_default();

    guild_config.afk_voice_channel_id = None;

    let config_json = serde_json::to_value(&guild_config)?;
    queries::update_guild_config(&data.db, guild_id_i64, config_json).await?;
    data.guild_configs
        .insert(guild_id_i64, (guild_config, std::time::Instant::now()));

    let embed = CreateEmbed::default()
        .title("AFK VC Cleared")
        .color(0x00BFFF)
        .description(
            "AFK voice channel cleared. All voice channels now count toward `voice_minutes`.",
        );
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    info!("Cleared AFK VC for guild {}", guild_id);

    logger(
        ctx.serenity_context(),
        data,
        guild_id,
        LogType::Info,
        format!("{} cleared AFK VC channel", ctx.author().name),
    )
    .await?;

    Ok(())
}
