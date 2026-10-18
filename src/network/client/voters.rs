//! Voter endpoint calls used by the terminal client.
pub use super::provision;
use super::*;
pub fn register(
    conn: &mut ApiClient,
    auth: &ClientAuth,
    session: &Session,
    election: i64,
    input: &VoterInput,
) -> AppResult<i64> {
    let _ = auth;
    conn.call(
        session,
        "voters.register",
        serde_json::to_value((election, input))?,
    )
}
pub fn list(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<Voter>> {
    conn.call(session, "voters.list", serde_json::to_value((election,))?)
}
pub fn enroll(conn: &mut ApiClient, session: &Session, election: i64, nid: &str) -> AppResult<()> {
    conn.call(
        session,
        "voters.enroll",
        serde_json::to_value((election, nid))?,
    )
}
pub fn remove(conn: &mut ApiClient, session: &Session, election: i64, voter: i64) -> AppResult<()> {
    conn.call(
        session,
        "voters.remove",
        serde_json::to_value((election, voter))?,
    )
}
pub fn update(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    voter: i64,
    input: &VoterInput,
) -> AppResult<()> {
    conn.call(
        session,
        "voters.update",
        serde_json::to_value((election, voter, input))?,
    )
}
