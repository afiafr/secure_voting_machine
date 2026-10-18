//! Authentication, bounded retries, and a local virtual-SMS delivery adapter.
//! A Session is issued only after both credential and OTP verification.
use crate::{
    models::Role,
    services::{audit, AppResult},
};
use bcrypt::{hash, verify, DEFAULT_COST};
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub struct Session {
    user_id: i64,
    role: Role,
}
impl Session {
    pub fn user_id(&self) -> i64 {
        self.user_id
    }
    pub fn role(&self) -> Role {
        self.role
    }
    #[cfg(test)]
    pub(crate) fn for_test(user_id: i64, role: Role) -> Self {
        Self { user_id, role }
    }
}

pub struct PendingLogin {
    user_id: i64,
    role: Role,
    code_hash: String,
    expires: i64,
    attempts: u8,
}

pub struct Auth {
    pepper: String,
    mailbox: PathBuf,
    cost: u32,
}
impl Auth {
    pub fn from_env(mailbox: PathBuf) -> AppResult<Self> {
        dotenvy::dotenv().ok();
        let pepper = std::env::var("APP_PEPPER")
            .map_err(|_| "Set APP_PEPPER before starting the application.")?;
        if pepper.len() < 16 || pepper.len() > 40 {
            return Err("APP_PEPPER must be 16-40 bytes; preserve it between runs.".into());
        }
        Ok(Self {
            pepper,
            mailbox,
            cost: DEFAULT_COST,
        })
    }
    #[cfg(test)]
    pub(crate) fn for_test(mailbox: PathBuf) -> Self {
        Self {
            pepper: "isolated-test-pepper-123".into(),
            mailbox,
            cost: 4,
        }
    }
    fn password_input(&self, password: &str) -> AppResult<String> {
        let input = format!("{password}{}", self.pepper);
        if input.len() > 72 {
            return Err("Password and pepper together must be at most 72 bytes.".into());
        }
        Ok(input)
    }
    pub(crate) fn password_hash(&self, password: &str) -> AppResult<String> {
        if password.len() < 10 {
            return Err("Use a password of at least 10 characters.".into());
        }
        Ok(hash(self.password_input(password)?, self.cost)?)
    }
    pub(crate) fn lookup(&self, value: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.pepper.as_bytes())
            .expect("HMAC accepts any key length");
        mac.update(value.as_bytes());
        format!("{:x}", mac.finalize().into_bytes())
    }
    pub(crate) fn new_pin(&self) -> AppResult<(String, String, String)> {
        // UUIDv4 supplies OS randomness. The 20 decimal digits are never a user ID.
        let pin = format!(
            "{:020}",
            Uuid::new_v4().as_u128() % 100_000_000_000_000_000_000u128
        );
        let lookup = self.lookup(&pin);
        let hashed = hash(self.password_input(&pin)?, self.cost)?;
        Ok((pin, lookup, hashed))
    }
    pub fn password_login(
        &self,
        conn: &Connection,
        email: &str,
        password: &str,
        role: Role,
    ) -> AppResult<PendingLogin> {
        if role == Role::Voter {
            return Err("Voters sign in with their PIN.".into());
        }
        let scope = format!("password:{}", self.lookup(&email.trim().to_lowercase()));
        check_limit(conn, &scope)?;
        let record: Option<(i64, String)> = conn
            .query_row(
                "SELECT u.id,c.password_hash FROM users u JOIN user_credentials c ON c.user_id=u.id
             WHERE lower(u.email)=lower(?1) AND u.role=?2 AND u.active=1",
                params![email.trim(), role.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((id, stored)) = record {
            if verify(self.password_input(password)?, &stored)? {
                reset_limit(conn, &scope)?;
                return self.issue_otp(conn, id, role);
            }
        }
        failed(conn, &scope, 5, "password_failed")?;
        Err("Invalid credentials or role.".into())
    }
    pub fn pin_login(&self, conn: &Connection, pin: &str) -> AppResult<PendingLogin> {
        // This is a single-machine limiter, not an IP-based network control.
        check_limit(conn, "pin-terminal")?;
        let record: Option<(i64, String)> = conn
            .query_row(
                "SELECT u.id,p.pin_hash FROM voter_pin_credentials p JOIN users u ON u.id=p.user_id
             WHERE p.lookup_hash=?1 AND u.role='voter' AND u.active=1",
                [self.lookup(pin.trim())],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((id, stored)) = record {
            if verify(self.password_input(pin.trim())?, &stored)? {
                reset_limit(conn, "pin-terminal")?;
                return self.issue_otp(conn, id, Role::Voter);
            }
        }
        failed(conn, "pin-terminal", 10, "pin_failed")?;
        Err("Invalid PIN.".into())
    }
    fn issue_otp(&self, conn: &Connection, id: i64, role: Role) -> AppResult<PendingLogin> {
        check_limit(conn, &format!("otp:{id}"))?;
        let phone: String =
            conn.query_row("SELECT phone_number FROM users WHERE id=?1", [id], |r| {
                r.get(0)
            })?;
        let code = format!("{:06}", Uuid::new_v4().as_u128() % 1_000_000);
        self.deliver(
            &phone,
            &format!("Login OTP: {code}\nExpires in 5 minutes.\n"),
        )?;
        Ok(PendingLogin {
            user_id: id,
            role,
            code_hash: self.lookup(&code),
            expires: now() + 300,
            attempts: 0,
        })
    }
    pub fn complete_otp(
        &self,
        conn: &Connection,
        pending: &mut Option<PendingLogin>,
        code: &str,
    ) -> AppResult<Session> {
        let mut login = pending.take().ok_or("Start login again.")?;
        if now() >= login.expires {
            audit(conn, None, Some(login.user_id), None, "otp_expired", 3)?;
            return Err("OTP expired; log in again.".into());
        }
        let scope = format!("otp:{}", login.user_id);
        check_limit(conn, &scope)?;
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.pepper.as_bytes()).expect("valid HMAC key");
        mac.update(code.trim().as_bytes());
        // HMAC strings are not secrets; compare fixed-length digests without early exit.
        let actual = format!("{:x}", mac.finalize().into_bytes());
        let equal = actual
            .as_bytes()
            .iter()
            .zip(login.code_hash.as_bytes())
            .fold(0u8, |v, (a, b)| v | (a ^ b))
            == 0;
        if !equal {
            login.attempts += 1;
            failed(conn, &scope, 3, "otp_failed")?;
            if login.attempts < 3 {
                *pending = Some(login);
            }
            return Err("Invalid OTP; three failures require a cooldown and a new login.".into());
        }
        reset_limit(conn, &scope)?;
        audit(conn, None, Some(login.user_id), None, "login_succeeded", 0)?;
        Ok(Session {
            user_id: login.user_id,
            role: login.role,
        })
    }
    /// Retry queued alerts after UI actions. A crash may duplicate an alert,
    /// but it cannot erase the underlying committed audit event.
    pub fn flush_notifications(&self, conn: &Connection) -> AppResult<()> {
        let mut stmt = conn.prepare(
            "SELECT notification_id,phone,message FROM notification_outbox WHERE delivered=0 ORDER BY notification_id LIMIT 100"
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let messages = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        for (id, phone, message) in messages {
            self.deliver(&phone, &message)?;
            conn.execute(
                "UPDATE notification_outbox SET delivered=1 WHERE notification_id=?1",
                [id],
            )?;
        }
        Ok(())
    }

    /// Virtual SMS is a same-host teaching simulation, not independent MFA.
    /// Credentials are never included in the election audit log or admin screen.
    pub(crate) fn deliver(&self, phone: &str, message: &str) -> AppResult<()> {
        fs::create_dir_all(&self.mailbox)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.mailbox, fs::Permissions::from_mode(0o700))?;
        }
        let path = mailbox_path(&self.mailbox, phone);
        let mut options = fs::OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        writeln!(file, "{message}")?;
        file.sync_all()?;
        Ok(())
    }
}
pub fn mailbox_path(root: &Path, phone: &str) -> PathBuf {
    root.join(format!("{:x}.txt", Sha256::digest(phone.trim().as_bytes())))
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn check_limit(conn: &Connection, scope: &str) -> AppResult<()> {
    let until: Option<i64> = conn
        .query_row(
            "SELECT blocked_until FROM auth_attempts WHERE scope=?1",
            [scope],
            |r| r.get(0),
        )
        .optional()?;
    if until.is_some_and(|t| t > now()) {
        return Err("Too many failed attempts. Try again in 5 minutes.".into());
    }
    if until.is_some_and(|t| t > 0) {
        reset_limit(conn, scope)?;
    }
    Ok(())
}
fn reset_limit(conn: &Connection, scope: &str) -> AppResult<()> {
    conn.execute("DELETE FROM auth_attempts WHERE scope=?1", [scope])?;
    Ok(())
}
fn failed(conn: &Connection, scope: &str, threshold: i64, event: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO auth_attempts(scope,failures,blocked_until) VALUES (?1,1,0)
        ON CONFLICT(scope) DO UPDATE SET failures=failures+1",
        [scope],
    )?;
    conn.execute(
        "UPDATE auth_attempts SET blocked_until=?2 WHERE scope=?1 AND failures>=?3",
        params![scope, now() + 300, threshold],
    )?;
    audit(conn, None, None, None, event, 3)?;
    Ok(())
}
