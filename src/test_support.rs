//! Isolated fixtures shared by UI and database tests.
use crate::{
    auth::{
        self,
        login::{Auth, Session},
    },
    database,
    models::{Role, VoterInput},
    services::{election, voters},
};
use rusqlite::Connection;
use std::{fs, path::PathBuf};
use uuid::Uuid;
pub(crate) struct Fixture {
    pub(crate) conn: Connection,
    pub(crate) auth: Auth,
    pub(crate) root: PathBuf,
    pub(crate) admin: Session,
    pub(crate) official: Session,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let root = std::env::temp_dir().join(format!("voting-test-{}", Uuid::new_v4()));
        let auth = Auth::for_test(root.clone());
        let mut conn = Connection::open_in_memory().unwrap();
        database::initialize(&mut conn).unwrap();
        voters::provision(
            &mut conn,
            &auth,
            "admin@example.test",
            "+15550000001",
            "Admin",
            "password1234",
            Role::Admin,
        )
        .unwrap();
        voters::provision(
            &mut conn,
            &auth,
            "official@example.test",
            "+15550000002",
            "Official",
            "password1234",
            Role::DistrictOfficial,
        )
        .unwrap();
        Self {
            conn,
            auth,
            root,
            admin: Session::for_test(1, Role::Admin),
            official: Session::for_test(2, Role::DistrictOfficial),
        }
    }
    pub(crate) fn create(&mut self) -> i64 {
        election::create(&mut self.conn, &self.admin, "School Election").unwrap()
    }
    pub(crate) fn register(&mut self, e: i64, n: usize) -> (i64, String) {
        let v = input(n);
        let id = voters::register(&mut self.conn, &self.auth, &self.admin, e, &v).unwrap();
        let pin = message(&self.root, &v.phone, "Voting PIN: ");
        (id, pin)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
pub(crate) fn input(n: usize) -> VoterInput {
    VoterInput {
        full_name: format!("Voter {n}"),
        email: format!("v{n}@example.test"),
        phone: format!("+15550001{n:03}"),
        dob: "2000-01-02".into(),
        nid: format!("NID{n}"),
    }
}
pub(crate) fn message(root: &std::path::Path, phone: &str, prefix: &str) -> String {
    fs::read_to_string(auth::login::mailbox_path(root, phone))
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix(prefix))
        .next_back()
        .unwrap()
        .to_string()
}
pub(crate) fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
pub(crate) fn choices(conn: &Connection, s: &Session, e: i64, index: usize) -> Vec<(i64, i64)> {
    election::positions(conn, s, e)
        .unwrap()
        .iter()
        .map(|p| (p.id, p.candidates[index].id))
        .collect()
}
