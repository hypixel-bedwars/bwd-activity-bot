/// Event-triggered full Hypixel sweep scheduler.
///
/// Runs a background loop that watches for upcoming event boundaries (start
/// and end) and launches a full Hypixel sweep timed to complete as close to
/// each boundary as possible.
///
/// # Timing
///
/// The sweeper processes one player per second, so:
///
/// ```text
/// sweep_duration_seconds = number_of_registered_players
/// pre_event_start_time   = event.start_date - sweep_duration
/// pre_event_end_time     = event.end_date   - sweep_duration
/// ```
///
/// When `now` enters the window `[pre_event_start_time, event.start_date)`,
/// a full sweep is spawned. The same logic applies for the end boundary.
///
/// # Duplicate prevention
///
/// `Data::is_full_sweep_running` (an `Arc<AtomicBool>`) is shared with the
/// regular stale sweep in `bot.rs`. Setting it blocks:
///   - A second event-triggered sweep from starting while one is in progress.
///   - The 2-hour stale sweep from running while a full sweep is active.
///
/// Per-event deduplication is tracked via in-memory `HashSet`s so the same
/// event never triggers more than one pre-start or one pre-end sweep, even
/// across multiple polling ticks inside the same window.
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use chrono::Utc;
use tokio::time::{Duration, interval};
use tracing::{info, warn};

use crate::database::queries;
use crate::shared::types::Data;
use crate::sweeper::hypixel_sweeper::run_full_hypixel_sweep;

/// Minimum sweep duration used even when the registered player count is very
/// small. Avoids a zero-second window that would never be entered.
const MIN_SWEEP_SECS: i64 = 60;

// Active user cutoff removed — use DB `active` column (migration 018).

/// How often the scheduler polls for upcoming event boundaries.
const POLL_INTERVAL_SECS: u64 = 30;

pub async fn start_event_sweep_scheduler(data: Arc<Data>) {
    // Per-event dedup: store the IDs of events for which we have already
    // successfully triggered a sweep so we do not re-trigger on the next tick.
    let mut scheduled_pre_start: HashSet<i64> = HashSet::new();
    let mut scheduled_pre_end: HashSet<i64> = HashSet::new();

    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));

    info!(
        "Event sweep scheduler started (poll interval {}s).",
        POLL_INTERVAL_SECS
    );

    loop {
        ticker.tick().await;

        // ----------------------------------------------------------------
        // 1. Estimate how long a full sweep will take.
        // ----------------------------------------------------------------
        let user_count: i64 = match queries::count_registered_users(&data.db).await {
            Ok(n) => n,
            Err(e) => {
                warn!(error = %e, "Event sweep scheduler: failed to count active users, skipping tick.");
                continue;
            }
        };

        // 1 second per user; floor at MIN_SWEEP_SECS to keep the window
        // meaningful when very few users are registered.
        let sweep_secs = user_count.max(MIN_SWEEP_SECS);
        let sweep_duration = chrono::Duration::seconds(sweep_secs);

        let now = Utc::now();

        // ----------------------------------------------------------------
        // 2. Pre-start sweep: check all pending events.
        // ----------------------------------------------------------------
        let pending_events = match queries::get_all_pending_events(&data.db).await {
            Ok(events) => events,
            Err(e) => {
                warn!(error = %e, "Event sweep scheduler: failed to query pending events.");
                vec![]
            }
        };

        for event in &pending_events {
            // Already triggered a sweep for this event — skip.
            if scheduled_pre_start.contains(&event.id) {
                continue;
            }

            // Enter the window when we are close enough that the sweep will
            // finish right around the event start time.
            let window_start = event.start_date - sweep_duration;

            if now >= window_start {
                // Atomically claim the global sweep flag.
                // swap returns the *old* value; false means we claimed it.
                if !data.is_full_sweep_running.swap(true, Ordering::SeqCst) {
                    scheduled_pre_start.insert(event.id);
                    let event_id = event.id;

                    info!(
                        event_id,
                        event_name = %event.name,
                        start_date = %event.start_date,
                        sweep_secs,
                        "Launching pre-event full sweep."
                    );

                    let data_clone = Arc::clone(&data);
                    tokio::spawn(async move {
                        run_full_hypixel_sweep(&data_clone).await;
                        data_clone
                            .is_full_sweep_running
                            .store(false, Ordering::SeqCst);
                        info!(event_id, "Pre-event full sweep completed.");
                    });
                } else {
                    // Another sweep is already running; we will retry on the
                    // next tick as long as the event ID is not yet in the set.
                    info!(
                        event_id = event.id,
                        "Pre-event sweep deferred — full sweep already in progress."
                    );
                }
            }
        }

        // ----------------------------------------------------------------
        // 3. Pre-end sweep: check all active events.
        // ----------------------------------------------------------------
        let active_events = match queries::get_all_active_events(&data.db).await {
            Ok(events) => events,
            Err(e) => {
                warn!(error = %e, "Event sweep scheduler: failed to query active events.");
                vec![]
            }
        };

        for event in &active_events {
            // Already triggered a pre-end sweep for this event — skip.
            if scheduled_pre_end.contains(&event.id) {
                continue;
            }

            let window_start = event.end_date - sweep_duration;

            if now >= window_start {
                if !data.is_full_sweep_running.swap(true, Ordering::SeqCst) {
                    scheduled_pre_end.insert(event.id);
                    let event_id = event.id;

                    info!(
                        event_id,
                        event_name = %event.name,
                        end_date = %event.end_date,
                        sweep_secs,
                        "Launching pre-event-end full sweep."
                    );

                    let data_clone = Arc::clone(&data);
                    tokio::spawn(async move {
                        run_full_hypixel_sweep(&data_clone).await;
                        data_clone
                            .is_full_sweep_running
                            .store(false, Ordering::SeqCst);
                        info!(event_id, "Pre-event-end full sweep completed.");
                    });
                } else {
                    info!(
                        event_id = event.id,
                        "Pre-event-end sweep deferred — full sweep already in progress."
                    );
                }
            }
        }
    }
}
