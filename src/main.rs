//! Startup and command-line modes for the voting application.
mod auth;
mod database;
mod models;
mod network;
mod security;
mod services;
#[cfg(test)]
mod test_support;
mod ui;
mod validation;
use network::{ApiClient, Network};
use services::AppResult;
use std::path::PathBuf;
use ui::{run, App};

/// Parse command modes, load configuration and start the TUI or TLS services.
fn main() -> AppResult<()> {
    dotenvy::dotenv().ok();
    let _trace_guard = init_security_tracing()?;
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|s| s == "--version") {
        println!(
            "secure_voting_machine {} (security course edition)",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    let mailbox =
        PathBuf::from(std::env::var("VOTING_SMS_DIR").unwrap_or_else(|_| "virtual_sms".into()));
    if let Some(i) = args.iter().position(|s| s == "--inbox") {
        let phone = args.get(i + 1).ok_or("Usage: --inbox PHONE")?;
        println!(
            "LOCAL SIMULATION: anyone with OS access to this inbox can read its messages.\n{}",
            std::fs::read_to_string(auth::login::mailbox_path(&mailbox, phone)?)?
        );
        return Ok(());
    }
    let tls_path =
        PathBuf::from(std::env::var("VOTING_TLS_DIR").unwrap_or_else(|_| "tls-dev".into()));
    if args.iter().any(|s| s == "--init-tls") {
        network::tls::Certificates::generate()?.save(&tls_path)?;
        println!("Development TLS identities created. Keep gateway/server keys private.");
        return Ok(());
    }
    let signup = args.iter().any(|s| s == "--signup");
    let setup = signup || args.iter().any(|s| s == "--setup");
    let client_only = args.iter().any(|s| s == "--client");
    let serve = args.iter().any(|s| s == "--serve");
    if serve && client_only {
        return Err("Choose either --serve or --client.".into());
    }
    if setup && (serve || client_only) {
        return Err("Use local --signup or --setup mode for staff registration.".into());
    }
    let base_port: u16 = std::env::var("VOTING_PORT")
        .unwrap_or_else(|_| "7440".into())
        .parse()?;
    if base_port > 65532 {
        return Err("VOTING_PORT must be at most 65532.".into());
    }
    // Remote clients need only the CA certificate, never the database or pepper.
    let network;
    let client = if client_only {
        ApiClient::connect(&std::fs::read(tls_path.join("ca.der"))?, base_port)?
    } else {
        let auth = auth::login::Auth::from_env(mailbox)?;
        let path = std::env::var("VOTING_DB").unwrap_or_else(|_| "voting.db".into());
        let conn = database::open(path)?;
        if let Ok(email) = std::env::var("VOTING_SYSTEM_EMAIL") {
            let email = validation::email(&email, false)?;
            conn.execute(
                "UPDATE system_settings SET value=?1 WHERE key='system_email'",
                [email],
            )?;
        }
        if setup {
            auth.approve_setup(&conn)?;
        }
        let certs = if serve {
            network::tls::Certificates::load(&tls_path)?
        } else {
            network::tls::Certificates::generate()?
        };
        network = Network::start(conn, auth, certs, serve.then_some(base_port), setup)?;
        if serve {
            println!("TLS services on loopback: database {base_port}, admin {}, district {}, voter {}. Ctrl+C to stop.",base_port+1,base_port+2,base_port+3);
            loop {
                std::thread::park();
            }
        }
        network.client.clone()
    };
    let app = if signup {
        App::signup(client)?
    } else {
        App::remote(client, setup)
    };
    let terminal = ratatui::init();
    let result = run(terminal, app);
    ratatui::restore();
    result?;
    Ok(())
}
fn init_security_tracing() -> AppResult<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::EnvFilter;

    std::fs::create_dir_all("logs")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions("logs", std::fs::Permissions::from_mode(0o700))?;
    }

    let appender = tracing_appender::rolling::daily("logs", "security.jsonl");

    let (writer, guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_current_span(false)
        .with_span_list(false)
        .with_writer(writer)
        .with_env_filter(EnvFilter::new("security=info"))
        .try_init()
        .map_err(std::io::Error::other)?;

    Ok(guard)
}
