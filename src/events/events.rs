use poise::serenity_prelude as serenity;
use tracing::warn;

use crate::commands::leaderboard::leaderboard as lb;
use crate::commands::registration::register::perform_registration;
use crate::shared::types::{Data, Error};

use crate::database::queries;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &Data,
) -> Result<(), Error> {
    if let serenity::FullEvent::InteractionCreate { interaction } = event {
        if let serenity::Interaction::Component(component) = interaction {
            if component.data.custom_id == "register_button" {
                handle_register_button(ctx, component, data).await?;
            } else if component.data.custom_id.starts_with("lb_page_") {
                if let Err(e) = lb::handle_pagination(ctx, component, data).await {
                    tracing::error!(error = %e, "Leaderboard pagination handler failed");
                }
            }
        }
    }

    Ok(())
}

async fn handle_register_button(
    ctx: &serenity::Context,
    component: &serenity::ComponentInteraction,
    data: &Data,
) -> Result<(), Error> {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => {
            respond_ephemeral(
                ctx,
                component,
                "This button can only be used inside a server.",
            )
            .await?;
            return Ok(());
        }
    };

    // Get user from database
    let db_user = match queries::get_user_by_discord_id(
        &data.db,
        component.user.id.get() as i64,
        guild_id.get() as i64,
    )
    .await
    {
        Ok(user) => user,
        Err(e) => {
            warn!("Failed to query DB for user: {}", e);
            respond_ephemeral(ctx, component, "Database error.").await?;
            return Ok(());
        }
    };

    // User must already have a Minecraft username stored
    let minecraft_username = match db_user {
        Some(user) => match user.minecraft_username {
            Some(username) => username,
            None => {
                respond_ephemeral(
                    ctx,
                    component,
                    "No Minecraft username found for your account.\n\n\
                    Please run the `/register <minecraft_username>` command first.",
                )
                .await?;
                return Ok(());
            }
        },
        None => {
            respond_ephemeral(
                ctx,
                component,
                "You are not registered yet.\n\n\
                Please run `/register <minecraft_username>` first.",
            )
            .await?;
            return Ok(());
        }
    };

    // Acknowledge interaction immediately
    component
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Defer(
                serenity::CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await?;

    let msg = perform_registration(
        ctx,
        data,
        guild_id,
        component.user.id,
        &component.user.tag(),
        &minecraft_username,
    )
    .await;

    let reply_text = match msg {
        Ok((text, Some((db_user_id, uuid)))) => {
            let _ = crate::commands::registration::register::fetch_and_cache_head_texture(
                &data.db,
                db_user_id,
                &uuid,
            )
            .await;

            text
        }
        Ok((text, None)) => text,
        Err(e) => {
            warn!(
                user = component.user.id.get(),
                error = %e,
                "perform_registration returned an unexpected error"
            );
            format!("An unexpected error occurred during registration: {e}")
        }
    };

    component
        .create_followup(
            ctx,
            serenity::CreateInteractionResponseFollowup::new()
                .content(reply_text)
                .ephemeral(true),
        )
        .await?;

    Ok(())
}

async fn respond_ephemeral(
    ctx: &serenity::Context,
    component: &serenity::ComponentInteraction,
    content: &str,
) -> Result<(), Error> {
    component
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}
