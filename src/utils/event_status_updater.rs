use crate::database::queries;
use std::sync::Arc;
use tokio::time::{Duration, interval};

pub async fn start_event_status_updater(db_pool: Arc<sqlx::PgPool>) {
    let mut interval = interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        if let Err(e) = queries::update_event_statuses(&db_pool).await {
            tracing::error!("Failed to update event statuses: {}", e);
        }
    }
}
