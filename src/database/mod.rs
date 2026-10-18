//! Schema setup and a non-destructive migration from the uploaded prototype.
use rusqlite::{Connection, Result};
use std::path::Path;

pub fn open(path: impl AsRef<Path>) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    initialize(&mut conn)?;
    Ok(conn)
}

pub fn initialize(conn: &mut Connection) -> Result<()> {
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    let tx = conn.transaction()?;
    tx.execute_batch(include_str!("schema.sql"))?;
    // The uploaded users table has no DOB/NID. Preserve existing accounts.
    let columns: Vec<String> = {
        let mut q = tx.prepare("PRAGMA table_info(users)")?;
        let rows = q.query_map([], |r| r.get(1))?;
        rows.collect::<Result<_>>()?
    };
    for (name, declaration) in [
        ("dob", "TEXT"),
        ("nid", "TEXT"),
        (
            "active",
            "INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1))",
        ),
    ] {
        if !columns.iter().any(|c| c == name) {
            tx.execute_batch(&format!(
                "ALTER TABLE users ADD COLUMN {name} {declaration};"
            ))?;
        }
    }
    tx.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS users_nid_unique ON users(nid) WHERE nid IS NOT NULL; PRAGMA user_version=2;")?;
    // The old voter_pins table is retained but never used: its SHA3(user_id)
    // credentials were predictable and cannot be converted into secure PINs.
    tx.commit()
}
