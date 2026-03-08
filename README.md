# Memum Activity Bot

A Discord bot for Minecraft guilds, focused on tracking player stats, XP, and milestones, with rich leaderboard and registration features. Built with Rust and [Poise](https://github.com/serenity-rs/poise).

---

## Features

- **User Registration**: Link Discord and Minecraft accounts.
- **Stat Tracking**: Track Bedwars and Discord activity stats.
- **XP & Level System**: Earn XP for in-game and Discord activity.
- **Leaderboards**: Paginated, image-based leaderboards with persistent channel support.
- **Milestones**: Custom level milestones with live progress.
- **Admin Controls**: Fine-grained XP/stat configuration, role management, and more.

---

## Commands

### Registration

| Command                | Description                                                                                   | Permissions      |
|------------------------|-----------------------------------------------------------------------------------------------|------------------|
| `/register <username>` | Link your Discord account to your Minecraft account.                                          | User             |
| `/unregister`          | Unlink your Minecraft account and remove your data.                                           | User             |
| `/send_registration_message <channel>` | Post a persistent registration message with a "Register" button.                | Admin            |

### Stats & Levels

| Command      | Description                                                                                      | Permissions      |
|--------------|--------------------------------------------------------------------------------------------------|------------------|
| `/stats`     | Show your stat changes since registration and XP rewards for each stat.                          | User             |
| `/level`     | Show your XP, level, progress to next level, and a level card image.                             | User             |

### Leaderboard

| Command                | Description                                                                                   | Permissions      |
|------------------------|-----------------------------------------------------------------------------------------------|------------------|
| `/leaderboard`         | Show a paginated leaderboard image of top players in the guild.                              | User             |
| `/leaderboard_create <channel>` | Set up a persistent leaderboard in a channel (auto-updating).                        | Admin            |
| `/leaderboard_remove`  | Remove the persistent leaderboard and stop auto-updates.                                      | Admin            |

### Milestones

| Command                | Description                                                                                   | Permissions      |
|------------------------|-----------------------------------------------------------------------------------------------|------------------|
| `/milestone add <level>`   | Add a new milestone level. Appears on the leaderboard.                                   | Admin            |
| `/milestone edit <level>`  | Edit an existing milestone's level.                                                      | Admin            |
| `/milestone remove <level>`| Remove a milestone.                                                                      | Admin            |
| `/milestone view`          | Show your progress toward the next milestone.                                            | User             |

### Admin: Stat & XP Configuration

| Command                | Description                                                                                   | Permissions      |
|------------------------|-----------------------------------------------------------------------------------------------|------------------|
| `/edit-stats add-bedwars <mode> <metric> <xp>` | Add a Bedwars stat to XP config.                                   | Admin            |
| `/edit-stats add-discord <stat> <xp>`         | Add a Discord activity stat to XP config.                           | Admin            |
| `/edit-stats edit <stat> <xp>`                | Edit XP value for a configured stat.                                | Admin            |
| `/edit-stats remove <stat>`                   | Remove a stat from XP config.                                       | Admin            |
| `/edit-stats list`                            | List all stats in XP config.                                        | Admin            |

### Admin: Role Management

| Command                                    | Description                                                         | Permissions      |
|---------------------------------------------|---------------------------------------------------------------------|------------------|
| `/set-register-role <role>`                | Set the role assigned to users on registration.                     | Admin            |
| `/set-nickname-registration-role <role>`   | Allow members with this role to auto-register via nickname parsing. | Admin            |
| `/clear-nickname-registration-role`        | Require all users to use `/register`.                               | Admin            |

### Admin: XP Management

| Command                | Description                                                                                   | Permissions      |
|------------------------|-----------------------------------------------------------------------------------------------|------------------|
| `/xp add <@user> <amount>`    | Add XP to a user.                                                                | Admin            |
| `/xp remove <@user> <amount>` | Remove XP from a user.                                                           | Admin            |

### Admin: Registration Override

| Command                | Description                                                                                   | Permissions      |
|------------------------|-----------------------------------------------------------------------------------------------|------------------|
| `/force_register <@user> <minecraft_username>` | Forcibly register a user, bypassing Hypixel Discord verification.   | Admin            |

---

## API-like Documentation

### Command Structure

All commands are implemented as Discord slash commands using [Poise](https://github.com/serenity-rs/poise). Each command is annotated with `#[poise::command(...)]` and includes detailed Rust doc comments in the source.

#### Example Command Handler

```rust
/// Register your Minecraft account to start tracking stats and earning XP.
#[poise::command(slash_command, guild_only)]
pub async fn register(ctx: Context<'_>, minecraft_username: String) -> Result<(), Error> {
    // ...
}
```

### Stat Configuration

- **Bedwars Stats**: Configurable by mode and metric (e.g., `eight_two_final_kills_bedwars`).
- **Discord Stats**: Configurable for tracked activities (messages, reactions, etc.).
- **XP Values**: Set per-stat via `/edit-stats` commands.

### Milestones

- **Add/Edit/Remove**: Managed via `/milestone` subcommands.
- **Progress**: Displayed to users with `/milestone view` and on the leaderboard.

### Persistent Leaderboard

- **Setup**: `/leaderboard_create <channel>`
- **Remove**: `/leaderboard_remove`
- **Auto-Update**: Messages are updated automatically by the bot.

### Registration Flow

1. User runs `/register <minecraft_username>`.
2. Bot verifies ownership via Mojang and Hypixel APIs.
3. On success, user is assigned the configured registered role.
4. Admins can override with `/force_register`.

---

## Development

### Project Structure

- `src/commands/` — All command handlers, grouped by feature.
- `src/cards/` — Image generation for level and leaderboard cards.
- `src/database/` — Database models and queries.
- `src/discord_stats/` — Discord activity tracking.
- `src/hypixel/` — Hypixel API integration.
- `src/utils/` — Shared utilities.

### Running the Bot

1. Set up environment variables as required by `AppConfig`.
2. Run database migrations (`migrations/`).
3. Build and run with Cargo:

   ```
   cargo run --release
   ```

---

## Contributing

- See the source code for detailed documentation on each command and module.
- Contributions and issues are welcome!

---

## License

MIT

---