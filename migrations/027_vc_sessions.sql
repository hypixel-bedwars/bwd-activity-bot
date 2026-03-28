CREATE TABLE vc_sessions (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  guild_id BIGINT NOT NULL REFERENCES guilds (guild_id) ON DELETE CASCADE,
  join_time TIMESTAMPTZ NOT NULL,
  leave_time TIMESTAMPTZ,

  CHECK (leave_time IS NULL OR leave_time > join_time)
);

CREATE INDEX idx_vc_user_time ON vc_sessions (user_id, join_time);
CREATE INDEX idx_vc_active ON vc_sessions (user_id, guild_id) WHERE leave_time IS NULL;

-- Only 1 active VC session per user
CREATE UNIQUE INDEX one_active_session_per_user
ON vc_sessions(user_id)
WHERE leave_time IS NULL;