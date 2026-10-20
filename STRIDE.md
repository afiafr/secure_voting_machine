# STRIDE: receipts for the threat model 🔐

Version 0.2.5. This is the implementation's security contract, not a claim that a
classroom program is a production election system. All six STRIDE categories apply.
Read this together with `docs/TESTING.md` and the generated function reference.

## People, assets and trust boundaries

Legitimate stakeholders: election administrators (configure elections/register voters),
district officials (open/close/tally/announce), voters, candidates, system engineers,
and maintainers. Adversaries include unauthenticated local clients, malicious voters,
malicious privileged users, competing candidates, and compromised gateway processes.

Assets: credentials and OTP challenges, session tokens, private certificate keys,
identity/eligibility records, ballot integrity and secrecy, election configuration,
tallies, audit evidence, notification outboxes, and service availability. The CLI
executable alone is not the complete asset list.

The TUI is untrusted. Three role gateways have distinct client certificates and keys.
A private CA signs their certificates. The database service requires mTLS, identifies
the exact gateway certificate, and separately checks the user's authenticated session,
current database role, active status and election ownership. Gateway certificates
identify services, not individual people. No endpoint accepts arbitrary SQL.

The backend OS account, filesystem, startup configuration and code are trusted.
Default demo mode puts all listeners in one process. Separate `--serve`/`--client`
processes demonstrate transport boundaries, not host isolation. An OS owner can read
inboxes or replace code/SQLite; POSIX permissions cannot protect against that owner.

## Mechanism-to-test map

| STRIDE | Attack | Implemented mechanism and location | Evidence / remaining limit |
| --- | --- | --- | --- |
| Spoofing | Guess staff password | `auth/login.rs`: bcrypt + pepper; five consecutive failures cause a five-minute identity cooldown; structured attempt/failure/lockout events | `failed_password_and_otp_attempts_lock_and_log`, `security_failures_have_counts_and_private_email_notifications`; targeted lockout remains possible |
| Spoofing | Guess voter PIN | Random 20-digit PIN, keyed lookup and bcrypt verifier; ten failures against the same lookup cause cooldown | `pin_attempt_limit_and_deactivated_user_session`; unknown guesses changing PIN need transport admission limits too |
| Spoofing | Guess/replay OTP | Six-digit random challenge, five-minute expiry, HMAC verification, three failures invalidate it and cause cooldown; opaque pending token | `otp_completes_once_and_wrong_role_cannot_log_in`, network OTP expiry/replay tests; inbox is virtual, not independent MFA |
| Spoofing | Impersonate gateway/server | TLS 1.3 CA/name validation, distinct mTLS gateway certificates with exact certificate-to-role binding | Negative TLS tests cover untrusted CA, wrong name, missing certificate, plaintext, wrong gateway |
| Tampering | Alter request on wire | rustls protects transport integrity on both hops; no plaintext fallback | TLS integration tests; endpoints see decrypted requests and must remain trusted |
| Tampering | Change submitted vote or vote twice | Whole-ballot transaction, enrollment update conditional on `has_voted=0`, primary/foreign keys, immutable ballot/choice triggers | Ballot immutability, rollback and two-connection race tests; database OS owner can remove triggers |
| Tampering | Modify live configuration | Owner checks and draft-only database freeze triggers; at least three positions/two candidates before opening | CRUD/lifecycle tests and schema constraints |
| Tampering | SQL injection / malformed fields | Bound SQL parameters; shared field validation; typed endpoint payloads and unknown-field rejection | Validation tests, direct malformed endpoint requests, UI invalid signup test; never interpolate user input into SQL |
| Repudiation | Deny lifecycle action | Timestamped actor/subject audit records; audit tables reject UPDATE/DELETE; notifications reference event IDs | Lifecycle/service tests; not cryptographically signed or independently witnessed |
| Repudiation | Hide failed login | `security_events` records attempt/failure/lockout type, associated user when identifiable, keyed scope and retry count; audit events accompany it | Security failure tests; malformed requests use generic `request_rejected`; blocked password scopes are pseudonymous |
| Repudiation | Lose notification on file error | Durable `email_outbox`; delivered flag set after append/sync; retry on later requests | Email delivery/retry test; crash may duplicate same Event ID, and complete host failure cannot be self-reported |
| Information disclosure | Admin reads choices | Services require DO for tally/results; no admin raw-ballot endpoint; admin audit/registration responses omit ballot IDs and choices | Direct cross-role endpoint tests, full election privacy test |
| Information disclosure | DO learns who voted | No DO voter-list/audit endpoint; DO notifications contain lifecycle/configuration only; no participation/authentication feed | Cross-role tests and notification-recipient tests |
| Information disclosure | Logs reveal credentials | Fixed event types, hashed scopes; client error strings are discarded; email messages exclude request bodies, PINs, OTPs, sessions and choices | Secret-exclusion and forged-log test; engineer's private simulated OTP inbox intentionally contains delivery secrets |
| Information disclosure | Access files | New database and private files use POSIX 0600, inbox/certificate directories 0700; keys excluded from ZIP | Local file permissions; set equivalent ACLs on Windows; SQLite is not encrypted at rest |
| Denial of service | Slow or stalled request | Ten-second end-to-end request budget, clamped at gateways; socket watchdog, SQLite progress interruption, pre-commit checks; timeout event | Deadline/client/watchdog/expired-write tests; bcrypt cannot be preempted and a completed commit cannot be undone by a lost response |
| Denial of service | Connection/handshake flood | 1 MiB frames; 32 active workers and 64 admissions/sec/listener; rejected transports aggregated; >10 rejections produce level-2 alert | Frame/TLS/flood tests; not volumetric protection or per-IP banning |
| Denial of service | Fill retry table | Maximum 4096 distinct retry scopes; expired blocked records cleaned up; no shared voter PIN lockout | Scoped PIN test; logs still need operator retention and storage monitoring |
| Elevation of privilege | Change claimed role/user ID | Server-held session after both factors; role/active checks per request; gateway certificate role must agree; ownership checks | Forged token, role-switch, wrong gateway and cross-owner tests |
| Elevation of privilege | Create privileged staff | Local engineer setup capability enabled only after a separate engineer OTP; ordinary client/serve cannot provision staff | Real TUI setup test, setup rejection test; engineer inbox relies on trusted OS account |

## Access matrix (not just hidden buttons)

| Resource | Admin | DO | Voter |
| --- | --- | --- | --- |
| Election configuration | Own drafts: create/edit | Read; lifecycle operations | Read own open enrolled election |
| Voter identity / participation | Own elections | Denied | Used internally for own authorization |
| Ballot choices | Denied | Backend tally reads; returns anonymous aggregates | Submit own complete ballot once, no general read |
| Audit lifecycle / voter registration | Own election, own account security events | Denied audit listing | Denied |
| Credentials / PIN hashes / OTP hashes | No endpoint | No endpoint | No endpoint |

Admin and DO identities are not linked through ballot IDs. Ballots contain no user ID,
PIN, session token or timestamp. This removes an explicit join; it does not defeat
transaction ordering, privileged filesystem observation, collusion, or one-voter
inference. Do not describe this as a cryptographic anonymous-voting protocol.

## Timeout and error semantics

Ten seconds is the latest user requirement (the PDF gave 20,000ms as an example).
One request carries a deadline through the two TLS hops. Servers cap a client-provided
future deadline; clients cannot extend it indefinitely. The database checks after
waiting for its mutex, interrupts long SQLite execution, and checks before commits.
The watchdog closes the socket even if hashing or filesystem I/O is still running.

A successful commit followed by a lost response has an **unknown outcome to the
client**. Refresh before retrying; mutations are never automatically retried. Timer
scheduling is not hard real-time. Partial filesystem deliveries can outlive a rolled
back database operation. No claim of universal cancellation is made.

`request_timeout`, authentication errors and rejected requests are recorded without
raw exception/body text. Transport incidents are counted in aggregates to avoid one
SQL insert per malformed network packet. Unattributed incidents go to the system
administrator, not every election administrator. Timed-out sessions with an identifiable
actor also generate an account-scoped event when the DB worker can resume.

## Notifications and privacy

`virtual_sms/emails/systemadmin@gmail.com.txt` is the default system inbox. Change
`VOTING_SYSTEM_EMAIL` in `.env` before startup. Admin inbox filenames use their validated
registered email, under the same `emails/` directory. User-supplied paths are rejected.
The system inbox gets errors. An admin gets own-account/owned-election events, including
voter security incidents associated with their enrollment. DO gets only lifecycle/
configuration SMS-style notifications. No notification includes ballot selections.

SQLite audit history is authoritative. Outbox delivery is at-least-once: duplicates
may occur after a crash, but retain the same Event ID. Email and SMS are simulated
text files. No network email/SMS provider is contacted, and logs/inboxes are not
independently tamper-proof against the backend OS account.

## Differences from the original Implementation Details PDF

- Latest privacy instructions override the PDF's broader suggestion to log party votes
  for administrators. Participation and choices are separated.
- bcrypt/rustls/random OTP and local inboxes replace speculative crate/provider names.
  A PIN is a secret credential, never `SHA3(user_id)` or a public ballot identifier.
- No national-ID provider was supplied. NID uniqueness is enforced locally; identity
  authenticity and phone ownership are not verified against a real national database.
  A separate read-only SQLite/mTLS demo registry now checks ID + name + DOB + phone; age
  must be 18+. Test records live in demo/voter_ids.txt.
- OTP is software-only as requested. Do not call same-account text inboxes strong MFA.
- TLS wraps actual network hops, not ordinary Rust function calls. HTTP status 500 does
  not apply to this custom length-prefixed JSON protocol; errors use `Response.error`.
- No per-IP ban is implemented: all demo clients share loopback, so such a ban would
  lock everyone out. Per-identity cooldowns and bounded listener admission are used.
- No external health monitor, independently signed log archive, disk encryption,
  real SMS/email provider or crash-proof cross-filesystem transaction is claimed.

## Incident levels

Level 3: rejected/suspected attempt (authentication, authorization, malformed request).
Level 2: repeated TLS/protocol flood alert or manually flagged suspicious change.
Level 1: confirmed service outage; an external monitor would be required to reliably
report a dead process. A normal timeout is not automatically proof of an attacker or
of total service outage. System and identity lockouts must not disable all voters.

## v0.2.5 simplification check

The role-specific gateway certificates, authenticated sessions, ownership checks,
OTP/retry limits, ten-second deadlines, fixed audit events, privacy filters and
immutable ballot constraints are unchanged. The shared `finish_change` function
keeps mutation and audit in one transaction and still checks the deadline before
commit. `audit_failure_rolls_back_election_edit` verifies that a logging failure
cannot leave an unaudited edit. No new endpoint or raw table access was added.
See `docs/USE_CASES.md` for the supplied Section 5.3 screenshot mapping.

Virtual SMS inbox filenames now use validated phone numbers (7–15 ASCII digits,
optional leading +). Only the fixed `engineer` alias is also accepted; path
separators and traversal strings are rejected. Inbox directory/file permissions
remain restricted. Phone numbers are now visible in filenames to the OS account.

## Eligibility, receipts and audit privacy

The registry simulator uses its own CA and exact backend client-certificate check.
Untrusted TLS peers cannot query it. Unknown ID, changed DOB/name or an unavailable
endpoint reject enrollment/submission. Registry rows are read-only after startup.
This prevents arbitrary identities within the demo dataset; it does not establish
real-world identity, phone ownership, citizenship or government registration.

Shared DOB validation plus a transaction-side age check reject under-18 voters.
Choices stay local until Submit. The atomic enrollment update blocks a second
ballot even with simultaneous requests. The same transaction queues a plain thank-you
text. No vote success/failure event is appended to the election audit. Authentication
and generic security failures still have structured logs. Admins can see the internal
participation flag through the voter list, and DOs cannot. A trusted database owner
can observe timing/outbox data; this is not cryptographic unlinkability.

Tests: `eighteenth_birthday_boundary_and_leap_day`,
`registry_rejects_unknown_mismatched_and_underage_without_writes`,
`registry_tls_fails_closed_and_requires_client_certificate`,
`lifecycle_rejections_and_private_atomic_vote_receipt`, plus the terminal walkthrough.

## Integration attack checks (v0.2.5)

| STRIDE category | Local attack/check | Expected defense |
| --- | --- | --- |
| Spoofing | Forged/replayed sessions and OTP; wrong-role gateway; missing/untrusted TLS certificate; unknown registry ID | Reject credentials/session/transport; staff/system audit or anonymous voter counters |
| Tampering | SQL-shaped ID; redirected phone; false adult DOB; repeat tally; direct result UPDATE/DELETE/REPLACE | Bound queries and canonical registry match; no partial registration; immutable results and timestamps |
| Repudiation | Roll back when an audit insert fails; retry tally/announcement; both PIN receipt answers | Successful changes and audit commit together; warnings persist for finality rejection |
| Information disclosure | Voter login/failure/error reports; inspect JSON traces, audit and notifications | No voter identity, PIN, OTP or ballot choices in individual audit/tracing; daily failure totals only |
| Denial of service | Password/OTP/PIN limits, oversized/truncated frames, stalled requests | Bounded retries/framing/workers; ten-second deadline; transport aggregation |
| Elevation of privilege | Call admin/DO endpoints directly as a voter; edit another admin's election | Backend role and ownership checks, independent of menus |

These are automated, isolated tests using synthetic data, not a production penetration
test or proof that all vulnerabilities are absent. A database/file owner can remove
triggers, inspect local inboxes, or change the registry; OS administrators remain trusted.
No general recount endpoint is provided. Existing historical logs are not scrubbed.
