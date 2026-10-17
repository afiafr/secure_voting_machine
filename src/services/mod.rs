pub mod election;
pub mod voters;
use crate::{auth::login::Session, models::Role};
use rusqlite::{params, Connection};
pub type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn require_role(conn: &Connection, session: &Session, role: Role) -> AppResult<()> {
    let valid: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?1 AND role=?2 AND active=1)",
        params![session.user_id(), role.as_str()],
        |r| r.get(0),
    )?;
    if !valid || session.role() != role {
        audit(
            conn,
            None,
            Some(session.user_id()),
            None,
            "authorization_denied",
            3,
        )?;
        return Err("This action is not permitted for your account.".into());
    }
    Ok(())
}
pub(crate) fn audit(
    conn: &Connection,
    election: Option<i64>,
    actor: Option<i64>,
    subject: Option<i64>,
    event: &str,
    severity: i64,
) -> AppResult<()> {
    conn.execute("INSERT INTO audit_events(election_id,actor_user_id,subject_user_id,event_type,severity) VALUES (?1,?2,?3,?4,?5)",
        params![election,actor,subject,event,severity])?;
    Ok(())
}
pub(crate) fn text(value: &str, label: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 200 || value.chars().any(char::is_control) {
        return Err(
            format!("{label} must contain 1-200 characters without control characters.").into(),
        );
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests;
