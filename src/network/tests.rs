//! These tests use actual TCP sockets and both TLS hops, not mocked transports.
use super::*;
use std::{
    fs,
    io::{Read, Write},
};
struct Fixture {
    network: Network,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("tls-voting-{}", Uuid::new_v4()));
        let mut conn = Connection::open_in_memory().unwrap();
        crate::database::initialize(&mut conn).unwrap();
        let auth = auth::login::Auth::for_test(root.clone());
        services::voters::provision(
            &mut conn,
            &auth,
            "admin@example.test",
            "+15550000001",
            "Admin",
            "password1234",
            Role::Admin,
        )
        .unwrap();
        services::voters::provision(
            &mut conn,
            &auth,
            "do@example.test",
            "+15550000002",
            "Official",
            "password1234",
            Role::DistrictOfficial,
        )
        .unwrap();
        let network = Network::start(
            conn,
            auth,
            tls::Certificates::generate().unwrap(),
            None,
            false,
        )
        .unwrap();
        Self { network, root }
    }
    fn message(&self, phone: &str, prefix: &str) -> String {
        fs::read_to_string(auth::login::mailbox_path(&self.root, phone))
            .unwrap()
            .lines()
            .filter_map(|s| s.strip_prefix(prefix))
            .next_back()
            .unwrap()
            .to_owned()
    }
    fn login(&self, role: Role) -> Session {
        let (email, phone) = if role == Role::Admin {
            ("admin@example.test", "+15550000001")
        } else {
            ("do@example.test", "+15550000002")
        };
        let mut pending = Some(
            ClientAuth
                .password_login(&self.network.client, email, "password1234", role)
                .unwrap(),
        );
        let code = self.message(phone, "Login OTP: ");
        ClientAuth
            .complete_otp(&self.network.client, &mut pending, &code)
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[test]
fn real_tls_full_election_and_role_privacy() {
    let f = Fixture::new();
    let mut api = f.network.client.clone();
    let admin = f.login(Role::Admin);
    let official = f.login(Role::DistrictOfficial);
    let e = client::election::create(&mut api, &admin, "TLS Election").unwrap();
    client::voters::register(
        &mut api,
        &ClientAuth,
        &admin,
        e,
        &VoterInput {
            full_name: "Network Voter".into(),
            email: "v@example.test".into(),
            phone: "+15550000003".into(),
            dob: "2000-01-02".into(),
            nid: "TLS-VOTER".into(),
        },
    )
    .unwrap();
    let pin = f.message("+15550000003", "Voting PIN: ");
    let mut pending = Some(ClientAuth.pin_login(&api, &pin).unwrap());
    let code = f.message("+15550000003", "Login OTP: ");
    let voter = ClientAuth.complete_otp(&api, &mut pending, &code).unwrap();
    assert!(client::election::start(&mut api, &admin, e).is_err());
    client::election::start(&mut api, &official, e).unwrap();
    let positions = client::election::positions(&api, &voter, e).unwrap();
    let choices: Vec<_> = positions
        .iter()
        .map(|p| (p.id, p.candidates[0].id))
        .collect();
    client::election::cast(&mut api, &voter, e, &choices).unwrap();
    assert!(client::election::cast(&mut api, &voter, e, &choices).is_err());
    assert!(client::election::results(&api, &admin, e).is_err());
    assert!(client::voters::list(&api, &voter, e).is_err());
    client::election::close(&mut api, &official, e).unwrap();
    client::election::tally(&mut api, &official, e).unwrap();
    client::election::announce(&mut api, &official, e).unwrap();
    assert_eq!(
        client::election::results(&api, &official, e)
            .unwrap()
            .iter()
            .map(|r| r.votes)
            .sum::<i64>(),
        3
    );
    let log = client::election::audit_log(&api, &admin, e)
        .unwrap()
        .join("\n");
    assert!(log.contains("voter_participated"));
    assert!(!log.contains(&pin));
    assert!(!log.contains("Hamberger"));
    assert_eq!(f.network.count("ballot_choices"), 3);
    api.logout(&admin).unwrap();
    assert!(client::election::list(&api, &admin).is_err());
}
#[test]
fn tls_rejects_untrusted_ca_wrong_hostname_plaintext_and_missing_client_certificate() {
    let f = Fixture::new();
    let api = &f.network.client;
    let request = Request {
        role: Role::Admin,
        token: None,
        operation: "election.list".into(),
        args: Value::Null,
    };
    let wrong = tls::Certificates::generate().unwrap();
    assert!(tls::exchange(
        api.addresses[0],
        tls::client_config(&wrong.ca, None).unwrap(),
        &request
    )
    .is_err());
    assert!(tls::exchange_named(
        api.addresses[0],
        api.config.clone(),
        &request,
        "attacker.invalid"
    )
    .is_err());
    assert!(tls::exchange(f.network.db_address, api.config.clone(), &request).is_err());
    let mut plain = tls::socket(api.addresses[0]).unwrap();
    plain
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut bytes = [0; 128];
    let result = plain.read(&mut bytes);
    assert!(result.is_err() || result.unwrap() == 0 || bytes[0] == 21); // TLS alert, never application JSON.
}
#[test]
fn session_and_challenge_cannot_be_forged_replayed_or_used_on_another_gateway() {
    let f = Fixture::new();
    let api = &f.network.client;
    let mut pending = Some(
        ClientAuth
            .password_login(api, "admin@example.test", "password1234", Role::Admin)
            .unwrap(),
    );
    let challenge = pending.as_ref().unwrap().token.clone();
    let code = f.message("+15550000001", "Login OTP: ");
    let admin = ClientAuth.complete_otp(api, &mut pending, &code).unwrap();
    assert!(api
        .request::<Session>(Role::Admin, Some(challenge), "otp", json!([code]))
        .is_err());
    assert!(api
        .request::<Vec<Election>>(
            Role::Admin,
            Some("forged-session".into()),
            "election.list",
            Value::Null
        )
        .is_err());
    assert!(api
        .request::<Vec<Election>>(
            Role::DistrictOfficial,
            Some(admin.token.clone()),
            "election.list",
            Value::Null
        )
        .is_err());
    let request = Request {
        role: Role::Voter,
        token: Some(admin.token.clone()),
        operation: "election.list".into(),
        args: Value::Null,
    };
    assert!(
        tls::exchange(api.addresses[0], api.config.clone(), &request)
            .unwrap()
            .error
            .is_some()
    );
    // Session metadata is display-only. The server ignores a client-edited user ID.
    let mut forged = admin.clone();
    forged.user_id = 9999;
    assert!(client::election::list(api, &forged).is_ok());
    f.network
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&admin.token)
        .unwrap()
        .expires = Instant::now();
    assert!(client::election::list(api, &admin).is_err());
}
#[test]
fn otp_expiration_retry_limit_and_setup_rejection_over_tls() {
    let f = Fixture::new();
    let api = &f.network.client;
    let mut pending = Some(
        ClientAuth
            .password_login(api, "admin@example.test", "password1234", Role::Admin)
            .unwrap(),
    );
    let code = f.message("+15550000001", "Login OTP: ");
    let wrong = if code == "000000" { "111111" } else { "000000" };
    for _ in 0..3 {
        assert!(ClientAuth.complete_otp(api, &mut pending, wrong).is_err());
    }
    assert!(ClientAuth.complete_otp(api, &mut pending, &code).is_err());
    let mut pending = Some(
        ClientAuth
            .password_login(
                api,
                "do@example.test",
                "password1234",
                Role::DistrictOfficial,
            )
            .unwrap(),
    );
    let code = f.message("+15550000002", "Login OTP: ");
    f.network
        .state
        .lock()
        .unwrap()
        .pending
        .get_mut(&pending.as_ref().unwrap().token)
        .unwrap()
        .expires = Instant::now();
    assert!(ClientAuth.complete_otp(api, &mut pending, &code).is_err());
    assert!(api
        .request::<()>(
            Role::Admin,
            None,
            "provision",
            json!([
                "bad@example.test",
                "+15550000005",
                "Bad",
                "password1234",
                Role::Admin
            ])
        )
        .is_err());
}
#[test]
fn framing_rejects_oversized_messages_before_allocating_payload() {
    let bytes = (1024u32 * 1024 + 1).to_be_bytes();
    assert!(tls::read_frame::<Request>(&mut &bytes[..]).is_err());
}

#[test]
fn internal_certificate_role_binding_and_transport_incident_log() {
    let root = std::env::temp_dir().join(format!("tls-identity-{}", Uuid::new_v4()));
    let mut conn = Connection::open_in_memory().unwrap();
    crate::database::initialize(&mut conn).unwrap();
    let certs = tls::Certificates::generate().unwrap();
    let admin_config = tls::client_config(&certs.ca, Some(&certs.gateways[0])).unwrap();
    let net = Network::start(
        conn,
        auth::login::Auth::for_test(root.clone()),
        certs,
        None,
        false,
    )
    .unwrap();
    let request = Request {
        role: Role::DistrictOfficial,
        token: None,
        operation: "election.list".into(),
        args: Value::Null,
    };
    let response = tls::exchange(net.db_address, admin_config, &request).unwrap();
    assert_eq!(response.error.as_deref(), Some("Wrong role gateway."));
    for _ in 0..11 {
        let mut socket = tls::socket(net.client.addresses[0]).unwrap();
        socket.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
        let mut byte = [0];
        let _ = socket.read(&mut byte);
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let count:i64=net.state.lock().unwrap().conn.query_row("SELECT COUNT(*) FROM audit_events WHERE event_type='tls_or_protocol_flood' AND severity=2",[],|r|r.get(0)).unwrap();
        if count > 0 {
            break;
        }
        assert!(Instant::now() < deadline, "TLS flood was not recorded");
        thread::sleep(Duration::from_millis(20));
    }
    drop(net);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn signup_responses_close_tls_cleanly_on_both_hops() {
    use rustls::pki_types::ServerName;
    let root = std::env::temp_dir().join(format!("tls-close-{}", Uuid::new_v4()));
    let mut conn = Connection::open_in_memory().unwrap();
    crate::database::initialize(&mut conn).unwrap();
    let certs = tls::Certificates::generate().unwrap();
    let internal = tls::client_config(&certs.ca, Some(&certs.gateways[0])).unwrap();
    let net = Network::start(
        conn,
        auth::login::Auth::for_test(root.clone()),
        certs,
        None,
        true,
    )
    .unwrap();
    for (i, address, config) in [
        (0, net.client.addresses[0], net.client.config.clone()),
        (1, net.db_address, internal),
    ] {
        let connection =
            rustls::ClientConnection::new(config, ServerName::try_from("localhost").unwrap())
                .unwrap();
        let mut stream = rustls::StreamOwned::new(connection, tls::socket(address).unwrap());
        let role = if i == 0 {
            Role::Admin
        } else {
            Role::DistrictOfficial
        };
        let request = Request {
            role: Role::Admin,
            token: net.client.setup_token.clone(),
            operation: "provision".into(),
            args: json!([
                format!("staff{i}@example.test"),
                format!("+1555000000{i}"),
                "Staff",
                "password1234",
                role
            ]),
        };
        tls::write_frame(&mut stream, &request).unwrap();
        let reply: Response = tls::read_frame(&mut stream).unwrap();
        assert!(reply.error.is_none(), "{:?}", reply.error);
        let mut byte = [0];
        assert_eq!(
            stream
                .read(&mut byte)
                .expect("server must send close_notify after signup response"),
            0
        );
        stream.conn.send_close_notify();
        let _ = stream.flush();
    }
    assert_eq!(net.count("user_credentials"), 2);
    drop(net);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn incomplete_frames_are_rejected_even_when_the_peer_closes() {
    let bytes = [0, 0, 0, 8, b'{', b'}'];
    let error = tls::read_frame::<Value>(&mut &bytes[..]).unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}

#[test]
fn client_closes_its_write_side_and_waits_for_a_slow_response() {
    // Reproduce slow debug-build password hashing without burning CPU in bcrypt.
    // Eleven seconds exceeds both the old idle (5s) and whole-request (10s) limits.
    let certs = tls::Certificates::generate().unwrap();
    let server_config = certs.server_config(false).unwrap();
    let client_config = tls::client_config(&certs.ca, None).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        tls::configure(&socket).unwrap();
        let mut stream = rustls::StreamOwned::new(
            rustls::ServerConnection::new(server_config).unwrap(),
            tls::DeadlineSocket::new(socket),
        );
        let _: Request = tls::read_frame(&mut stream).unwrap();
        stream.sock.begin_response();
        let mut byte = [0];
        assert_eq!(
            stream
                .read(&mut byte)
                .expect("client must send close_notify"),
            0
        );
        thread::sleep(Duration::from_secs(11));
        tls::write_frame(
            &mut stream,
            &Response {
                value: json!("complete"),
                error: None,
            },
        )
        .unwrap();
        stream.conn.send_close_notify();
        stream.flush().unwrap();
    });
    let request = Request {
        role: Role::Admin,
        token: None,
        operation: "test".into(),
        args: Value::Null,
    };
    let result = tls::exchange(address, client_config, &request);
    server.join().unwrap();
    assert_eq!(result.unwrap().value, json!("complete"));
}

#[test]
fn configured_worker_waits_for_data_on_an_inherited_nonblocking_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    // Model BSD/macOS inheritance explicitly so Linux exercises the same case.
    server.set_nonblocking(true).unwrap();
    tls::configure(&server).unwrap();
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        client.write_all(b"x").unwrap();
    });
    let mut byte = [0];
    let result = server.read_exact(&mut byte);
    writer.join().unwrap();
    result.expect("configured TLS worker must wait instead of returning WouldBlock");
    assert_eq!(byte, *b"x");
}
