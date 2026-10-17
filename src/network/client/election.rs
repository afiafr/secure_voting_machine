//! Election endpoint calls used by the terminal client.
use super::*;
pub fn list(conn: &ApiClient, session: &Session) -> AppResult<Vec<Election>> {
    conn.call(session, "election.list", serde_json::to_value(())?)
}
pub fn get(conn: &ApiClient, session: &Session, id: i64) -> AppResult<Election> {
    conn.call(session, "election.get", serde_json::to_value((id,))?)
}
pub fn positions(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<Position>> {
    conn.call(
        session,
        "election.positions",
        serde_json::to_value((election,))?,
    )
}
pub fn create(conn: &mut ApiClient, session: &Session, name: &str) -> AppResult<i64> {
    conn.call(session, "election.create", serde_json::to_value((name,))?)
}
pub fn rename(conn: &mut ApiClient, session: &Session, election: i64, name: &str) -> AppResult<()> {
    conn.call(
        session,
        "election.rename",
        serde_json::to_value((election, name))?,
    )
}
pub fn delete(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()> {
    conn.call(
        session,
        "election.delete",
        serde_json::to_value((election,))?,
    )
}
pub fn add_position(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    name: &str,
) -> AppResult<()> {
    conn.call(
        session,
        "election.add_position",
        serde_json::to_value((election, name))?,
    )
}
pub fn rename_position(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    position: i64,
    name: &str,
) -> AppResult<()> {
    conn.call(
        session,
        "election.rename_position",
        serde_json::to_value((election, position, name))?,
    )
}
pub fn delete_position(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    position: i64,
) -> AppResult<()> {
    conn.call(
        session,
        "election.delete_position",
        serde_json::to_value((election, position))?,
    )
}
pub fn add_candidate(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    position: i64,
    name: &str,
    party: &str,
) -> AppResult<()> {
    conn.call(
        session,
        "election.add_candidate",
        serde_json::to_value((election, position, name, party))?,
    )
}
pub fn link_candidate(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    position: i64,
    candidate: i64,
) -> AppResult<()> {
    conn.call(
        session,
        "election.link_candidate",
        serde_json::to_value((election, position, candidate))?,
    )
}
pub fn update_candidate(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    candidate: i64,
    name: &str,
    party: &str,
) -> AppResult<()> {
    conn.call(
        session,
        "election.update_candidate",
        serde_json::to_value((election, candidate, name, party))?,
    )
}
pub fn unlink_candidate(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    position: i64,
    candidate: i64,
) -> AppResult<()> {
    conn.call(
        session,
        "election.unlink_candidate",
        serde_json::to_value((election, position, candidate))?,
    )
}
pub fn start(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()> {
    conn.call(
        session,
        "election.start",
        serde_json::to_value((election,))?,
    )
}
pub fn close(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()> {
    conn.call(
        session,
        "election.close",
        serde_json::to_value((election,))?,
    )
}
pub fn cast(
    conn: &mut ApiClient,
    session: &Session,
    election: i64,
    choices: &[(i64, i64)],
) -> AppResult<()> {
    conn.call(
        session,
        "election.cast",
        serde_json::to_value((election, choices))?,
    )
}
pub fn tally(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()> {
    conn.call(
        session,
        "election.tally",
        serde_json::to_value((election,))?,
    )
}
pub fn announce(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()> {
    conn.call(
        session,
        "election.announce",
        serde_json::to_value((election,))?,
    )
}
pub fn results(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<ResultRow>> {
    conn.call(
        session,
        "election.results",
        serde_json::to_value((election,))?,
    )
}
pub fn party_results(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<String>> {
    conn.call(
        session,
        "election.party_results",
        serde_json::to_value((election,))?,
    )
}
pub fn audit_log(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<String>> {
    conn.call(
        session,
        "election.audit_log",
        serde_json::to_value((election,))?,
    )
}
pub fn flag_incident(conn: &ApiClient, session: &Session, election: i64) -> AppResult<()> {
    conn.call(
        session,
        "election.flag_incident",
        serde_json::to_value((election,))?,
    )
}
