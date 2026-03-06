/// `/send_registration_message` command — admin only.
///
/// Posts a persistent registration message containing instructions and a
/// "Register" button in the specified channel. When a member clicks the
/// button, the bot reads their server nickname, extracts their Minecraft
/// username, and runs the same registration flow as `/register`.
use poise::serenity_prelude::{self as serenity, CreateEmbed};
use tracing::info;

use crate::shared::types::{Context, Error};

/// Send a registration message with a Register button to a channel. Admin only.
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn send_registration_message(
    ctx: Context<'_>,
    #[description = "The channel to send the registration message to"]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    if !ctx
        .data()
        .config
        .admin_user_ids
        .contains(&ctx.author().id.get())
    {
        let embed = CreateEmbed::default()
            .title("Permission Denied")
            .color(0xFF4444)
            .description("You do not have permission to use this command.");
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let embed = CreateEmbed::default()
        .title("🔗 Account Registration")
        .color(0x00BFFF)
        .description(
            "Link your **Minecraft account** to start earning **XP** and tracking your stats on this server."
        )
        .field(
            "📛 Step 1 — Set Your Nickname",
            "Your server nickname must follow this format:\n\
            `[NNN emoji] YourMinecraftUsername`\n\n\
            **Examples:**\n\
            `[313 💫] VA80`\n\
            `[204 ✨] CosmicFuji`",
            false,
        )
        .field(
            "🔗 Step 2 — Link Your Discord on Hypixel",
            "Your **Hypixel profile** must have your Discord set as a social link.\n\n\
            **In-game command:**\n\
            `/socials discord <your_discord_tag>`",
            false,
        )
        .field(
            "✅ Final Step",
            "Once both steps are completed, press the **Register** button below.\n\
            The bot will automatically verify your account.",
            false,
        )
        .footer(serenity::CreateEmbedFooter::new(
            "Your Minecraft username must match the nickname you set.",
        ));

    let message = serenity::CreateMessage::new().embed(embed).components(vec![
        serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new("register_button")
                .label("Register")
                .emoji('✅')
                .style(serenity::ButtonStyle::Success),
        ]),
    ]);

    channel
        .id
        .send_message(&ctx.serenity_context().http, message)
        .await?;

    info!(
        "Sent registration message to channel '{}' ({})",
        channel.name, channel.id
    );

    let embed = CreateEmbed::default()
        .title("Registration Message Sent")
        .color(0x00BFFF)
        .description(format!("Registration message sent to <#{}>.", channel.id));
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
