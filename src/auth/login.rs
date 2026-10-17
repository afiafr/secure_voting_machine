use rusqlite::{Connection, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use dotenvy::dotenv;
use std::env;
use sha3::{Sha3_256, Digest};

// A simple random oracle function that hashes the input using SHA3-256.
fn random_oracle(input: &[u8]) -> Vec<u8> {
    let result = Sha3_256::digest(input);
    result.to_vec()
}

// Initializes the SQLite database and creates the necessary tables if they don't exist.
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

        CREATE TABLE IF NOT EXISTS voter_pins (
            user_id INTEGER PRIMARY KEY,
            voter_pin TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
        ",
    )?;

    Ok(conn)
}

// Mixes the provided password with a pepper value from the environment variables.
fn mix_password_with_pepper(
    password: &str,
) -> std::result::Result<String, env::VarError> {
    dotenv().ok();
    let pepper = env::var("APP_PEPPER")?;
    Ok(format!("{password}{pepper}"))
}

// Handles the PIN verification process, returning true if the PIN is valid.
pub(crate) fn handle_pin(
    conn: &Connection,
    pin: &str,
) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    let is_valid = verify_pin(conn, pin)?;

    if !is_valid {
        println!("Invalid PIN.");
        return Ok(false);
    }

    println!("Login successful!");

    Ok(true)
}

// Handles the user login process, returning true if the login is successful.
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

// Registers a new user in the database, hashing their password and storing it securely.
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


// Registers a new voter PIN in the database, associating it with the user's ID.
pub(crate) fn register_voter_pin(
    conn: &Connection,
    email: &str,
    full_name: &str,
    phone_number: &str,
) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    conn.execute(
        "INSERT INTO users (email, phone_number, role, full_name) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![email, phone_number, "Voter", full_name],
    )?;

    let user_id: i64 = conn.query_row(
        "SELECT id FROM users WHERE email = ?1",
        rusqlite::params![email],
        |row| row.get(0),
    )?;

    let voter_pin = random_oracle(user_id.to_string().as_bytes());

    conn.execute(
        "INSERT INTO voter_pins (user_id, voter_pin)
         VALUES (?1, ?2)",
        rusqlite::params![user_id, voter_pin],
    )?;

    Ok(true)
}

// Verifies the provided PIN against the stored hash in the database.
pub(crate) fn verify_pin(
    conn: &Connection,
    pin: &str,
) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT c.voter_pin
         FROM voter_pins AS c
         JOIN users AS u ON u.id = c.user_id
         WHERE u.voter_pin = ?1",
    )?;

    let mut rows = stmt.query(rusqlite::params![pin])?;

    if let Some(row) = rows.next()? {
        let stored_hash: String = row.get(0)?;
        let combined_input = mix_password_with_pepper(pin)?;

        Ok(verify(&combined_input, &stored_hash)?)
    } else {
        Ok(false)
    }
}

// Verifies the user's login credentials by checking the provided email and password against the stored hash in the database.
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


// Tests for the user login and registration process.
#[cfg(test)] 
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};
    static CONN: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
        let conn = initialize_database().expect("Failed to initialize database");
        Mutex::new(conn)
    });

    // Test Registration
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

    // Test PIN Verification
    #[test]
    fn test_pin_verification() {
        match verify_pin(&CONN.lock().unwrap(), "1234") {
            Ok(true) => println!("PIN Verified! 🎉"),
            Ok(false) => println!("Invalid PIN. ❌"),
            Err(e) => println!("Database error during PIN verification: {}", e),
        }
    }
}
