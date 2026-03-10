/// Discord activity stat tracking.
///
/// - `tracker` — event listener that records messages, reactions, and command usage.
/// - `validation` — logic to determine if a message should count for XP (e.g. not a command, not a duplicate, etc.).
pub mod tracker;
pub mod validation;
