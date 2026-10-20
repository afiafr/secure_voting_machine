# Implementation details — code map

Version 0.2.5. Generated signatures describe parameters and return types; summaries
come from source comments. `Result` errors propagate to the caller, and the TUI shows
a safe message. See STRIDE.md for authorization requirements and security boundaries.

## Folder contracts

| Folder/file | Responsibility and contract |
| --- | --- |
| src/ui | Screens, key events, forms and client validation. No direct production SQLite access. |
| src/network/client | Typed requests; tokens carry identity, not caller-supplied user IDs. |
| src/network | TLS framing, separate gateway identities, database dispatch and session lifecycle. |
| src/auth | Password/PIN verification, OTP, retry counters, local message delivery. |
| src/services | Role/ownership/state rules and transactional domain operations. |
| src/database | Idempotent schema setup, migrations and database constraints. |
| src/validation.rs | Shared format checks; frontend convenience and backend enforcement. |
| src/security.rs | Deadlines and structured security events, without raw credentials. |
| src/models.rs | Serialized role and domain data types. |
| tests | Real terminal workflow script; Rust tests live alongside the modules they test. |
| scripts | Regenerate this inventory after editing doc comments. |

## Database specification

`src/database/schema.sql` defines users, password/PIN credentials, elections, positions,
candidates, position assignments, enrollment, anonymous ballots/choices, candidate/party
results, audit/security events, retry counters, notification/email outboxes and system
settings. Foreign keys constrain election/position/candidate relationships. Triggers
freeze live configuration and prohibit accepted-vote/audit edits. Outboxes provide
retryable delivery, not a transaction spanning SQLite and the filesystem.

## Other files

Cargo.toml declares direct crates/features; Cargo.lock pins the resolved dependency tree.
README.md is the installation/run guide; STRIDE.md is the threat/control/test map;
dummy.md introduces the code; docs/DEMO.md addresses oral-exam questions 9–14;
docs/TESTING.md reports validation. `.env.example` lists configuration names without
real secrets. LICENSE records licensing. Documentation is kept in Markdown and
Rust source comments; generated HTML is not included.

## Function inventory

All Rust functions, including test helpers, appear below. Signatures are authoritative
for inputs and outputs; follow the source link for implementation and error branches.
Tests run only under `cfg(test)` and must not be confused with production endpoints.

### [src/auth/login.rs](src/auth/login.rs)

Authentication, bounded retries, and a local virtual-SMS delivery adapter. A Session is issued only after both credential and OTP verification.

#### `user_id`

Return the authenticated user ID.

```rust
pub fn user_id(&self) -> i64
```

#### `role`

Return the authenticated role.

```rust
pub fn role(&self) -> Role
```

#### `for_test`

Internal helper: for test.

```rust
pub(crate) fn for_test(user_id: i64, role: Role) -> Self
```

#### `from_env`

Load and validate the password pepper; configure normal bcrypt cost.

```rust
pub fn from_env(mailbox: PathBuf) -> AppResult<Self>
```

#### `for_test`

Internal helper: for test.

```rust
pub(crate) fn for_test(mailbox: PathBuf) -> Self
```

#### `password_input`

Combine the password with the pepper, rejecting bcrypt input overflow.

```rust
fn password_input(&self, password: &str) -> AppResult<String>
```

#### `password_hash`

Hash a signup password using bcrypt and the configured pepper.

```rust
pub(crate) fn password_hash(&self, password: &str) -> AppResult<String>
```

#### `lookup`

Create a keyed digest for lookup/rate-limit scopes without storing the raw secret.

```rust
pub(crate) fn lookup(&self, value: &str) -> String
```

#### `new_pin`

Generate a random decimal PIN and return its lookup digest and bcrypt verifier.

```rust
pub(crate) fn new_pin(&self) -> AppResult<(String, String, String)>
```

#### `password_login`

Verify staff credentials and retry limits; issue an OTP challenge on success.

```rust
pub fn password_login( &self, conn: &Connection, email: &str, password: &str, role: Role, ) -> AppResult<PendingLogin>
```

#### `pin_login`

Verify a voter PIN and its scoped retry limit; issue an OTP challenge.

```rust
pub fn pin_login(&self, conn: &Connection, pin: &str) -> AppResult<PendingLogin>
```

#### `issue_otp`

Deliver an expiring virtual OTP and return the pending verification state.

```rust
fn issue_otp(&self, conn: &Connection, id: i64, role: Role) -> AppResult<PendingLogin>
```

#### `complete_otp`

Consume a valid OTP once and issue a session; count failures and expire challenges.

```rust
pub fn complete_otp( &self, conn: &Connection, pending: &mut Option<PendingLogin>, code: &str, ) -> AppResult<Session>
```

#### `flush_notifications`

Retry queued alerts after UI actions. A crash may duplicate an alert, but it cannot erase the underlying committed audit event.

```rust
pub fn flush_notifications(&self, conn: &Connection) -> AppResult<()>
```

#### `approve_setup`

Local engineer approval before enabling privileged staff provisioning.

```rust
pub fn approve_setup(&self, conn: &Connection) -> AppResult<()>
```

#### `flush_emails`

Append email-style alerts. Failed delivery leaves the durable outbox pending.

```rust
pub fn flush_emails(&self, conn: &Connection) -> AppResult<()>
```

#### `deliver`

Virtual SMS is a same-host teaching simulation, not independent MFA. Credentials are never included in the election audit log or admin screen.

```rust
pub(crate) fn deliver(&self, phone: &str, message: &str) -> AppResult<()>
```

#### `mailbox_path`

Use the validated phone number as the inbox filename; reject path characters.

```rust
pub fn mailbox_path(root: &Path, phone: &str) -> AppResult<PathBuf>
```

#### `now`

Return Unix seconds for persisted authentication expiry timestamps.

```rust
fn now() -> i64
```

#### `check_limit`

Reject authentication while its scope is in cooldown; clear expired cooldowns.

```rust
fn check_limit(conn: &Connection, scope: &str) -> AppResult<()>
```

#### `reset_limit`

Clear the successful or expired authentication retry counter.

```rust
fn reset_limit(conn: &Connection, scope: &str) -> AppResult<()>
```

#### `failed`

Increment a bounded retry record, apply cooldown and log sanitized failure/lockout events.

```rust
fn failed( conn: &Connection, scope: &str, threshold: i64, event: &str, user: Option<i64>, ) -> AppResult<()>
```

#### `otp_message`

Render a virtual text with New York time; keep the existing code line for readers/tests.

```rust
fn otp_message(name: &str, phone: &str, purpose: &str, code: &str, sent: DateTime<Utc>) -> String
```

#### `otp_text_names_recipient_and_formats_est_and_edt`

Internal helper: otp text names recipient and formats est and edt.

```rust
fn otp_text_names_recipient_and_formats_est_and_edt()
```

#### `expiry_is_five_minutes_even_when_clocks_fall_back`

Internal helper: expiry is five minutes even when clocks fall back.

```rust
fn expiry_is_five_minutes_even_when_clocks_fall_back()
```

### [src/auth/mod.rs](src/auth/mod.rs)



### [src/database/mod.rs](src/database/mod.rs)

Schema setup and a non-destructive migration from the uploaded prototype.

#### `open`

Open the SQLite file with private POSIX permissions and initialize its schema.

```rust
pub fn open(path: impl AsRef<Path>) -> Result<Connection>
```

#### `initialize`

Create tables/triggers transactionally and preserve compatible existing accounts.

```rust
pub fn initialize(conn: &mut Connection) -> Result<()>
```

### [src/main.rs](src/main.rs)

Startup and command-line modes for the voting application.

#### `main`

Parse command modes, load configuration and start the TUI or TLS services.

```rust
fn main() -> AppResult<()>
```

#### `init_security_tracing`

Internal helper: init security tracing.

```rust
fn init_security_tracing() -> AppResult<tracing_appender::non_blocking::WorkerGuard>
```

### [src/models.rs](src/models.rs)

Data transferred between SQLite services and the terminal UI.

#### `as_str`

Return the stable database role name.

```rust
pub fn as_str(self) -> &'static str
```

#### `label`

Return the role label shown in the TUI.

```rust
pub fn label(self) -> &'static str
```

### [src/network/client/election.rs](src/network/client/election.rs)

Typed election calls. The backend checks role, ownership and election state.

#### `list`

Return records visible to the authenticated role and ownership scope.

```rust
pub fn list(conn: &ApiClient, session: &Session) -> AppResult<Vec<Election>>
```

#### `get`

Read one authorized election by its ID.

```rust
pub fn get(conn: &ApiClient, session: &Session, id: i64) -> AppResult<Election>
```

#### `positions`

Read the authorized election positions and their assigned candidates.

```rust
pub fn positions(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<Position>>
```

#### `create`

Create a draft election and default positions/candidates in one transaction.

```rust
pub fn create(conn: &mut ApiClient, session: &Session, name: &str) -> AppResult<i64>
```

#### `rename`

Rename an owned draft election and record the change.

```rust
pub fn rename(conn: &mut ApiClient, session: &Session, election: i64, name: &str) -> AppResult<()>
```

#### `delete`

Cancel an owned draft election while preserving its audit history.

```rust
pub fn delete(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()>
```

#### `add_position`

Add a position to an owned draft and record the change.

```rust
pub fn add_position( conn: &mut ApiClient, session: &Session, election: i64, name: &str, ) -> AppResult<()>
```

#### `rename_position`

Rename an owned draft position after validating election membership.

```rust
pub fn rename_position( conn: &mut ApiClient, session: &Session, election: i64, position: i64, name: &str, ) -> AppResult<()>
```

#### `delete_position`

Remove a draft position and its assignments, preserving minimum constraints.

```rust
pub fn delete_position( conn: &mut ApiClient, session: &Session, election: i64, position: i64, ) -> AppResult<()>
```

#### `add_candidate`

Create and assign a candidate to a draft position transactionally.

```rust
pub fn add_candidate( conn: &mut ApiClient, session: &Session, election: i64, position: i64, name: &str, party: &str, ) -> AppResult<()>
```

#### `link_candidate`

Assign an existing candidate to another position in the same draft election.

```rust
pub fn link_candidate( conn: &mut ApiClient, session: &Session, election: i64, position: i64, candidate: i64, ) -> AppResult<()>
```

#### `update_candidate`

Update a draft candidate name/party and audit the change.

```rust
pub fn update_candidate( conn: &mut ApiClient, session: &Session, election: i64, candidate: i64, name: &str, party: &str, ) -> AppResult<()>
```

#### `unlink_candidate`

Remove a candidate assignment from a draft position.

```rust
pub fn unlink_candidate( conn: &mut ApiClient, session: &Session, election: i64, position: i64, candidate: i64, ) -> AppResult<()>
```

#### `start`

Require a district official, validate minimum ballot configuration and open voting.

```rust
pub fn start(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()>
```

#### `close`

Require a district official, close the open election and record its timestamp.

```rust
pub fn close(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()>
```

#### `cast`

Atomically record an eligible voter’s complete ballot and participation; reject repeat votes.

```rust
pub fn cast( conn: &mut ApiClient, session: &Session, election: i64, choices: &[(i64, i64)], ) -> AppResult<()>
```

#### `tally`

Require a closed election and district official; persist candidate/party totals.

```rust
pub fn tally(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()>
```

#### `announce`

Require district-official authorization and a tally; mark results announced.

```rust
pub fn announce(conn: &mut ApiClient, session: &Session, election: i64) -> AppResult<()>
```

#### `results`

Require a district official and return candidate vote totals without voter identities.

```rust
pub fn results(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<ResultRow>>
```

#### `party_results`

Require a district official and return party totals by position.

```rust
pub fn party_results(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<String>>
```

#### `audit_log`

Return the owning admin’s lifecycle/participation view, excluding ballot choices.

```rust
pub fn audit_log(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<String>>
```

#### `flag_incident`

Record a suspicious election change for investigation.

```rust
pub fn flag_incident(conn: &ApiClient, session: &Session, election: i64) -> AppResult<()>
```

#### `audit_list`

Internal helper: audit list.

```rust
pub fn audit_list(conn: &ApiClient, session: &Session) -> AppResult<Vec<Election>>
```

### [src/network/client/mod.rs](src/network/client/mod.rs)

Client transport, authentication, and typed domain endpoint calls.

#### `connect`

Configure a CA-trusting client for the three role gateway addresses.

```rust
pub fn connect(ca: &[u8], base_port: u16) -> AppResult<Self>
```

#### `request`

Send one typed TLS request and decode its response or transport/service error.

```rust
pub(super) fn request<T: serde::de::DeserializeOwned>( &self, role: Role, token: Option<String>, operation: &str, args: Value, ) -> AppResult<T>
```

#### `call`

Send an authenticated operation using the server-issued session token.

```rust
pub(super) fn call<T: serde::de::DeserializeOwned>( &self, session: &Session, operation: &str, args: impl serde::Serialize, ) -> AppResult<T>
```

#### `logout`

Clear local session, pending authentication and role-specific screen data.

```rust
pub fn logout(&self, session: &Session) -> AppResult<()>
```

#### `report_error`

Report a client operation failure; the server stores only a fixed safe description.

```rust
pub fn report_error( &self, session: Option<&Session>, screen: &str, message: &str, ) -> AppResult<()>
```

#### `password_login`

Verify staff credentials and retry limits; issue an OTP challenge on success.

```rust
pub fn password_login( &self, conn: &ApiClient, email: &str, password: &str, role: Role, ) -> AppResult<PendingLogin>
```

#### `pin_login`

Verify a voter PIN and its scoped retry limit; issue an OTP challenge.

```rust
pub fn pin_login(&self, conn: &ApiClient, pin: &str) -> AppResult<PendingLogin>
```

#### `complete_otp`

Consume a valid OTP once and issue a session; count failures and expire challenges.

```rust
pub fn complete_otp( &self, conn: &ApiClient, pending: &mut Option<PendingLogin>, code: &str, ) -> AppResult<Session>
```

#### `provision`

Create a staff account and password verifier through the engineer setup capability.

```rust
pub fn provision( conn: &mut ApiClient, _auth: &ClientAuth, email: &str, phone: &str, name: &str, password: &str, role: Role, ) -> AppResult<()>
```

### [src/network/client/voters.rs](src/network/client/voters.rs)

Typed voter calls. The backend checks role and ownership.

#### `register`

Register identity, PIN verifier, enrollment and audit transactionally; deliver the PIN privately.

```rust
pub fn register( conn: &mut ApiClient, auth: &ClientAuth, session: &Session, election: i64, input: &VoterInput, ) -> AppResult<i64>
```

#### `list`

Return records visible to the authenticated role and ownership scope.

```rust
pub fn list(conn: &ApiClient, session: &Session, election: i64) -> AppResult<Vec<Voter>>
```

#### `enroll`

Enroll an existing voter by NID without regenerating their PIN.

```rust
pub fn enroll(conn: &mut ApiClient, session: &Session, election: i64, nid: &str) -> AppResult<()>
```

#### `remove`

Remove an eligible unvoted enrollment; do not delete accepted voting history.

```rust
pub fn remove(conn: &mut ApiClient, session: &Session, election: i64, voter: i64) -> AppResult<()>
```

#### `update`

Update allowed voter profile fields; forbid ordinary OTP-phone/NID reassignment.

```rust
pub fn update( conn: &mut ApiClient, session: &Session, election: i64, voter: i64, input: &VoterInput, ) -> AppResult<()>
```

#### `pin_receipt`

Confirm delivery without returning the credential to the administrator.

```rust
pub fn pin_receipt( conn: &ApiClient, session: &Session, election: i64, voter: i64, received: bool, ) -> AppResult<()>
```

### [src/network/dispatch.rs](src/network/dispatch.rs)

Explicit endpoint allowlist; no endpoint accepts SQL or a caller-selected actor ID.

#### `service_call`

Decode an allowlisted operation and invoke the matching domain service.

```rust
pub(super) fn service_call( state: &mut State, session: &auth::login::Session, operation: &str, args: Value, ) -> AppResult<Value>
```

### [src/network/mod.rs](src/network/mod.rs)

Four loopback TLS listeners: three role gateways and one private database service. Gateways use distinct mTLS identities. Database authorization uses server-held sessions.

#### `role_index`

Map a role to its dedicated gateway address/certificate index.

```rust
fn role_index(role: Role) -> usize
```

### [src/network/monitor.rs](src/network/monitor.rs)

Aggregate rejected transports so floods cannot create one database write per packet.

#### `start`

Start bounded transport monitoring and aggregate security incidents.

```rust
pub(super) fn start( state: Arc<Mutex<State>>, stop: Arc<AtomicBool>, rejected: Arc<AtomicUsize>, timeouts: Arc<AtomicUsize>, ) -> thread::JoinHandle<()>
```

### [src/network/protocol.rs](src/network/protocol.rs)

Wire records. Session tokens are opaque credentials, not caller-selected identities.

#### `user_id`

Return the authenticated user ID.

```rust
pub fn user_id(&self) -> i64
```

#### `role`

Return the authenticated role.

```rust
pub fn role(&self) -> Role
```

### [src/network/registry.rs](src/network/registry.rs)

Fictional read-only statewide registry, isolated behind an authenticated TLS service.

#### `event`

Internal helper: event.

```rust
fn event(&self) -> &'static str
```

#### `fmt`

Internal helper: fmt.

```rust
fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
```

#### `verify`

Internal helper: verify.

```rust
pub(super) fn verify(&self, id: &str, name: &str, dob: &str, phone: &str) -> AppResult<()>
```

#### `registration`

Internal helper: registration.

```rust
pub(super) fn registration( &self, conn: &Connection, session: &auth::login::Session, election: i64, input: &VoterInput, ) -> AppResult<VoterInput>
```

#### `registry_database`

Internal helper: registry database.

```rust
fn registry_database() -> AppResult<Connection>
```

#### `start`

Internal helper: start.

```rust
pub(super) fn start(stop: Arc<AtomicBool>) -> AppResult<(Registry, thread::JoinHandle<()>)>
```

#### `shipped_registry_matches_documented_records_and_is_read_only`

Internal helper: shipped registry matches documented records and is read only.

```rust
fn shipped_registry_matches_documented_records_and_is_read_only()
```

#### `registry_tls_fails_closed_and_requires_client_certificate`

Internal helper: registry tls fails closed and requires client certificate.

```rust
fn registry_tls_fails_closed_and_requires_client_certificate()
```

### [src/network/server.rs](src/network/server.rs)

TLS listeners, gateway forwarding, and worker lifecycle.

#### `start`

Bind the database and role gateways, establish certificate binding and own worker lifetimes.

```rust
pub fn start( conn: Connection, auth: auth::login::Auth, certs: tls::Certificates, base_port: Option<u16>, setup: bool, ) -> AppResult<Self>
```

#### `test_session`

Internal helper: test session.

```rust
pub fn test_session(&self, id: i64, role: Role) -> Session
```

#### `count`

Internal helper: count.

```rust
pub fn count(&self, table: &str) -> i64
```

#### `drop`

Internal helper: drop.

```rust
fn drop(&mut self)
```

#### `response`

Convert service success/failure into the wire response envelope.

```rust
fn response(result: AppResult<Value>) -> Response
```

#### `listen`

Accept bounded connections and run TLS workers with a ten-second socket watchdog.

```rust
pub(super) fn listen( listener: TcpListener, config: Arc<rustls::ServerConfig>, stop: Arc<AtomicBool>, rejected: Arc<AtomicUsize>, timeouts: Arc<AtomicUsize>, handler: impl Fn(Request, Option<Vec<u8>>) -> Response + Send + Sync + 'static, ) -> AppResult<thread::JoinHandle<()>>
```

### [src/network/state.rs](src/network/state.rs)

Database-side login challenges, session lifetime, and authenticated dispatch.

#### `dispatch`

Validate gateway role, authenticate challenges/sessions and dispatch authorized operations.

```rust
pub(super) fn dispatch(&mut self, request: Request, peer_role: Role) -> AppResult<Value>
```

### [src/network/tests.rs](src/network/tests.rs)

These tests use actual TCP sockets and both TLS hops, not mocked transports.

#### `new`

Test: new.

```rust
fn new() -> Self
```

#### `message`

Test: message.

```rust
fn message(&self, phone: &str, prefix: &str) -> String
```

#### `login`

Test: login.

```rust
fn login(&self, role: Role) -> Session
```

#### `drop`

Test: drop.

```rust
fn drop(&mut self)
```

#### `real_tls_full_election_and_role_privacy`

Test: real tls full election and role privacy.

```rust
fn real_tls_full_election_and_role_privacy()
```

#### `tls_rejects_untrusted_ca_wrong_hostname_plaintext_and_missing_client_certificate`

Test: tls rejects untrusted ca wrong hostname plaintext and missing client certificate.

```rust
fn tls_rejects_untrusted_ca_wrong_hostname_plaintext_and_missing_client_certificate()
```

#### `session_and_challenge_cannot_be_forged_replayed_or_used_on_another_gateway`

Test: session and challenge cannot be forged replayed or used on another gateway.

```rust
fn session_and_challenge_cannot_be_forged_replayed_or_used_on_another_gateway()
```

#### `otp_expiration_retry_limit_and_setup_rejection_over_tls`

Test: otp expiration retry limit and setup rejection over tls.

```rust
fn otp_expiration_retry_limit_and_setup_rejection_over_tls()
```

#### `framing_rejects_oversized_messages_before_allocating_payload`

Test: framing rejects oversized messages before allocating payload.

```rust
fn framing_rejects_oversized_messages_before_allocating_payload()
```

#### `internal_certificate_role_binding_and_transport_incident_log`

Test: internal certificate role binding and transport incident log.

```rust
fn internal_certificate_role_binding_and_transport_incident_log()
```

#### `signup_responses_close_tls_cleanly_on_both_hops`

Test: signup responses close tls cleanly on both hops.

```rust
fn signup_responses_close_tls_cleanly_on_both_hops()
```

#### `incomplete_frames_are_rejected_even_when_the_peer_closes`

Test: incomplete frames are rejected even when the peer closes.

```rust
fn incomplete_frames_are_rejected_even_when_the_peer_closes()
```

#### `client_rejects_response_after_ten_second_deadline`

Test: client rejects response after ten second deadline.

```rust
fn client_rejects_response_after_ten_second_deadline()
```

#### `configured_worker_waits_for_data_on_an_inherited_nonblocking_socket`

Test: configured worker waits for data on an inherited nonblocking socket.

```rust
fn configured_worker_waits_for_data_on_an_inherited_nonblocking_socket()
```

#### `direct_endpoints_reject_bad_fields_and_cross_role_reads`

Test: direct endpoints reject bad fields and cross role reads.

```rust
fn direct_endpoints_reject_bad_fields_and_cross_role_reads()
```

#### `expired_wire_request_cannot_create_election`

Test: expired wire request cannot create election.

```rust
fn expired_wire_request_cannot_create_election()
```

#### `server_watchdog_closes_stalled_handler_and_counts_timeout`

Test: server watchdog closes stalled handler and counts timeout.

```rust
fn server_watchdog_closes_stalled_handler_and_counts_timeout()
```

#### `registry_rejects_unknown_mismatched_and_underage_without_writes`

Test: registry rejects unknown mismatched and underage without writes.

```rust
fn registry_rejects_unknown_mismatched_and_underage_without_writes()
```

#### `registry_phone_forgery_minor_and_injection_attempts_have_private_audits`

Test: registry phone forgery minor and injection attempts have private audits.

```rust
fn registry_phone_forgery_minor_and_injection_attempts_have_private_audits()
```

#### `unauthenticated_voter_failures_do_not_make_individual_request_logs`

Test: unauthenticated voter failures do not make individual request logs.

```rust
fn unauthenticated_voter_failures_do_not_make_individual_request_logs()
```

### [src/network/tls.rs](src/network/tls.rs)

Loopback development PKI and bounded, length-prefixed JSON over TLS 1.3. Public clients trust only the CA. Only gateways hold client certificates.

#### `generate`

Generate a private development CA, server identity and three distinct gateway identities.

```rust
pub fn generate() -> AppResult<Self>
```

#### `save`

Write the certificate bundle with private permissions; refuse to overwrite an existing bundle.

```rust
pub fn save(&self, path: &std::path::Path) -> AppResult<()>
```

#### `load`

Read an existing development certificate/key bundle.

```rust
pub fn load(path: &std::path::Path) -> AppResult<Self>
```

#### `server_config`

Configure TLS 1.3, optionally requiring trusted client certificates.

```rust
pub fn server_config(&self, mutual: bool) -> AppResult<Arc<rustls::ServerConfig>>
```

#### `roots`

Build the explicit trust store from this project’s CA certificate.

```rust
fn roots(ca: &[u8]) -> AppResult<RootCertStore>
```

#### `client_config`

Configure TLS 1.3 CA verification and optional gateway client authentication.

```rust
pub fn client_config( ca: &[u8], identity: Option<&Identity>, ) -> AppResult<Arc<rustls::ClientConfig>>
```

#### `socket`

Connect with a bounded TCP timeout and configure blocking I/O.

```rust
pub fn socket(address: SocketAddr) -> AppResult<TcpStream>
```

#### `configure`

Restore blocking mode explicitly and apply TCP/socket timeouts.

```rust
pub fn configure(socket: &TcpStream) -> AppResult<()>
```

#### `new`

Internal helper: new.

```rust
pub fn new(socket: TcpStream) -> Self
```

#### `begin_response`

Preserve the original deadline across handshake, request and response.

```rust
pub fn begin_response(&mut self)
```

#### `limit`

Shorten the socket deadline without ever extending its original budget.

```rust
pub fn limit(&mut self, deadline: Instant)
```

#### `remaining`

Return the remaining monotonic request budget or a timeout error.

```rust
fn remaining(&self) -> std::io::Result<Duration>
```

#### `read`

Read bytes under the remaining request and idle timeout.

```rust
fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>
```

#### `write`

Write bytes under the remaining request and idle timeout.

```rust
fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>
```

#### `flush`

Flush the wrapped transport.

```rust
fn flush(&mut self) -> std::io::Result<()>
```

#### `read_frame`

Read bounded length-prefixed JSON; reject oversized/truncated/malformed frames.

```rust
pub fn read_frame<T: serde::de::DeserializeOwned>(stream: &mut impl Read) -> AppResult<T>
```

#### `write_frame`

Encode bounded length-prefixed JSON and flush it.

```rust
pub fn write_frame<T: Serialize>(stream: &mut impl Write, value: &T) -> AppResult<()>
```

#### `exchange`

Send a request to a localhost TLS endpoint and read its complete response.

```rust
pub fn exchange( address: SocketAddr, config: Arc<rustls::ClientConfig>, request: &Request, ) -> AppResult<Response>
```

#### `exchange_named`

Verify the server name and exchange one request within its remaining deadline.

```rust
pub(super) fn exchange_named( address: SocketAddr, config: Arc<rustls::ClientConfig>, request: &Request, name: &str, ) -> AppResult<Response>
```

### [src/security.rs](src/security.rs)

Request deadlines and sanitized security events. Never accept log text as evidence.

#### `now_ms`

Return Unix milliseconds for the cross-hop request deadline.

```rust
pub fn now_ms() -> u64
```

#### `set_deadline`

Clamp the request budget to ten seconds and store a monotonic worker deadline.

```rust
pub fn set_deadline(ms: u64)
```

#### `clear_deadline`

Clear the worker deadline before recording post-request security events.

```rust
pub fn clear_deadline()
```

#### `expired`

Return whether the current worker has exhausted its request budget.

```rust
pub fn expired() -> bool
```

#### `check_deadline`

Reject expired work before credential issuance or transaction commit.

```rust
pub fn check_deadline() -> AppResult<()>
```

#### `event`

Structured fields only: passwords, OTPs, request bodies and ballots never enter logs.

```rust
pub fn event( conn: &Connection, user: Option<i64>, kind: &str, scope: Option<&str>, attempts: i64, ) -> AppResult<()>
```

#### `voter_metric`

Internal helper: voter metric.

```rust
pub fn voter_metric(conn: &Connection, kind: &str) -> AppResult<()>
```

### [src/services/election.rs](src/services/election.rs)

Election CRUD and lifecycle. Every mutation checks the session in this layer.

#### `owned`

Require election ownership by the administrator, optionally requiring draft status.

```rust
fn owned(conn: &Connection, session: &Session, election: i64, draft: bool) -> AppResult<()>
```

#### `read_allowed`

Check whether this role/session may read the requested election.

```rust
fn read_allowed(conn: &Connection, session: &Session, election: i64) -> AppResult<()>
```

#### `list`

Return records visible to the authenticated role and ownership scope.

```rust
pub fn list(conn: &Connection, session: &Session) -> AppResult<Vec<Election>>
```

#### `audit_list`

Include cancelled elections when an owning administrator reviews history.

```rust
pub fn audit_list(conn: &Connection, session: &Session) -> AppResult<Vec<Election>>
```

#### `election_list`

Internal helper: election list.

```rust
fn election_list( conn: &Connection, session: &Session, include_cancelled: bool, ) -> AppResult<Vec<Election>>
```

#### `get`

Read one authorized election by its ID.

```rust
pub fn get(conn: &Connection, session: &Session, id: i64) -> AppResult<Election>
```

#### `positions`

Read the authorized election positions and their assigned candidates.

```rust
pub fn positions(conn: &Connection, session: &Session, election: i64) -> AppResult<Vec<Position>>
```

#### `create`

Create a draft election and default positions/candidates in one transaction.

```rust
pub fn create(conn: &mut Connection, session: &Session, name: &str) -> AppResult<i64>
```

#### `rename`

Rename an owned draft election and record the change.

```rust
pub fn rename( conn: &mut Connection, session: &Session, election: i64, name: &str, ) -> AppResult<()>
```

#### `delete`

Draft deletion is a cancellation, preserving the audit trail.

```rust
pub fn delete(conn: &mut Connection, session: &Session, election: i64) -> AppResult<()>
```

#### `add_position`

Add a position to an owned draft and record the change.

```rust
pub fn add_position( conn: &mut Connection, session: &Session, election: i64, name: &str, ) -> AppResult<()>
```

#### `position_exists`

Reject a position that does not belong to the specified election.

```rust
fn position_exists(conn: &Connection, election: i64, position: i64) -> AppResult<()>
```

#### `rename_position`

Rename an owned draft position after validating election membership.

```rust
pub fn rename_position( conn: &mut Connection, session: &Session, election: i64, position: i64, name: &str, ) -> AppResult<()>
```

#### `delete_position`

Remove a draft position and its assignments, preserving minimum constraints.

```rust
pub fn delete_position( conn: &mut Connection, session: &Session, election: i64, position: i64, ) -> AppResult<()>
```

#### `delete_orphans`

Remove candidate rows that have no remaining position assignments.

```rust
fn delete_orphans(conn: &Connection, election: i64) -> AppResult<()>
```

#### `add_candidate`

Create and assign a candidate to a draft position transactionally.

```rust
pub fn add_candidate( conn: &mut Connection, session: &Session, election: i64, position: i64, name: &str, party: &str, ) -> AppResult<()>
```

#### `link_candidate`

Assign an existing candidate to another position in the same draft election.

```rust
pub fn link_candidate( conn: &mut Connection, session: &Session, election: i64, position: i64, candidate: i64, ) -> AppResult<()>
```

#### `update_candidate`

Update a draft candidate name/party and audit the change.

```rust
pub fn update_candidate( conn: &mut Connection, session: &Session, election: i64, candidate: i64, name: &str, party: &str, ) -> AppResult<()>
```

#### `unlink_candidate`

Remove a candidate assignment from a draft position.

```rust
pub fn unlink_candidate( conn: &mut Connection, session: &Session, election: i64, position: i64, candidate: i64, ) -> AppResult<()>
```

#### `start`

Require a district official, validate minimum ballot configuration and open voting.

```rust
pub fn start(conn: &mut Connection, session: &Session, election: i64) -> AppResult<()>
```

#### `close`

Require a district official, close the open election and record its timestamp.

```rust
pub fn close(conn: &mut Connection, session: &Session, election: i64) -> AppResult<()>
```

#### `cast`

A whole ballot is submitted once. Editing selections before confirmation is allowed. No identity or credential is written to ballots/ballot_choices.

```rust
pub fn cast( conn: &mut Connection, session: &Session, election: i64, choices: &[(i64, i64)], ) -> AppResult<()>
```

#### `tally`

Require a closed election and district official; persist candidate/party totals.

```rust
pub fn tally(conn: &mut Connection, session: &Session, election: i64) -> AppResult<()>
```

#### `announce`

Require district-official authorization and a tally; mark results announced.

```rust
pub fn announce(conn: &mut Connection, session: &Session, election: i64) -> AppResult<()>
```

#### `results`

Require a district official and return candidate vote totals without voter identities.

```rust
pub fn results(conn: &Connection, session: &Session, election: i64) -> AppResult<Vec<ResultRow>>
```

#### `party_results`

Require a district official and return party totals by position.

```rust
pub fn party_results( conn: &Connection, session: &Session, election: i64, ) -> AppResult<Vec<String>>
```

#### `audit_log`

Return the owning admin’s lifecycle/participation view, excluding ballot choices.

```rust
pub fn audit_log(conn: &Connection, session: &Session, election: i64) -> AppResult<Vec<String>>
```

#### `flag_incident`

Administrators can flag suspicious lifecycle changes without exposing ballots.

```rust
pub fn flag_incident(conn: &Connection, session: &Session, election: i64) -> AppResult<()>
```

### [src/services/mod.rs](src/services/mod.rs)



#### `require_role`

Recheck the session against the active database role; log rejected access.

```rust
pub(crate) fn require_role(conn: &Connection, session: &Session, role: Role) -> AppResult<()>
```

#### `audit`

Insert a structured audit event; database triggers enqueue permitted notifications.

```rust
pub(crate) fn audit( conn: &Connection, election: Option<i64>, actor: Option<i64>, subject: Option<i64>, event: &str, severity: i64, ) -> AppResult<()>
```

#### `finish_change`

Record a successful election change and commit only while its deadline is valid. Taking ownership of the transaction ensures errors roll back both data and audit.

```rust
fn finish_change( tx: Transaction<'_>, session: &Session, election: i64, subject: Option<i64>, event: &str, ) -> AppResult<()>
```

#### `trace_audit`

Emit only allowlisted fields, after the database transaction has committed.

```rust
pub(crate) fn trace_audit(event: &str, actor: Option<i64>, election: Option<i64>, severity: i64)
```

### [src/services/tests.rs](src/services/tests.rs)

Database transactions, constraints, role boundaries, and authentication tests.

#### `initialization_is_idempotent_and_defaults_are_not_created_on_login`

Test: initialization is idempotent and defaults are not created on login.

```rust
fn initialization_is_idempotent_and_defaults_are_not_created_on_login()
```

#### `registration_generates_private_unique_pin_and_enrollment`

Test: registration generates private unique pin and enrollment.

```rust
fn registration_generates_private_unique_pin_and_enrollment()
```

#### `duplicate_and_invalid_registration_leave_no_partial_rows`

Test: duplicate and invalid registration leave no partial rows.

```rust
fn duplicate_and_invalid_registration_leave_no_partial_rows()
```

#### `delivery_failure_rolls_back_registration`

Test: delivery failure rolls back registration.

```rust
fn delivery_failure_rolls_back_registration()
```

#### `role_checks_and_cross_admin_ownership_are_enforced_in_services`

Test: role checks and cross admin ownership are enforced in services.

```rust
fn role_checks_and_cross_admin_ownership_are_enforced_in_services()
```

#### `position_candidate_crud_and_linking_persist`

Test: position candidate crud and linking persist.

```rust
fn position_candidate_crud_and_linking_persist()
```

#### `voter_update_enroll_and_remove_preserve_pin`

Test: voter update enroll and remove preserve pin.

```rust
fn voter_update_enroll_and_remove_preserve_pin()
```

#### `lifecycle_tally_party_totals_and_announcement`

Test: lifecycle tally party totals and announcement.

```rust
fn lifecycle_tally_party_totals_and_announcement()
```

#### `ballots_are_atomic_immutable_and_do_not_link_to_identity`

Test: ballots are atomic immutable and do not link to identity.

```rust
fn ballots_are_atomic_immutable_and_do_not_link_to_identity()
```

#### `open_elections_are_frozen_and_foreign_keys_reject_other_elections`

Test: open elections are frozen and foreign keys reject other elections.

```rust
fn open_elections_are_frozen_and_foreign_keys_reject_other_elections()
```

#### `failed_password_and_otp_attempts_lock_and_log`

Test: failed password and otp attempts lock and log.

```rust
fn failed_password_and_otp_attempts_lock_and_log()
```

#### `otp_completes_once_and_wrong_role_cannot_log_in`

Test: otp completes once and wrong role cannot log in.

```rust
fn otp_completes_once_and_wrong_role_cannot_log_in()
```

#### `reopen_persists_election_edits_and_records`

Test: reopen persists election edits and records.

```rust
fn reopen_persists_election_edits_and_records()
```

#### `legacy_account_schema_is_preserved`

Test: legacy account schema is preserved.

```rust
fn legacy_account_schema_is_preserved()
```

#### `cancelled_elections_disappear_without_deleting_audit`

Test: cancelled elections disappear without deleting audit.

```rust
fn cancelled_elections_disappear_without_deleting_audit()
```

#### `concurrent_duplicate_submissions_commit_only_one_ballot`

Test: concurrent duplicate submissions commit only one ballot.

```rust
fn concurrent_duplicate_submissions_commit_only_one_ballot()
```

#### `pin_attempt_limit_and_deactivated_user_session`

Test: pin attempt limit and deactivated user session.

```rust
fn pin_attempt_limit_and_deactivated_user_session()
```

#### `security_failures_have_counts_and_private_email_notifications`

Test: security failures have counts and private email notifications.

```rust
fn security_failures_have_counts_and_private_email_notifications()
```

#### `expired_mutation_does_not_commit`

Test: expired mutation does not commit.

```rust
fn expired_mutation_does_not_commit()
```

#### `email_delivery_failure_keeps_outbox_for_retry`

Test: email delivery failure keeps outbox for retry.

```rust
fn email_delivery_failure_keeps_outbox_for_retry()
```

#### `audit_failure_rolls_back_election_edit`

Test: audit failure rolls back election edit.

```rust
fn audit_failure_rolls_back_election_edit()
```

#### `inbox_uses_phone_filename_and_rejects_paths`

Test: inbox uses phone filename and rejects paths.

```rust
fn inbox_uses_phone_filename_and_rejects_paths()
```

#### `lifecycle_rejections_and_private_atomic_vote_receipt`

Test: lifecycle rejections and private atomic vote receipt.

```rust
fn lifecycle_rejections_and_private_atomic_vote_receipt()
```

#### `final_results_and_timestamps_survive_repeated_tally_and_sql_tampering`

Test: final results and timestamps survive repeated tally and sql tampering.

```rust
fn final_results_and_timestamps_survive_repeated_tally_and_sql_tampering()
```

#### `tally_audit_failure_rolls_back_totals_and_timestamp`

Test: tally audit failure rolls back totals and timestamp.

```rust
fn tally_audit_failure_rolls_back_totals_and_timestamp()
```

#### `voter_events_are_aggregate_only_and_identity_inserts_are_blocked`

Test: voter events are aggregate only and identity inserts are blocked.

```rust
fn voter_events_are_aggregate_only_and_identity_inserts_are_blocked()
```

#### `receipt_confirmation_is_owned_private_and_audited_for_both_answers`

Test: receipt confirmation is owned private and audited for both answers.

```rust
fn receipt_confirmation_is_owned_private_and_audited_for_both_answers()
```

#### `incident_audits_are_owner_scoped_and_append_only`

Test: incident audits are owner scoped and append only.

```rust
fn incident_audits_are_owner_scoped_and_append_only()
```

### [src/services/voters.rs](src/services/voters.rs)

Registration is one operation: identity, PIN credential, enrollment and audit.

#### `enrollment_allowed`

Require the owning administrator and an enrollable election state.

```rust
pub(crate) fn enrollment_allowed( conn: &Connection, session: &Session, election: i64, ) -> AppResult<()>
```

#### `register`

Register identity, PIN verifier, enrollment and audit transactionally; deliver the PIN privately.

```rust
pub fn register( conn: &mut Connection, auth: &Auth, session: &Session, election: i64, input: &VoterInput, ) -> AppResult<i64>
```

#### `list`

Return records visible to the authenticated role and ownership scope.

```rust
pub fn list(conn: &Connection, session: &Session, election: i64) -> AppResult<Vec<Voter>>
```

#### `enroll`

Enroll an existing voter by NID without regenerating their PIN.

```rust
pub fn enroll(conn: &mut Connection, session: &Session, election: i64, nid: &str) -> AppResult<()>
```

#### `remove`

Removes eligibility, not historical identity or credentials.

```rust
pub fn remove( conn: &mut Connection, session: &Session, election: i64, voter: i64, ) -> AppResult<()>
```

#### `update`

Update allowed voter profile fields; forbid ordinary OTP-phone/NID reassignment.

```rust
pub fn update( conn: &mut Connection, session: &Session, election: i64, voter: i64, input: &VoterInput, ) -> AppResult<()>
```

#### `provision`

Called only from the explicit engineer setup mode, never an ordinary role menu.

```rust
pub fn provision( conn: &mut Connection, auth: &Auth, email: &str, phone: &str, name: &str, password: &str, role: Role, ) -> AppResult<()>
```

#### `pin_receipt`

Record only the administrator's delivery confirmation, never the PIN or voter ID.

```rust
pub fn pin_receipt( conn: &Connection, session: &Session, election: i64, voter: i64, received: bool, ) -> AppResult<()>
```

### [src/test_support.rs](src/test_support.rs)

Isolated fixtures shared by UI and database tests.

#### `new`

Test: new.

```rust
pub(crate) fn new() -> Self
```

#### `create`

Test: create.

```rust
pub(crate) fn create(&mut self) -> i64
```

#### `register`

Test: register.

```rust
pub(crate) fn register(&mut self, e: i64, n: usize) -> (i64, String)
```

#### `drop`

Test: drop.

```rust
fn drop(&mut self)
```

#### `input`

Test: input.

```rust
pub(crate) fn input(n: usize) -> VoterInput
```

#### `message`

Test: message.

```rust
pub(crate) fn message(root: &std::path::Path, phone: &str, prefix: &str) -> String
```

#### `count`

Test: count.

```rust
pub(crate) fn count(conn: &Connection, table: &str) -> i64
```

#### `choices`

Test: choices.

```rust
pub(crate) fn choices(conn: &Connection, s: &Session, e: i64, index: usize) -> Vec<(i64, i64)>
```

### [src/ui/draw.rs](src/ui/draw.rs)

Rendering only: no database or network calls occur here.

#### `draw_list`

Render a titled selectable list.

```rust
fn draw_list( frame: &mut Frame, area: Rect, title: &str, items: Vec<String>, selected: usize, help: &str, )
```

#### `draw`

Render the current screen, notices, forms and confirmations.

```rust
pub(super) fn draw(frame: &mut Frame, app: &mut App)
```

#### `draw_ballot_editor`

Render positions and candidates with the current editor selection.

```rust
fn draw_ballot_editor(frame: &mut Frame, area: Rect, app: &mut App)
```

#### `draw_form`

Render fields with secret values masked.

```rust
fn draw_form(frame: &mut Frame, area: Rect, form: &Form)
```

#### `draw_district`

Render election state and district-official action hints.

```rust
fn draw_district(frame: &mut Frame, area: Rect, app: &App)
```

### [src/ui/events.rs](src/ui/events.rs)

Keyboard actions for the role menus and election screens.

#### `handle_key`

Route a key to the active form/screen and display operation errors.

```rust
pub(super) fn handle_key(&mut self, key: KeyEvent)
```

#### `handle_screen`

Route screen-level selection, login and navigation actions.

```rust
pub(super) fn handle_screen(&mut self, key: KeyEvent) -> AppResult<()>
```

#### `handle_admin`

Handle election-editor navigation and position/candidate actions.

```rust
pub(super) fn handle_admin(&mut self, key: KeyEvent) -> AppResult<()>
```

#### `voter_form`

Open a voter registration or edit form with the appropriate field values.

```rust
pub(super) fn voter_form(&mut self, voter: Option<Voter>)
```

#### `handle_voters`

Handle registration, enrollment, editing and removal selection.

```rust
pub(super) fn handle_voters(&mut self, key: KeyEvent) -> AppResult<()>
```

#### `handle_district`

Handle election lifecycle commands available to district officials.

```rust
pub(super) fn handle_district(&mut self, key: KeyEvent) -> AppResult<()>
```

#### `show_results`

Fetch and display anonymous candidate and party totals.

```rust
pub(super) fn show_results(&mut self) -> AppResult<()>
```

#### `handle_vote`

Update local ballot selections and request final submission confirmation.

```rust
pub(super) fn handle_vote(&mut self, key: KeyEvent) -> AppResult<()>
```

### [src/ui/forms.rs](src/ui/forms.rs)

Form input and confirmed mutations. State changes follow successful server writes.

#### `handle_form`

Handle input/navigation/submission; preserve invalid forms and clear secret fields.

```rust
pub(super) fn handle_form(&mut self, key: KeyEvent) -> AppResult<()>
```

#### `submit_form`

Validate client fields and call the appropriate typed backend operation.

```rust
pub(super) fn submit_form(&mut self, form: &Form) -> AppResult<()>
```

#### `handle_confirmation`

Execute a confirmed operation and refresh the affected UI state.

```rust
pub(super) fn handle_confirmation(&mut self, key: KeyEvent) -> AppResult<()>
```

### [src/ui/mod.rs](src/ui/mod.rs)

UI state and navigation helpers; persistence is accessed through the TLS client.

#### `signup`

Staff registration is an explicit local setup capability, never public signup.

```rust
pub(crate) fn signup(conn: ApiClient) -> AppResult<Self>
```

#### `remote`

Initialize TUI state around an API client without a direct database connection.

```rust
pub(crate) fn remote(conn: ApiClient, setup_mode: bool) -> Self
```

#### `new`

Internal helper: new.

```rust
fn new(conn: rusqlite::Connection, auth: auth::login::Auth, setup_mode: bool) -> Self
```

#### `session`

Require an authenticated session before protected UI actions.

```rust
fn session(&self) -> AppResult<&Session>
```

#### `election_id`

Require a selected election and return its database ID.

```rust
fn election_id(&self) -> AppResult<i64>
```

#### `info`

Set the user-visible notice.

```rust
fn info(&mut self, message: impl Into<String>)
```

#### `form`

Build editable fields and open the requested form.

```rust
fn form( &mut self, purpose: FormPurpose, title: &str, fields: Vec<(&'static str, String, bool)>, )
```

#### `confirm`

Ask for confirmation before a destructive or final action.

```rust
fn confirm(&mut self, message: &str, action: Action)
```

#### `logout`

Clear local session, pending authentication and role-specific screen data.

```rust
fn logout(&mut self)
```

#### `reload_election`

Reload selected election configuration from the backend after changes.

```rust
fn reload_election(&mut self) -> AppResult<()>
```

#### `load_elections`

Fetch allowed elections and open the appropriate selection screen.

```rust
fn load_elections(&mut self, purpose: SelectionPurpose) -> AppResult<()>
```

#### `select_election`

Load the chosen election for editing, administration or voting.

```rust
fn select_election(&mut self) -> AppResult<()>
```

#### `load_voters`

Fetch the authorized enrollment/participation view.

```rust
fn load_voters(&mut self) -> AppResult<()>
```

#### `load_audit`

Fetch the authorized election audit view.

```rust
fn load_audit(&mut self) -> AppResult<()>
```

#### `rows`

Flatten positions and candidates into editor rows.

```rust
fn rows(&self) -> Vec<(usize, Option<usize>)>
```

#### `current`

Resolve the selected editor row to position and optional candidate indices.

```rust
fn current(&self) -> Option<(usize, Option<usize>)>
```

#### `ballot_rows`

Build the row-to-position/candidate index mapping used by the editor.

```rust
fn ballot_rows(positions: &[Position]) -> Vec<(usize, Option<usize>)>
```

#### `run`

Drive the terminal draw/input loop and restore terminal state on exit.

```rust
pub(crate) fn run(mut terminal: DefaultTerminal, mut app: App) -> io::Result<()>
```

### [src/ui/tests.rs](src/ui/tests.rs)

UI events and rendering tested against live TLS endpoints.

#### `key`

Test: key.

```rust
fn key(code: KeyCode) -> KeyEvent
```

#### `fill`

Test: fill.

```rust
fn fill(app: &mut App, values: &[&str])
```

#### `render`

Test: render.

```rust
fn render(app: &mut App, w: u16, h: u16) -> String
```

#### `ui_login_goes_to_menu_then_create_and_edit_load_database`

Test: ui login goes to menu then create and edit load database.

```rust
fn ui_login_goes_to_menu_then_create_and_edit_load_database()
```

#### `ui_r_registers_voter_n_does_nothing_and_masks_password`

Test: ui r registers voter n does nothing and masks password.

```rust
fn ui_r_registers_voter_n_does_nothing_and_masks_password()
```

#### `ui_voter_casts_once_and_district_announces_tie`

Test: ui voter casts once and district announces tie.

```rust
fn ui_voter_casts_once_and_district_announces_tie()
```

#### `ui_failed_write_does_not_create_a_local_placeholder`

Test: ui failed write does not create a local placeholder.

```rust
fn ui_failed_write_does_not_create_a_local_placeholder()
```

#### `ui_empty_results_and_role_switch_do_not_leak_previous_session`

Test: ui empty results and role switch do not leak previous session.

```rust
fn ui_empty_results_and_role_switch_do_not_leak_previous_session()
```

#### `staff_signup_creates_both_roles_without_granting_a_login_session`

Test: staff signup creates both roles without granting a login session.

```rust
fn staff_signup_creates_both_roles_without_granting_a_login_session()
```

#### `invalid_signup_stays_in_form_without_database_write`

Test: invalid signup stays in form without database write.

```rust
fn invalid_signup_stays_in_form_without_database_write()
```

### [src/validation.rs](src/validation.rs)

Shared format checks: the TUI gives quick feedback; the server checks again.

#### `text`

Trim a required field and reject empty, oversized or control-character input.

```rust
pub fn text(value: &str, label: &str) -> AppResult<String>
```

#### `email`

Validate and normalize an email; optionally allow an empty voter email.

```rust
pub fn email(value: &str, optional: bool) -> AppResult<String>
```

#### `phone`

Require 7–15 ASCII digits, with at most one leading country-code plus.

```rust
pub fn phone(value: &str) -> AppResult<String>
```

#### `digits`

Reject codes with an incorrect length or non-ASCII digits.

```rust
pub fn digits(value: &str, length: usize, label: &str) -> AppResult<()>
```

#### `password`

Check password size/control characters; apply minimum length at signup.

```rust
pub fn password(value: &str, signup: bool) -> AppResult<()>
```

#### `voter_fields`

Validate identity fields and a real, non-future birth date; return normalized data.

```rust
pub fn voter_fields(input: &VoterInput) -> AppResult<VoterInput>
```

#### `voter`

Require an adult after validating the supplied identity fields.

```rust
pub fn voter(input: &VoterInput) -> AppResult<VoterInput>
```

#### `adult_on`

Calendar-age check, shared by enrollment and the final voting transaction.

```rust
pub fn adult_on(dob: NaiveDate, today: NaiveDate) -> AppResult<()>
```

#### `eighteenth_birthday_boundary_and_leap_day`

Internal helper: eighteenth birthday boundary and leap day.

```rust
fn eighteenth_birthday_boundary_and_leap_day()
```

#### `rejects_malformed_and_oversized_fields`

Internal helper: rejects malformed and oversized fields.

```rust
fn rejects_malformed_and_oversized_fields()
```

#### `rejects_impossible_future_and_out_of_range_birth_dates`

Internal helper: rejects impossible future and out of range birth dates.

```rust
fn rejects_impossible_future_and_out_of_range_birth_dates()
```

### [tests/terminal_smoke.py](tests/terminal_smoke.py)

Python support tool; requires only the standard library.

- `wait_for(check, description, timeout)`: wait for.

- `main()`: main.

- `__init__(self, args, env, cwd)`:   init  .

- `drain(self, seconds)`: drain.

- `keys(self, value)`: keys.

- `wait_text(self, text)`: wait text.

- `screen(self)`: Read the visible grid, accounting for Ratatui's changed-cell output.

This handles the cursor/erase commands used in our fixed 120x36 test;
it is not a general terminal emulator. Color and cursor modes are ignored.

- `close(self)`: close.

- `query(sql)`: query.

- `inbox(phone, prefix)`: inbox.

- `login(role)`: login.

- `ready()`: ready.

- `trace_events()`: trace events.

### [scripts/generate_docs.py](scripts/generate_docs.py)

Python support tool; requires only the standard library.
