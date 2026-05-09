//! Background tasks. A tiny runner that spawns one tokio task per
//! recurring job. Kept simple on purpose — we don't ship a cron DSL,
//! just "run this every N seconds".

use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};

use crate::auth::purge_expired_sessions;
use crate::auth::recovery::purge_expired_reset_tokens;
use crate::orm::Db;

/// Spin up the standard housekeeping tasks. Call this once during
/// app startup. Returns immediately — the tasks run forever.
pub fn spawn_housekeeping(db: Db) {
    spawn_session_sweeper(db.clone());
}

/// Periodic maintenance loop — every 10 minutes, sweep:
///
/// 1. Expired sessions (`auth::purge_expired_sessions`) — removes
///    rows from `rustio_sessions` where `expires_at < NOW()`.
/// 2. Reset-token rows past their forensic-retention window
///    (`auth::recovery::purge_expired_reset_tokens` — R1 commit
///    #12) — removes rows from `rustio_password_reset_tokens`
///    where `expires_at < NOW() - INTERVAL '7 days'`.
///
/// **Failure isolation:** the two sweeps run sequentially within
/// each tick but each handles its own `Result` — a failure in one
/// does NOT prevent the other from running, and neither prevents
/// the next tick. The next tick happens unconditionally on the
/// timer.
///
/// The 10-minute interval is kept from R0; reset-token sweeping
/// piggy-backs on the existing tick rather than spawning a second
/// timer loop.
///
/// (The function name is preserved for backwards compatibility —
/// downstream projects already call `spawn_session_sweeper`
/// directly. The expanded scope is documented above.)
pub fn spawn_session_sweeper(db: Db) {
    log::info!("background housekeeping sweeper spawned (10 min interval)");
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(600));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // Skip the first tick (it fires immediately) — no need to
        // sweep the second the server boots.
        tick.tick().await;
        loop {
            tick.tick().await;

            // Session sweep — independent of the recovery-token
            // sweep below. A failure here does NOT prevent the
            // recovery sweep from running on this same tick.
            match purge_expired_sessions(&db).await {
                Ok(0) => {}
                Ok(n) => log::info!("swept {n} expired sessions"),
                Err(e) => log::warn!("session sweep failed: {e}"),
            }

            // Recovery-token sweep — independent of the session
            // sweep above. A failure here does NOT prevent the
            // session sweep from running, AND does not prevent
            // the next tick. The subsystem-tagged target lets
            // operators filter by stream.
            match purge_expired_reset_tokens(&db).await {
                Ok(0) => {}
                Ok(n) => log::info!(
                    target: "rustio_admin::recovery_sweeper",
                    "swept {n} expired reset tokens"
                ),
                Err(e) => log::warn!(
                    target: "rustio_admin::recovery_sweeper",
                    "recovery-token sweep failed: {e}"
                ),
            }
        }
    });
}
