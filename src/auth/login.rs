use rusqlite::{Connection, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use dotenvy::dotenv;
use std::env;

pub(crate) fn initialize_database() -> Result<Connection> {
    let conn = Connection::open("users.db")?;

    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            email TEXT UNIQUE,
            phone_number TEXT NOT NULL
                CHECK(length(trim(phone_number)) > 0),
            role TEXT NOT NULL
                CHECK(role IN ('admin', 'district_official', 'voter')),
            full_name TEXT NOT NULL
                CHECK(length(trim(full_name)) > 0),

            CHECK (
                role = 'voter'
                OR (email IS NOT NULL AND length(trim(email)) > 0)
            )
        );

        CREATE TABLE IF NOT EXISTS user_credentials (
            user_id INTEGER PRIMARY KEY,
            password_hash TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
        ",
    )?;

    Ok(conn)
}

fn mix_password_with_pepper(
    password: &str,
) -> std::result::Result<String, env::VarError> {
    dotenv().ok();
    let pepper = env::var("APP_PEPPER")?;
    Ok(format!("{password}{pepper}"))
}

pub(crate) fn handle_login(
    conn: &Connection,
    email: &str,
    raw_password: &str,
) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    let is_valid = verify_user_login(conn, email, raw_password)?;

    if !is_valid {
        println!("Invalid email or password.");
        return Ok(false); // Exit this function before reaching protected code
    }

    println!("Login successful!");
    // These run only after successful login.

    Ok(true)
}


pub(crate) fn register_user(
    conn: &Connection,
    email: &str,
    phone_number: &str,
    role: &str,
    full_name: &str,
    raw_password: &str,
) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    conn.execute(
        "INSERT INTO users (email, phone_number, role, full_name) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![email, phone_number, role, full_name],
    )?;
    // The user must already exist in the users table.
    let user_id: i64 = conn.query_row(
        "SELECT id FROM users WHERE email = ?1",
        rusqlite::params![email],
        |row| row.get(0),
    )?;

    let combined_input = mix_password_with_pepper(raw_password)?;
    let hashed_password = hash(&combined_input, DEFAULT_COST)?;

    conn.execute(
        "INSERT INTO user_credentials (user_id, password_hash)
         VALUES (?1, ?2)",
        rusqlite::params![user_id, hashed_password],
    )?;

    Ok(true)
}


pub(crate) fn verify_user_login(
    conn: &Connection,
    email: &str,
    raw_password: &str,
) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT c.password_hash
         FROM user_credentials AS c
         JOIN users AS u ON u.id = c.user_id
         WHERE u.email = ?1",
    )?;

    let mut rows = stmt.query(rusqlite::params![email])?;

    if let Some(row) = rows.next()? {
        let stored_hash: String = row.get(0)?;
        let combined_input = mix_password_with_pepper(raw_password)?;

        // Propagate verification errors instead of panicking.
        Ok(verify(&combined_input, &stored_hash)?)
    } else {
        Ok(false)
    }
}

#[cfg(test)] 
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};
    static CONN: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
        let conn = initialize_database().expect("Failed to initialize database");
        Mutex::new(conn)
    });

    #[test]
    fn test_registration() {
        match register_user(&CONN.lock().unwrap(), "rustacean@example.com", "56565556656", "voter", "Rustacean", "MySecret123") {
            Ok(_) => println!("Account created successfully!"),
            Err(e) => println!("Skipped registration: {}", e),
        }
    }

    // Test Login
    #[test]
    fn test_login() {
        match verify_user_login(&CONN.lock().unwrap(), "rustacean@example.com", "MySecret123") {
            Ok(true) => println!("Access Granted! 🎉"),
            Ok(false) => println!("Access Denied! ❌"),
            Err(e) => println!("Database error during login: {}", e),
        }
    }
}
