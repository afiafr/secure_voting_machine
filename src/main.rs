//! Startup and command-line modes for the voting application.
mod auth;
mod database;
mod models;
mod network;
mod services;
#[cfg(test)]
mod test_support;
mod ui;
use network::{ApiClient, Network};
use services::AppResult;
use std::path::PathBuf;
use ui::{run, App};

fn main() -> AppResult<()> {
    dotenvy::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|s| s == "--version") {
        println!(
            "secure_voting_machine {} (blocking TLS worker socket fix)",
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
            std::fs::read_to_string(auth::login::mailbox_path(&mailbox, phone))?
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
