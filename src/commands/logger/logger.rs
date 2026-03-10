use crate::database::queries;
use crate::shared::types::Data;
use poise::serenity_prelude::{self as serenity, CreateEmbed};

pub enum LogType {
    Info,
    Warn,
    Debug,
    Error,
}

pub async fn logger(
    ctx: &serenity::Context,
    data: &Data,
    guild_id: serenity::GuildId,
    log_type: LogType,
    msg: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let guild_id_i64 = guild_id.get() as i64;

    // Fetch configured log channel
    let Some(channel_id) = queries::get_guild_log_channel(&data.db, guild_id_i64).await? else {
        return Ok(()); // logging not configured
    };

    let channel_id = serenity::ChannelId::new(channel_id as u64);

    let embed = match log_type {
        LogType::Info => CreateEmbed::default()
            .title("ℹ️ Info")
            .description(msg)
            .color(0x3498db),

        LogType::Warn => CreateEmbed::default()
            .title("⚠️ Warning")
            .description(msg)
            .color(0xf1c40f),

        LogType::Debug => CreateEmbed::default()
            .title("🐛 Debug")
            .description(msg)
            .color(0x95a5a6),

        LogType::Error => CreateEmbed::default()
            .title("❌ Error")
            .description(msg)
            .color(0xe74c3c),
    };

    channel_id
        .send_message(ctx, serenity::CreateMessage::default().embed(embed))
        .await?;

    Ok(())
}

pub async fn logger_system(
    http: &serenity::Http,
    pool: &sqlx::PgPool,
    guild_id: i64,
    log_type: LogType,
    msg: String,
) {
    let channel_id = match queries::get_guild_log_channel(pool, guild_id).await {
        Ok(Some(id)) => id,
        _ => return, // logging not configured
    };

    let channel = serenity::ChannelId::new(channel_id as u64);

    let embed = match log_type {
        LogType::Info => serenity::CreateEmbed::new()
            .title("ℹ️ Info")
            .description(msg)
            .color(0x3498db),

        LogType::Warn => serenity::CreateEmbed::new()
            .title("⚠️ Warning")
            .description(msg)
            .color(0xf1c40f),

        LogType::Error => serenity::CreateEmbed::new()
            .title("❌ Error")
            .description(msg)
            .color(0xe74c3c),

        LogType::Debug => serenity::CreateEmbed::new()
            .title("🐛 Debug")
            .description(msg)
            .color(0x95a5a6),
    };

    let _ = channel
        .send_message(http, serenity::CreateMessage::new().embed(embed))
        .await;
}
