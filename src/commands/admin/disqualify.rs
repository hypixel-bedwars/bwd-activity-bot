use ::serenity::all::ChannelId;
use chrono::{Duration, Utc};
use poise::serenity_prelude::{self as serenity, CreateEmbed};
use tracing::info;

use crate::commands::logger::logger::{LogType, logger};
use crate::database::queries;
use crate::shared::types::{Context, Error};

const LOG_CHANNEL: u64 = 1486217374309941308;

async fn autocomplete_event_name<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<serenity::AutocompleteChoice> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id.get() as i64,
        None => return Vec::new(),
    };

    let events = queries::list_events_by_status(&ctx.data().db, guild_id, "active")
        .await
        .unwrap_or_default();

    let partial_lower = partial.to_lowercase();

    events
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&partial_lower))
        .take(25)
        .map(|e| serenity::AutocompleteChoice::new(e.name.clone(), e.id.to_string()))
        .collect()
}

#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    required_permissions = "BAN_MEMBERS",
    default_member_permissions = "BAN_MEMBERS"
)]
pub async fn disqualify(
    ctx: Context<'_>,
    user: serenity::User,
    #[autocomplete = "autocomplete_event_name"] event: Option<i64>,
    reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This command must be run in a server")?;
    let guild_i64 = guild_id.get() as i64;
    let pool = &ctx.data().db;

    // Load user
    let db_user = queries::get_user_by_discord_id_any(pool, user.id.get() as i64, guild_i64)
        .await?
        .ok_or("User is not registered in this guild")?;

    if let Some(event_id) = event {
        let event_row = queries::get_event_by_id(pool, event_id)
            .await?
            .ok_or("Event not found")?;

        let already = queries::is_user_disqualified_from_event(pool, event_id, db_user.id).await?;

        if already {
            ctx.say("User is already disqualified from this event.")
                .await?;
            return Ok(());
        }

        queries::disqualify_user_from_event(
            pool,
            event_id,
            db_user.id,
            ctx.author().id.get() as i64,
            guild_i64,
            reason.as_deref(),
        )
        .await?;

        let embed = CreateEmbed::new()
            .title("Event Disqualification Applied")
            .description(format!(
                "User <@{}> has been disqualified from **{}**.\nReason: {}",
                user.id,
                event_row.name,
                reason.as_deref().unwrap_or("No reason provided.")
            ))
            .color(0xFFA500);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;

        info!(
            admin = %ctx.author().name,
            target = %user.id,
            event_id = event_row.id,
            "Applied event DQ"
        );

        let _ = logger(
            ctx.serenity_context(),
            ctx.data(),
            guild_id,
            LogType::Warn,
            format!(
                "{} disqualified <@{}> from event {}",
                ctx.author().name,
                user.id,
                event_row.name
            ),
        )
        .await;

        let embed = CreateEmbed::new()
            .title("⚠️ Event Disqualification")
            .color(0xFFA500)
            .fields(vec![
                ("User", format!("<@{}>", user.id), true),
                ("Moderator", ctx.author().name.clone(), true),
                ("Event", event_row.name.clone(), true),
                (
                    "Reason",
                    reason
                        .as_deref()
                        .unwrap_or("No reason provided.")
                        .to_string(),
                    false,
                ),
            ])
            .timestamp(chrono::Utc::now());

        let _ = ChannelId::new(LOG_CHANNEL)
            .send_message(&ctx.http(), serenity::CreateMessage::new().embed(embed))
            .await;
    }

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    required_permissions = "BAN_MEMBERS",
    default_member_permissions = "BAN_MEMBERS"
)]
pub async fn undisqualify(
    ctx: Context<'_>,
    #[description = "User to re-qualify"] user: serenity::User,
    #[autocomplete = "autocomplete_event_name"]
    #[description = "Event ID"]
    event: i64,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be in a server")?;
    let guild_i64 = guild_id.get() as i64;
    let pool = &ctx.data().db;

    let db_user = queries::get_user_by_discord_id_any(pool, user.id.get() as i64, guild_i64)
        .await?
        .ok_or("User not found")?;

    let event_row = queries::get_event_by_id(pool, event)
        .await?
        .ok_or("Event not found")?;

    if event_row.guild_id != guild_i64 {
        ctx.say("That event does not belong to this guild.").await?;
        return Ok(());
    }

    queries::requalify_user_for_event(
        pool,
        event,
        db_user.id,
        ctx.author().id.get() as i64,
        guild_i64,
        None,
    )
    .await?;

    let embed = CreateEmbed::new()
        .title("Event Requalification")
        .description(format!(
            "<@{}> is no longer disqualified from event {}.",
            user.id, event
        ))
        .color(0x00FF00);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    let _ = logger(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        LogType::Info,
        format!("{} undisqualified <@{}>", ctx.author().name, user.id),
    )
    .await;

    let embed = CreateEmbed::new()
        .title("⚠️ Disqualification Removed")
        .color(0x00FF00)
        .fields(vec![
            ("User", format!("<@{}>", user.id), true),
            ("Moderator", ctx.author().name.clone(), true),
            ("Event", event.to_string(), true),
        ])
        .timestamp(chrono::Utc::now());

    let _ = ChannelId::new(LOG_CHANNEL)
        .send_message(&ctx.http(), serenity::CreateMessage::new().embed(embed))
        .await;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    required_permissions = "BAN_MEMBERS",
    default_member_permissions = "BAN_MEMBERS"
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "User to ban"] user: serenity::User,
    #[description = "Reason for ban"] reason: Option<String>,
    #[description = "Duration of ban, e.g use 1d, 2w, 1y (leave empty for permanent)"]
    input_duration: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be in a server")?;
    let guild_i64 = guild_id.get() as i64;
    let pool = &ctx.data().db;

    let duration: Option<Duration> = if let Some(input) = input_duration {
        let input = input.trim();

        if input.len() < 2 {
            ctx.say("Invalid duration format.").await?;
            return Ok(());
        }

        let (num_part, unit) = input.split_at(input.len() - 1);

        let value = match num_part.parse::<i64>() {
            Ok(n) => n,
            Err(_) => {
                ctx.say("Invalid duration format. Use `1h`, `1d`, `2w`, or `1y`.")
                    .await?;
                return Ok(());
            }
        };

        let dur = match unit {
            "h" => Duration::hours(value),
            "d" => Duration::days(value),
            "w" => Duration::weeks(value),
            "y" => Duration::days(value * 365),
            _ => {
                ctx.say("Invalid unit. Use `h`, `d`, `w`, or `y`.").await?;
                return Ok(());
            }
        };

        Some(dur)
    } else {
        None
    };

    let db_user = queries::get_user_by_discord_id_any(pool, user.id.get() as i64, guild_i64)
        .await?
        .ok_or("User not found")?;

    let expires_at = duration.map(|d| Utc::now() + d);

    queries::ban_user_from_events(
        pool,
        db_user.id,
        ctx.author().id.get() as i64,
        guild_i64,
        expires_at,
        reason.as_deref(),
    )
    .await?;

    info!(
        admin = %ctx.author().name,
        target = %user.id,
        duration = ?duration,
        "Applied global ban"
    );

    let _ = logger(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        LogType::Error,
        format!(
            "{} globally banned <@{}> for {}",
            ctx.author().name,
            user.id,
            reason.as_deref().unwrap_or("No reason provided.")
        ),
    )
    .await;

    let embed = CreateEmbed::new()
        .title("⚠️ Global Disqualification")
        .color(0xFFA500)
        .fields(vec![
            ("User", format!("<@{}>", user.id), true),
            ("Moderator", ctx.author().name.clone(), true),
            (
                "Duration",
                duration
                    .map(|d| format!("{} seconds", d.num_seconds()))
                    .unwrap_or("Permanent".to_string()),
                true,
            ),
            (
                "Reason",
                reason
                    .as_deref()
                    .unwrap_or("No reason provided.")
                    .to_string(),
                false,
            ),
        ])
        .timestamp(chrono::Utc::now());

    let _ = ChannelId::new(LOG_CHANNEL)
        .send_message(&ctx.http(), serenity::CreateMessage::new().embed(embed))
        .await;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    required_permissions = "BAN_MEMBERS",
    default_member_permissions = "BAN_MEMBERS"
)]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "User to unban"] user: serenity::User,
    #[description = "Reason for unban"] reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be in a server")?;
    let guild_i64 = guild_id.get() as i64;
    let pool = &ctx.data().db;

    let db_user = queries::get_user_by_discord_id_any(pool, user.id.get() as i64, guild_i64)
        .await?
        .ok_or("User not found")?;

    queries::unban_user_from_events(
        pool,
        db_user.id,
        ctx.author().id.get() as i64,
        guild_i64,
        reason.as_deref(),
    )
    .await?;

    let _ = logger(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        LogType::Info,
        format!("{} lifted global ban on <@{}>", ctx.author().name, user.id),
    )
    .await;

    let embed = CreateEmbed::new()
        .title("⚠️ User Unbanned")
        .color(0x00FF00)
        .fields(vec![
            ("User", format!("<@{}>", user.id), true),
            ("Moderator", ctx.author().name.clone(), true),
        ])
        .timestamp(chrono::Utc::now());

    let _ = ChannelId::new(LOG_CHANNEL)
        .send_message(
            &ctx.http(),
            serenity::CreateMessage::new().embed(embed.clone()),
        )
        .await;

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    rename = "list-punishments",
    required_permissions = "BAN_MEMBERS",
    default_member_permissions = "BAN_MEMBERS"
)]
pub async fn punishments(ctx: Context<'_>) -> Result<(), Error> {
    let pool = &ctx.data().db;
    let guild_id = ctx.guild_id().unwrap().get() as i64;

    let rows = queries::get_active_punishments(pool, guild_id).await?;

    if rows.is_empty() {
        ctx.say("No active punishments.").await?;
        return Ok(());
    }

    let mut desc = String::new();

    for (user_id, action, event_id, expiry, reason, _) in rows.iter().take(20) {
        let line = match action.as_str() {
            "ban" => {
                let expiry_text = expiry
                    .map(|t| format!("<t:{}:R>", t.timestamp()))
                    .unwrap_or_else(|| "Permanent".to_string());

                format!(
                    "🔴 <@{}> banned ({})\n> {}\n\n",
                    user_id, expiry_text, reason
                )
            }

            "disqualify" => {
                format!(
                    "🟠 <@{}> disqualified from event `{}`\n> {}\n\n",
                    user_id,
                    event_id.unwrap_or(0),
                    reason
                )
            }

            _ => continue, // should never happen
        };

        desc.push_str(&line);
    }

    // truncate if too long (Discord limit ~4096 chars)
    if desc.len() > 4000 {
        desc.truncate(4000);
        desc.push_str("\n...and more");
    }

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("Active Punishments")
                .description(desc),
        ),
    )
    .await?;

    Ok(())
}
