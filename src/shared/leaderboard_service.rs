use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::Mutex;
use tracing::debug;

use crate::shared::cache::TimedCache;
use crate::shared::types::Error;

type RankKey = (i64, i64);

struct LeaderboardService {
    pool: PgPool,
    cache: Arc<TimedCache<RankKey, i64>>,
    inflight: Mutex<HashMap<RankKey, Arc<Mutex<()>>>>,
}

static SERVICE: OnceLock<Arc<LeaderboardService>> = OnceLock::new();

pub fn init(pool: PgPool, ttl: Duration) {
    let service = Arc::new(LeaderboardService {
        pool,
        cache: Arc::new(TimedCache::new(ttl)),
        inflight: Mutex::new(HashMap::new()),
    });

    if SERVICE.set(Arc::clone(&service)).is_ok() {
        let purge_cache = Arc::clone(&service.cache);
        let purge_interval = ttl.max(Duration::from_secs(30));

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(purge_interval);
            loop {
                ticker.tick().await;
                purge_cache.purge_expired().await;
            }
        });
    }
}

fn get_service() -> Result<&'static Arc<LeaderboardService>, Error> {
    SERVICE
        .get()
        .ok_or_else(|| std::io::Error::other("leaderboard_service is not initialized").into())
}

pub async fn get_user_rank(event_id: i64, user_id: i64) -> Result<i64, Error> {
    let service = get_service()?;
    let key = (event_id, user_id);

    if let Some(rank) = service.cache.get(&key).await {
        return Ok(rank);
    }

    let key_lock = {
        let mut inflight = service.inflight.lock().await;
        Arc::clone(
            inflight
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    };

    let _guard = key_lock.lock().await;

    let result: Result<i64, Error> = if let Some(rank) = service.cache.get(&key).await {
        Ok(rank)
    } else {
        let rank = query_user_event_rank(&service.pool, event_id, user_id)
            .await?
            .unwrap_or(0);
        service.cache.insert(key, rank).await;
        Ok(rank)
    };

    {
        let mut inflight = service.inflight.lock().await;
        if let Some(existing) = inflight.get(&key) {
            if Arc::ptr_eq(existing, &key_lock) {
                inflight.remove(&key);
            }
        }
    }

    result
}

async fn query_user_event_rank(
    pool: &PgPool,
    event_id: i64,
    user_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    debug!(
        "leaderboard_service::query_user_event_rank: event_id={}, user_id={}",
        event_id, user_id
    );

    sqlx::query_scalar::<_, i64>(
        r#"
        WITH user_totals AS (
            SELECT ex.user_id, SUM(ex.xp_earned) AS total_event_xp
            FROM event_xp ex
            WHERE ex.event_id = $1
            GROUP BY ex.user_id
        ),
        eligible_users AS (
            SELECT ut.user_id, ut.total_event_xp
            FROM user_totals ut
            JOIN users u ON u.id = ut.user_id
            LEFT JOIN LATERAL (
                SELECT m.action_type, m.ban_expires_at
                FROM modrec m
                WHERE m.user_id = ut.user_id
                  AND m.action_type IN ('ban', 'unban')
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT 1
            ) latest_ban ON TRUE
            LEFT JOIN LATERAL (
                SELECT m.action_type
                FROM modrec m
                WHERE m.user_id = ut.user_id
                  AND m.event_id = $1
                  AND m.action_type IN ('disqualify', 'undisqualify')
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT 1
            ) latest_event_action ON TRUE
            WHERE u.active = TRUE
              AND NOT COALESCE(
                  latest_ban.action_type = 'ban'::modrec_action
                  AND (
                      latest_ban.ban_expires_at IS NULL
                      OR latest_ban.ban_expires_at > NOW()
                  ),
                  FALSE
              )
              AND COALESCE(
                  latest_event_action.action_type <> 'disqualify'::modrec_action,
                  TRUE
              )
        )
        SELECT rank FROM (
            SELECT
                eu.user_id,
                RANK() OVER (
                    ORDER BY eu.total_event_xp DESC, eu.user_id ASC
                ) AS rank
            FROM eligible_users eu
        ) ranked
        WHERE user_id = $2
        "#,
    )
    .bind(event_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
