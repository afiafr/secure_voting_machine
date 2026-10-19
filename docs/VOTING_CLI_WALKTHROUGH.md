# Voting CLI walkthrough — wat do? (what to do) 🗳️

Hey! If you're visiting from another project group, this is the guide for trying
our voting system without knowing the code first. We'll create staff accounts,
register a voter, run an election, and look at the security checks along the way.

This guide describes the v0.2.2 application with the corrected `Cargo.lock`.
The interface runs inside Terminal. There is no website, physical voting machine,
real SMS provider, or real email server involved.

**About security:** we explain which attacks the application rejects and why.
We do not claim that nobody can attack it. The final sections explain the trusted
parts of the demo and what an adversary with stronger access could still do.

## Quick map: who does what?

| Role | Their job | Login |
| --- | --- | --- |
| Election Administrator | Create/edit their elections, register/enroll voters, view their audit logs | Email + password, then OTP |
| District Official (DO) | Open/close elections, tally votes, announce/display winners | Email + password, then OTP |
| Voter | Cast one final ballot in an eligible open election | Assigned PIN, then OTP |
| Engineer/operator | Approve staff signup and run the local installation | Local setup access plus engineer OTP |

The engineer is a setup responsibility, not a fourth voting role in the main menu.
An admin cannot read vote choices. A DO cannot read the voter list or participation
history through the application's endpoints.

## 1. Get the project open

1. Unzip the project and find the folder containing `Cargo.toml`, `src`, and `README.md`.
2. Open Terminal. On macOS, Spotlight → Terminal → Enter works.
3. Type `cd `, drag that project folder into Terminal, then press Enter.
4. Run `ls`. If you cannot see `Cargo.toml`, you're in the wrong folder.
5. Check your tools:

```sh
cargo --version
rustc --version
python3 --version
```

If Rust/Cargo is missing, use the official Rust installer linked in the project's
README, reopen Terminal, and return to this folder. You also need a C compiler:
macOS users can install Apple's tools with `xcode-select --install`; Linux setup
commands are in the README. Commands in this guide assume macOS/Linux or WSL.

Build the program:

```sh
cargo build --locked
```

Cargo downloads libraries and creates the runnable program. Wait for the build to
finish successfully. The executable will be `target/debug/secure_voting_machine`.

Use the **corrected Cargo.lock** supplied with the lockfile fix. If an older copy
reports “unexpected end of input while parsing minor version number”, replace that
file: an old uploaded copy had `chrono` version `0.4` where an exact version was
required. Don't manually shorten versions in Cargo.lock. Dependency ranges belong
in Cargo.toml; Cargo generates the lockfile.

All remaining commands should be run from this same project folder. Paths such as
`virtual_sms/` are relative to the directory where you launch the app.

## 2. Set up the private secret once

The app needs `APP_PEPPER`: a private value used when protecting credentials.
For a new installation, this creates a random secret in `.env`:

```sh
umask 077
python3 -c 'import secrets; open(".env", "x").write("APP_PEPPER=" + secrets.token_hex(16) + "\n")'
```

If `.env` already exists, the command refuses to overwrite it. Keep the existing
file if you're using an existing database. Changing its pepper breaks verification
of existing credentials. Do not send `.env` to the group chat or commit it to Git.

Our database is created automatically when the backend starts. You don't need to
create the tables manually or install a separate SQLite server.

## 3. Sign up an Administrator

In Terminal A:

```sh
cargo run --locked -- --signup
```

The extra `--` separates Cargo's options from our app's options. The app first asks
for engineer approval. Leave it waiting and open **Terminal B**, in the same folder.
Read the engineer's simulated text inbox:

```sh
cat virtual_sms/engineer.txt
```

You can also use:

```sh
./target/debug/secure_voting_machine --inbox engineer
```

Find the newest line beginning `Setup OTP:`. Enter that six-digit code in Terminal A.
The signup form should now open.

Use Tab/Shift+Tab to move between fields. Fill in email, password, name and phone.
At Role, use Left/Right or Space to choose **Administrator**, then press Enter.

Example identity values for a disposable demo:

| Field | Example / rule |
| --- | --- |
| Email | `admin@example.test` |
| Full name | `Demo Admin` |
| Phone | `+15550000001` |
| Password | Choose your own; at least 10 bytes. Password + pepper must fit bcrypt's 72-byte limit |
| Role | Administrator; stored internally as `admin` |

Phone numbers require 7–15 ASCII digits and may start with `+`. Don't include spaces
or hyphens. Example addresses and numbers here are fictitious; no messages are sent
outside your computer.

Signup creates an account; it does not grant a logged-in session. Ctrl+C exits.

## 4. Sign up a District Official

Run `cargo run --locked -- --signup` again. Retrieve the **new** engineer Setup OTP,
then create the DO account. For example, use `do@example.test` and `+15550000002`.
At Role, select **District Official**, stored as `district_official`.

The engineer approval is required each time you start setup mode. There is no public
“make me admin” action in the normal client. Voters will be registered through the
admin interface later; do not try to create a voter with staff signup.

## 5. Log in — and find the text-message OTP 📱

Start the normal app:

```sh
cargo run --locked
```

Choose Administrator with Up/Down and Enter. Enter the registered email and password.
After successful credential verification, the app asks for an OTP.

**Your real phone will not buzz.** The text is represented by a local `.txt` file,
named after the phone number you registered. For our example admin:

```text
virtual_sms/+15550000001.txt
```

Leave Terminal A at the OTP screen. In Terminal B:

```sh
cat 'virtual_sms/+15550000001.txt'
```

Or:

```sh
./target/debug/secure_voting_machine --inbox +15550000001
```

Look for the latest `Login OTP:` line and enter its six-digit code in Terminal A.
The file contains previous messages too, so use the newest applicable code.

### Which code is which?

| Text prefix | What it means | Where it goes |
| --- | --- | --- |
| `Setup OTP:` | Engineer approval to enter staff signup | Setup prompt before the form |
| `Login OTP:` | Temporary second login step; expires after five minutes and works once | OTP form after password/PIN verification |
| `Voting PIN:` | Twenty-digit voter credential, issued at registration | Voter login PIN field, before their login OTP |

An OTP and a PIN are different. A new OTP is issued during login. The voter keeps
the same PIN across logins unless a separate supported credential-management process
changes it; registering/enrolling them in another election doesn't create a new PIN.

If you registered without a leading `+`, the filename also has no `+`. For example,
`01712345678` uses `virtual_sms/01712345678.txt`. If `VOTING_SMS_DIR` is set in `.env`,
use that directory instead. In separate-server mode, files are written on the server.
Old hash-named inbox files are not automatically renamed; request a new login OTP.

## 6. Admin: create your first election

After login, **wat do? (what to do)** offers:

1. Create new election.
2. Edit existing election.
3. Register a voter.
4. View audit log.

Choose Create new election, enter a name such as `Snack Council`, then press Enter.
The draft is saved with President, Vice President and Secretary. Each has two
candidates: Hamberger Red / Red Party and Blue Sandwich / Blue Party.

The defaults are editable. Candidate identities can be shared between positions,
so editing a linked candidate changes that candidate in all linked positions.

| Editor key | What it does |
| --- | --- |
| Arrows | Select a position/candidate row and a Position/Candidate/Party column |
| P | Add a position with two default candidates |
| C | Add a candidate to the selected position |
| E | Edit selected position or candidate name/party |
| D | Delete selected draft position or remove candidate assignment, after confirmation |
| L | Link an existing candidate ID from this election to this position |
| F2 | Rename the election |
| X | Cancel a draft election while keeping its audit history |
| R | Open the voter screen |
| Esc | Return to the admin menu |

Keep at least three positions and two candidates per position. Edits must belong
to your own draft election. Once the DO opens it, configuration is frozen.

To check persistence, return to the menu, choose Edit existing election, and select
your election. The app reloads saved positions and candidates from the database.

## 7. Register a voter and find their PIN

Open the election's voter screen with R, or use Register a voter from the admin menu
and select the election. Press **R** to open registration.

Example:

| Field | Example |
| --- | --- |
| Full name | `Demo Voter` |
| Date of birth | `2000-01-02` |
| NID/passport ID | `DEMO-VOTER-001` |
| Phone | `+15550000003` |
| Email | `voter@example.test`, or leave blank |

Press Enter. The backend checks the input and local duplicate identity records,
creates the voter, enrolls them, generates a numeric PIN, and records the event.
The PIN is delivered to:

```sh
cat 'virtual_sms/+15550000003.txt'
```

Find `Voting PIN:`. It is not displayed on the admin's registration or audit screen.
For a demo, let the person playing the voter read their inbox in the second terminal.

Other voter-screen controls: **I** enrolls an existing voter by NID, **E** edits
permitted unvoted draft profile fields, **D** removes a draft enrollment, and **Esc**
returns. NID and OTP phone changes are restricted to prevent redirecting credentials.
Registration uses local uniqueness checks; it does not verify real identity or phone
ownership against a government database.

## 8. DO: open the election

Sign out of the admin interface, or exit and restart the app. Choose District Official,
enter the DO's email/password, and read their newest OTP:

```sh
cat 'virtual_sms/+15550000002.txt'
```

Select the election, press **S**, and confirm with Enter/Y. It becomes open. Its
opening time and the responsible official are saved. Esc/N cancels a confirmation.

The DO can see election configuration but cannot list voter identities or audit
participation through the supported endpoints.

## 9. Voter: cast a ballot

Choose Voter from the main menu. Enter the **Voting PIN** from the voter's inbox.
Then read the same file again for the newly issued **Login OTP** and enter it.

Select your eligible open election. Use Up/Down to reach a candidate and Space to
select them. Pick one candidate for every position. You can change these local
selections until submission.

Press **S**, then Enter/Y to confirm the whole ballot. Once accepted, the choices
are final. Logging in again will not make that election eligible for another vote.
An incomplete ballot is rejected, and draft/closed elections cannot accept votes.

A timeout doesn't prove that a vote failed: the commit may have succeeded before
the response was lost. Refresh or log back in to check eligibility before retrying.

## 10. DO: close, tally, announce

Log back in as DO and select the election:

| Key | Action |
| --- | --- |
| F | Finish/close the open election; confirm |
| T | Tally the closed election; store candidate and party totals |
| A | Announce tallied results and display winners |
| R | Display results again |
| Esc | Return to election selection |

The results handle ties and positions with no votes. The DO sees candidate/party
totals, not the identity of people who voted for them.

## 11. Admin: audit history and simulated email ✉️

Log in as admin, choose View audit log, and select your election. Use Up/Down to
scroll, R to refresh, F to flag a concern, and Esc to return.

Audit entries include event time, type and actor. Registration/participation entries
identify the relevant voter. The admin can see **who voted**, but no PIN, ballot ID,
selected candidate or political-party choice.

Security alerts also use **local text files that represent email inboxes**:

```sh
cat 'virtual_sms/emails/systemadmin@gmail.com.txt'
cat 'virtual_sms/emails/admin@example.test.txt'
```

Use the admin email you actually registered. The system address defaults to
`systemadmin@gmail.com`; `VOTING_SYSTEM_EMAIL` in `.env` can change it. These files
are not Gmail mailboxes, and nothing is sent over SMTP. A file might not exist until
a relevant event has been delivered.

A message contains fields like these (illustrative values):

```text
To: admin@example.test
Subject: Voting system event — password_failed
Event ID: 42
Timestamp (UTC): 2026-10-19 18:00:00
Severity: 3
Message: password_failed
Details: See the authorized audit view; credentials and ballot contents are excluded.
---
```

The event name describes the error category. We intentionally do not put submitted
passwords, OTPs, PINs or raw request bodies in these alerts. Retry counts and structured
security details are stored in the database; the file is a notification, not a dump
of every internal log field.

The system inbox receives error alerts. An admin receives relevant account/election
alerts. Unidentified incidents may go only to the system inbox. DO notifications are
limited to configuration/lifecycle information and do not disclose voter participation.

Severity 3 means a rejected/suspected attempt; level 2 is an incident/flagged concern.
Level 1 denotes an outage, but this app cannot reliably email about its own death—an
independent monitor would be needed. A typo or timeout alone doesn't prove an attacker.

Delivery uses a database outbox so failed file writes can be retried later. A crash
can cause duplicate delivery with the same Event ID. SQLite audit records are the
source of truth; simulated emails are not independently tamper-proof evidence.

## 12. Where things live

| Location | Contents |
| --- | --- |
| `voting.db` | Default SQLite database, created automatically |
| `.env` | Private pepper and optional path/port settings |
| `virtual_sms/<phone>.txt` | Login OTPs, voter PIN delivery and permitted local notifications |
| `virtual_sms/engineer.txt` | Engineer setup OTP |
| `virtual_sms/emails/<email>.txt` | Simulated error email inbox |
| `tls-dev/` | Persisted certificates/private keys for separate-server mode |
| `src/ui/` | Screen rendering, forms and key handlers |
| `src/auth/login.rs` | Credential/OTP checks, retry limits and inbox delivery |
| `src/network/` | TLS transport, role gateways, sessions and endpoint dispatch |
| `src/services/` | Election/voter rules and SQL operations |
| `src/database/schema.sql` | Tables, foreign keys and security triggers |
| `src/validation.rs`, `src/security.rs` | Input checks, deadlines and structured security events |
| `STRIDE.md` | Detailed threat model and mechanism-to-test map |
| `implementation_details.md` | Function-by-function code map |

Keep real databases, secrets, inboxes and private keys out of your repository.

## 13. STRIDE: what happens if someone tries to cheat?

STRIDE names six kinds of threats. All six matter here. The examples below assume
an adversary who can use or modify a client but **does not control the trusted backend
OS account, database file, source code or private keys**.

| Category | Example adversary action | Our defense | What this does not guarantee |
| --- | --- | --- | --- |
| **S — Spoofing**: pretending to be someone else | Guess a password/PIN; replay an OTP; impersonate a gateway | bcrypt verifiers with a private pepper; random 20-digit voter PIN; expiring single-use OTP; retry cooldowns; server-held sessions; CA-validated TLS and distinct role gateway client certificates | Same-OS access to inboxes defeats the independence of the OTP step. Stolen credentials/sessions remain sensitive |
| **T — Tampering**: changing data | Change a request in transit, submit twice, rewrite an accepted vote, inject SQL | TLS integrity; bound SQL parameters; validated typed payloads; atomic ballot/participation transaction; FK/unique constraints; immutable ballot triggers; draft/owner checks | A database/OS owner can alter files or remove triggers; TLS cannot make a compromised endpoint trustworthy |
| **R — Repudiation**: denying an action | Deny opening an election or hide failed login attempts | Timestamped audit records with actors; structured authentication events; append-only triggers; event IDs and retryable notifications | Logs aren't signed by an independent witness and cannot resist a trusted OS owner rewriting the database |
| **I — Information disclosure**: reading secrets | Admin requests choices; DO requests voters; sniff traffic; inject secrets into logs | Role-scoped endpoint allowlist; ownership checks; no direct voter-to-ballot ID field; encrypted transport; sanitized events; restricted local file permissions | SQLite isn't encrypted at rest. One-voter results, collusion and timing observations can still reveal information |
| **D — Denial of service**: exhausting resources | Stall a request, send oversized frames, flood connections or keep guessing | Ten-second budget; socket watchdog; SQLite interruption/pre-commit checks; 1 MiB frame limit; 32 workers and 64 admissions/sec per listener; bounded retry records/cooldowns | Not protection against every flood. CPU/file operations aren't universally preemptible; targeted lockouts and disk exhaustion remain concerns |
| **E — Elevation of privilege**: gaining extra rights | Change a role field, forge a user ID, edit another admin's election, create staff through a normal client | Server-issued session after both login steps; current DB role/active checks; certificate-to-gateway-role binding; ownership checks; engineer-only signup capability | An attacker controlling the trusted OS/configuration or keys is beyond this application boundary |

### Why selecting “Admin” isn't enough

Menu choice only chooses which login flow to use. The server verifies the identity
and issues a session. Every protected operation checks the required role; ownership
checks also apply to admin elections. Editing the client to show an extra button
does not grant backend permission.

Each role gateway has a different client certificate/private key when connecting
to the database service. The database service checks both the gateway identity and
the person's session. Users aren't each issued certificates. The local SQLite file
does not speak TLS; its backend service does. Ordinary Rust function calls within
that service do not need a TLS handshake.

### Retry and timeout numbers you can explain in the demo

- Five wrong password attempts: five-minute cooldown for that identity's scope.
- Three wrong OTP attempts: invalidate the challenge and apply a five-minute cooldown.
- Ten wrong guesses at the same PIN lookup: five-minute cooldown for that lookup;
  this does not block every voter, and changing unknown PIN guesses is a remaining limitation.
- OTP expiry: five minutes; successful OTP use consumes the challenge.
- Request timeout: ten seconds across the request path; close the offending connection.
  We don't shut down the entire voting system or claim a permanent attacker/IP ban.

### The honest boundary

Default demo mode runs the TUI and services on one machine, in one process, with real
TLS connections between components. Separate server/client mode demonstrates the
network separation, but still isn't full host isolation. All listeners bind loopback.

A person who owns the backend OS account can read the simulated inbox, inspect SQLite,
replace the program or copy private keys. File permissions help separate OS accounts;
they do not stop the file owner. No claim is made of production-election certification,
cryptographic ballot anonymity or impossible-to-attack software.

## 14. Try the defenses on a disposable demo

Use synthetic accounts and your own local test installation. Don't lock another
person's account or modify a real election to test a hypothesis.

| Try this | Expected result |
| --- | --- |
| Submit an empty signup or impossible DOB | Form remains open with an error; no partial account |
| Register the same demo NID again | Duplicate rejected; no second PIN/identity |
| Log in before the DO opens the election | Voter cannot select the draft for voting |
| Submit without selecting every position | Incomplete ballot rejected |
| Log in after successfully voting | No second ballot for that election |
| Remove a position/candidate below the minimum | Backend rejects the edit |
| Edit an election after opening | Draft-only checks reject the change |
| Use a staff account in the wrong role login | Authentication/access rejected |
| Reuse a consumed OTP | Rejected; start a new login |
| Make five wrong password attempts on your disposable account | Cooldown and security events; relevant simulated email alert |

For attacks the TUI doesn't expose, use the supplied tests. They send malformed
payloads and cross-role requests directly, test untrusted/missing TLS certificates,
check duplicate-vote races and verify immutability/rollback at the database layer:

```sh
cargo test --locked
```

To exercise the actual terminal UI against a separate TLS server:

```sh
cargo build --locked
python3 tests/terminal_smoke.py
```

The Python script creates temporary synthetic accounts/data and drives the real TUI.
The corrected lockfile was checked with a successful Linux Cargo build, 44 passing
Rust tests and Clippy. Earlier TUI verification covered the phone-named inbox and
full election journey; macOS execution was not performed in our test environment.
Passing tests are evidence for those cases, not proof that all attacks are impossible.

## 15. Stuck? Start here

| Problem | Check |
| --- | --- |
| No text file | Did you initiate signup/login first? Correct phone, project folder and VOTING_SMS_DIR? |
| Wrong OTP | Newest code? Correct Setup/Login prefix? Six digits? Not expired or already used? |
| Invalid phone | Use 7–15 digits, optional leading +; remove spaces/hyphens |
| Password/PIN stopped working after an update | Did you preserve the original database and APP_PEPPER? |
| No election available | Is it open, is this voter enrolled, and have they already voted? |
| No email file | Was a relevant alert generated/delivered? Correct registered email and inbox root? |
| Terminal looks broken | Resize to at least 60 columns × 20 rows; Ctrl+C exits |
| Duplicate Rust function errors | Replace the old function/file; don't append another copy below it |
| Request timeout | Refresh before retrying a mutation; don't assume rollback |
| Lockfile parse error | Use the corrected generated Cargo.lock; don't manually shorten package versions |

Tell your teammate the command, the screen and the error text. Keep passwords,
PINs, OTPs, sessions and `.env` out of screenshots and group messages.

all best (all the best) — may your ballots be immutable and your demo uneventful 🫡
