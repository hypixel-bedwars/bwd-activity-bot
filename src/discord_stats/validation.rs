use crate::shared::types::Data;
use time::OffsetDateTime;

// Note for future self: Right now your cooldown is per user globally, so if you wanna do this for
// multiple guilds you might want to change the key to (user_id, guild_id) or something like that.
pub fn validate_message(user_id: i64, content: &str, data: &Data) -> bool {
    let content = content.trim();

    let min_len = data.config.min_message_length as usize;
    let cooldown = data.config.message_cooldown_seconds as i64;

    // 1. Minimum length
    if content.len() < min_len {
        return false;
    }

    // 2. Ignore commands
    if content.starts_with('/') || content.starts_with('!') || content.starts_with("s?") {
        return false;
    }

    // 3. Ignore messages with no alphanumeric characters
    if !content.chars().any(|c| c.is_alphanumeric()) {
        return false;
    }

    // 4. Duplicate message check
    {
        let mut last_messages = data.message_validation.last_message.lock().unwrap();

        if let Some(last) = last_messages.get(&user_id) {
            if last == content {
                return false;
            }
        }

        last_messages.insert(user_id, content.to_string());
    }

    // 5. Cooldown check
    {
        let mut cooldowns = data.message_validation.last_counted.lock().unwrap();
        let now = OffsetDateTime::now_utc();

        if let Some(last_time) = cooldowns.get(&user_id) {
            if (now - *last_time).whole_seconds() < cooldown {
                return false;
            }
        }

        cooldowns.insert(user_id, now);
    }

    true
}
