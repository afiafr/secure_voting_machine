# Implementation notes

## Request and data flow

1. The Ratatui client collects input and calls a typed function in `network/client`.
2. That function sends a bounded, length-prefixed JSON RPC request over TLS 1.3 to
   the appropriate Administrator, District Official or Voter gateway.
3. The gateway forwards the request to the database service over a new mutual-TLS
   connection using its own client certificate.
4. The database service validates the certificate against its CA and its exact
   gateway certificate allowlist. The certificate's role must match the request.
5. The service resolves the opaque session token. Role and identity come from the
   server's session record and current database account, not client-provided IDs.
6. The domain service validates authorization, ownership, state and input, then
   performs a SQLite transaction. It returns a result; the UI refreshes after success.

These are JSON RPC endpoints, not HTTP endpoints. Neither arbitrary SQL nor arbitrary
file paths are accepted. TLS uses the certificate for authentication and negotiates
symmetric traffic keys; requests are not individually encrypted with a public key.
SQLite is embedded: only its service opens it, so encryption covers the network hops,
not SQLite's local file operations. In-process helpers call each other normally.

Each operation opens a TLS connection. The design prioritizes a readable classroom
implementation over pooling or high throughput. The server serializes access to its
SQLite connection. Database transactions still protect competing independent
connections, demonstrated by the concurrent ballot test.

## Authentication

Administrator and official passwords use bcrypt with per-password salts plus an
environment pepper. Voter PINs are randomly generated 20-digit values, stored as a
keyed lookup digest and a bcrypt verifier. The predictable SHA3(user_id) PIN scheme
from the prototype has been removed.

A correct password/PIN produces only a five-minute challenge. OTP verification is
required before an opaque 30-minute session token is issued. Both are randomly
identified, server-held and bound to a role gateway. OTPs allow three unsuccessful
attempts, and success consumes the challenge. Logout revokes the session. Restarting
the server discards pending challenges and sessions. Current account role/active
status is rechecked on protected calls.

Failed passwords are limited by a keyed account identifier (five failures); failed
PINs are limited for the shared terminal service (ten failures). OTP failures are
limited per user (three). Cooldowns last five minutes and are stored in SQLite.
Successful first-factor login cannot erase the OTP failure counter.

## Database model

| Table | Purpose / keys |
| --- | --- |
| `users` | Identity, role, DOB, NID, phone and active flag; primary key `id` |
| `user_credentials` | `user_id` is both primary key and foreign key to `users.id` |
| `voter_pin_credentials` | Private PIN lookup/verifier; primary/foreign key `user_id` |
| `elections` | Creator, lifecycle status, actor IDs and timestamps |
| `election_positions` | Ordered named positions; foreign key to election |
| `candidates` | Candidate identity and political party within an election |
| `position_candidates` | Many-to-many assignment, constrained to the same election |
| `election_voters` | Election eligibility and irreversible participation flag |
| `ballots` | Random ballot ID and election; no voter ID, PIN or cast timestamp |
| `ballot_choices` | One choice per ballot/position; candidate assignment foreign key |
| `candidate_results` | Stored counts per election/position/candidate |
| `party_results` | Stored counts per election/position/party |
| `audit_events` | Append-only application history with actor/subject, time and severity |
| `auth_attempts` | Persisted failure counts and cooldowns |
| `error_reports` | UI failure context without submitted fields |
| `notification_outbox` | Committed audit notification delivery queue |

Composite foreign keys reject candidates or positions from another election.
Foreign keys are enabled for every connection opened by the application. Triggers
freeze the ballot structure after opening, enforce lifecycle transitions, protect
immutable ballots/choices and prevent resetting participation. Service transactions
also enforce owner/role rules and minimum position/candidate counts.

Default candidates are two shared identities assigned to all three default positions.
New positions receive two default candidates. A candidate name/party edit updates the
identity, so it affects every position linked to that candidate. Unlinking a candidate
removes only that assignment, deleting the candidate identity if it becomes orphaned.

## CRUD and irreversible operations

| Entity | Create/read/update/delete behavior |
| --- | --- |
| Election | Admin creates, lists and renames owned drafts; deleting cancels the draft and preserves history |
| Position | Admin adds/lists/renames/deletes within owned drafts; minimum three preserved |
| Candidate | Admin adds/links/reads/edits/unlinks; minimum two per position preserved |
| Voter enrollment | Admin registers/enrolls/reads/updates allowed identity fields/removes draft enrollment |
| Accounts | Engineer provisions admin/official through `--signup`; no ordinary privilege-granting UI |
| Ballot | Voter creates once; no edit/delete; official gets aggregate results |
| Audit | Appends/reads; no application edit/delete |

“CRUD covered” does not mean allowing ballot rewrites or deleting election history.
Voter NID and phone are not editable from the ordinary registration form, to avoid
redirecting another person's authentication messages. Account deletion, phone changes
and credential reset are deliberately not ordinary admin features.

## Requirements alignment and changes from the draft

- Role welcome screen, admin menu, existing election loading, all candidate/position
  edits, R registration, PIN delivery/login, OTP, lifecycle actions and DB-backed
  ballots are implemented and tested.
- Audit data records participation only; it does not record a voter's selections.
  This follows the user's later privacy requirement where the PDF suggested broader logs.
- Software-only delivery is a private local inbox instead of Twilio, serial-port
  emulation or a paid service. It has an explicit simulation boundary.
- Authorization and state checks stop unauthorized changes; notifications supplement
  these checks rather than serving as the access-control mechanism.
- Wall-clock timestamps use SQLite UTC date/time values. Rust `Instant` is used for
  durations/deadlines, not persisted event timestamps.
- The voting submission is a complete ballot transaction, not independently saved
  partial positions. A voter can change their on-screen selection until confirmation,
  then cannot change any recorded choice.
- A simulated incident notification outbox is implemented; external email, national
  identity lookup, an external monitoring service and independent tamper-proof logs
  are not implemented. The document's proposed integrations do not come with provider
  credentials, and their guarantees are not claimed here.

## Migration

New runs use `voting.db`. Initialization is idempotent and non-destructively adds
DOB, NID and active columns to the prototype `users` table. Legacy accounts and
credential hashes remain intact. Their original pepper is required for verification.
Existing staff also need usable registered phone numbers for OTP.

The legacy `voter_pins` table is retained, but never read for login because its
predictable values are unsafe and cannot be converted into secret PINs. Re-register
or explicitly migrate those voters through a separately reviewed procedure before
using their accounts. Arbitrary other legacy election schemas are not automatically
converted; use a fresh database or supply a schema-specific migration. Back up an
existing database before opting into a migration.
