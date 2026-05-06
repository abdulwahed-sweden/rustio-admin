//! Background tasks. A tiny runner that spawns one tokio task per
//! recurring job. Kept simple on purpose — we don't ship a cron DSL,
//! just "run this every N seconds".

use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};

use crate::auth::purge_expired_sessions;
use crate::orm::Db;

/// Spin up the standard housekeeping tasks. Call this once during
/// app startup. Returns immediately — the tasks run forever.
pub fn spawn_housekeeping(db: Db) {
    spawn_session_sweeper(db.clone());
}

/// Delete expired sessions every 10 minutes.
pub fn spawn_session_sweeper(db: Db) {
    log::info!("background session sweeper spawned (10 min interval)");
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(600));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // Skip the first tick (it fires immediately) — no need to
        // sweep the second the server boots.
        tick.tick().await;
        loop {
            tick.tick().await;
            match purge_expired_sessions(&db).await {
                Ok(0) => {}
                Ok(n) => log::info!("swept {n} expired sessions"),
                Err(e) => log::warn!("session sweep failed: {e}"),
            }
        }
    });
}
