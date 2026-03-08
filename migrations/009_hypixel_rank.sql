-- Migration 009: Add Hypixel rank columns to the users table.
--
-- hypixel_rank           — The player's Hypixel rank package as returned by the
--                          API (e.g. "VIP", "VIP_PLUS", "MVP", "MVP_PLUS",
--                          "SUPERSTAR" for MVP++, or NULL for no rank).
--                          Populated on the first Hypixel sweep after this
--                          migration runs; NULL means not yet fetched.
--
-- hypixel_rank_plus_color — The colour of the player's rank "+" symbol, as
--                           returned by the API's `rankPlusColor` field
--                           (e.g. "RED", "GOLD", "DARK_GREEN", etc.).
--                           Only meaningful for MVP+ and MVP++; NULL otherwise.

ALTER TABLE users
    ADD COLUMN hypixel_rank            TEXT,
    ADD COLUMN hypixel_rank_plus_color TEXT;
