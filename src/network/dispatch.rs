//! Explicit endpoint allowlist; no endpoint accepts SQL or a caller-selected actor ID.
use super::*;
pub(super) fn service_call(
    state: &mut State,
    session: &auth::login::Session,
    operation: &str,
    args: Value,
) -> AppResult<Value> {
    match operation {
        "election.list" => {
            let (): () = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::list(
                &state.conn,
                session,
            )?)?)
        }
        "election.get" => {
            let (id,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::get(
                &state.conn,
                session,
                id,
            )?)?)
        }
        "election.positions" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::positions(
                &state.conn,
                session,
                election,
            )?)?)
        }
        "election.create" => {
            let (name,): (String,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::create(
                &mut state.conn,
                session,
                &name,
            )?)?)
        }
        "election.rename" => {
            let (election, name): (i64, String) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::rename(
                &mut state.conn,
                session,
                election,
                &name,
            )?)?)
        }
        "election.delete" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::delete(
                &mut state.conn,
                session,
                election,
            )?)?)
        }
        "election.add_position" => {
            let (election, name): (i64, String) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::add_position(
                &mut state.conn,
                session,
                election,
                &name,
            )?)?)
        }
        "election.rename_position" => {
            let (election, position, name): (i64, i64, String) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::rename_position(
                &mut state.conn,
                session,
                election,
                position,
                &name,
            )?)?)
        }
        "election.delete_position" => {
            let (election, position): (i64, i64) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::delete_position(
                &mut state.conn,
                session,
                election,
                position,
            )?)?)
        }
        "election.add_candidate" => {
            let (election, position, name, party): (i64, i64, String, String) =
                serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::add_candidate(
                &mut state.conn,
                session,
                election,
                position,
                &name,
                &party,
            )?)?)
        }
        "election.link_candidate" => {
            let (election, position, candidate): (i64, i64, i64) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::link_candidate(
                &mut state.conn,
                session,
                election,
                position,
                candidate,
            )?)?)
        }
        "election.update_candidate" => {
            let (election, candidate, name, party): (i64, i64, String, String) =
                serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::update_candidate(
                &mut state.conn,
                session,
                election,
                candidate,
                &name,
                &party,
            )?)?)
        }
        "election.unlink_candidate" => {
            let (election, position, candidate): (i64, i64, i64) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::unlink_candidate(
                &mut state.conn,
                session,
                election,
                position,
                candidate,
            )?)?)
        }
        "election.start" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::start(
                &mut state.conn,
                session,
                election,
            )?)?)
        }
        "election.close" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::close(
                &mut state.conn,
                session,
                election,
            )?)?)
        }
        "election.cast" => {
            let (election, choices): (i64, Vec<(i64, i64)>) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::cast(
                &mut state.conn,
                session,
                election,
                &choices,
            )?)?)
        }
        "election.tally" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::tally(
                &mut state.conn,
                session,
                election,
            )?)?)
        }
        "election.announce" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::announce(
                &mut state.conn,
                session,
                election,
            )?)?)
        }
        "election.results" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::results(
                &state.conn,
                session,
                election,
            )?)?)
        }
        "election.party_results" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::party_results(
                &state.conn,
                session,
                election,
            )?)?)
        }
        "election.audit_log" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::audit_log(
                &state.conn,
                session,
                election,
            )?)?)
        }
        "election.flag_incident" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::election::flag_incident(
                &state.conn,
                session,
                election,
            )?)?)
        }
        "voters.register" => {
            let (election, input): (i64, VoterInput) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::voters::register(
                &mut state.conn,
                &state.auth,
                session,
                election,
                &input,
            )?)?)
        }
        "voters.list" => {
            let (election,): (i64,) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::voters::list(
                &state.conn,
                session,
                election,
            )?)?)
        }
        "voters.enroll" => {
            let (election, nid): (i64, String) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::voters::enroll(
                &mut state.conn,
                session,
                election,
                &nid,
            )?)?)
        }
        "voters.remove" => {
            let (election, voter): (i64, i64) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::voters::remove(
                &mut state.conn,
                session,
                election,
                voter,
            )?)?)
        }
        "voters.update" => {
            let (election, voter, input): (i64, i64, VoterInput) = serde_json::from_value(args)?;
            Ok(serde_json::to_value(services::voters::update(
                &mut state.conn,
                session,
                election,
                voter,
                &input,
            )?)?)
        }
        _ => Err("Unknown endpoint.".into()),
    }
}
