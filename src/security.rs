//! Request deadlines and sanitized security events. Never accept log text as evidence.
use crate::services::AppResult;
use rusqlite::{params, Connection};
use std::{
    cell::Cell,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
thread_local! { static DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) }; }
/// Return Unix milliseconds for the cross-hop request deadline.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
/// Clamp the request budget to ten seconds and store a monotonic worker deadline.
pub fn set_deadline(ms: u64) {
    DEADLINE.with(|d| {
        d.set(Some(
            Instant::now() + Duration::from_millis(ms.saturating_sub(now_ms()).min(10_000)),
        ))
    });
}
/// Clear the worker deadline before recording post-request security events.
pub fn clear_deadline() {
    DEADLINE.with(|d| d.set(None));
}
/// Return whether the current worker has exhausted its request budget.
pub fn expired() -> bool {
    DEADLINE.with(|d| d.get().is_some_and(|at| Instant::now() >= at))
}
/// Reject expired work before credential issuance or transaction commit.
pub fn check_deadline() -> AppResult<()> {
    if expired() {
        Err("Request timed out; refresh state before retrying.".into())
    } else {
        Ok(())
    }
}
/// Structured fields only: passwords, OTPs, request bodies and ballots never enter logs.
pub fn event(
    conn: &Connection,
    user: Option<i64>,
    kind: &str,
    scope: Option<&str>,
    attempts: i64,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO security_events(user_id,event_type,scope,attempts) VALUES (?1,?2,?3,?4)",
        params![user, kind, scope, attempts],
    )?;
    crate::services::audit(conn, None, user, None, kind, 3)
}
