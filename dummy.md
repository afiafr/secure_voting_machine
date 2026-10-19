# dummy.md — wat do? (what to do) 🧭

Okay, so you've opened the ZIP and there are a bunch of Rust files. You don't
need to understand all of them before trying the app. Let's get it running first,
then we can follow what happens when you press a button.

We're building a voting system for the security course. An admin sets up elections
and registers voters, a District Official opens/closes elections and counts votes,
and a voter casts a ballot. The menus run **inside Terminal**. That's what TUI
means. There's no website to open. We removed generated HTML to keep things smaller.

**Before you start:** our fresh build hit a dependency-download issue involving
`num-traits 0.2.29`. We tested this update using compiled dependencies already on
hand, but haven't verified a fresh install. If you hit that error, you haven't
messed up. Check the build note in section 2; don't delete your database or secret.

## Start here if you've never loaded the project

1. Download and unzip the project. On a Mac, double-click the ZIP. Find the folder
   containing `Cargo.toml`, `README.md`, and `src`.
2. Open **Terminal**: Spotlight → type Terminal → Enter. VS Code's integrated
   terminal works too, but VS Code isn't required.
3. Type `cd `, including the space, drag that project folder into Terminal, then
   press Enter. This tells Terminal which folder we're working in.
4. Type `ls`. You should see `Cargo.toml`. If you only see another folder named
   `secure_voting_machine`, go inside that folder and check again.
5. Check whether Rust is installed:

```sh
cargo --version
rustc --version
```

If either says “command not found”, install Rust using the official installer linked
in [README.md](README.md), reopen Terminal, and return to the project folder.
On a Mac, you may also need Apple's compiler tools:

```sh
xcode-select --install
```

Then build the app:

```sh
cargo build --locked
```

“Build” means Cargo turns our `.rs` source into a runnable program. The first build
downloads dependencies, so it needs internet access and may take a while. Wait for
it to finish. If it stops at the dependency error above, this step did not succeed;
don't continue as though it did.

After a successful build, the program is `target/debug/secure_voting_machine`.
You don't double-click `main.rs`; you run the program from Terminal. Follow
**section 2** to create the secret and staff accounts, then section 3 for the first
election. Section 1 comes first because “where did my OTP go?” is probably the
first thing we'll ask each other.

Commands here are for macOS/Linux. Windows/WSL setup is in [README.md](README.md).
The function-by-function map is [implementation_details.md](implementation_details.md).
You can leave that for later. [docs/USE_CASES.md](docs/USE_CASES.md) maps the assignment.

## 1. Where is my text-message OTP? 📱

**Check the text file named after the phone number you registered.** Your actual
phone will not buzz—we're simulating SMS with local files. No Twilio bill needed.

If you registered with `+15550000001`, open this file in the project directory:

```text
virtual_sms/+15550000001.txt
```

Leave the app on the OTP screen. Open a **second Terminal tab/window** and `cd`
into the same project folder. Then type:

```sh
cat 'virtual_sms/+15550000001.txt'
```

Or use the app's inbox command after building:

```sh
./target/debug/secure_voting_machine --inbox +15550000001
```

That number is just an example—use your registered number, including `+`
if you supplied it. A number without `+`, such as `01712345678`, uses
`virtual_sms/01712345678.txt`. Phone numbers accept 7–15 digits with an optional
leading `+`; spaces, hyphens and path separators are rejected.

The file keeps old messages too. Scroll to the bottom and use the **latest
relevant entry**. Here's how to tell them apart:

| Message prefix | Meaning | Where to enter it |
| --- | --- | --- |
| `Login OTP:` | Six-digit, single-use login code | The OTP form after password or PIN verification |
| `Voting PIN:` | Twenty-digit voter credential created at registration | Voter login's PIN field; then retrieve the separate login OTP |
| `Setup OTP:` | Engineer approval for staff signup | The terminal prompt before the signup form |

OTPs expire after five minutes. Do not confuse the PIN with an OTP. The PIN is
not regenerated each login. Admins register voters but do not see the PIN in the
admin UI or audit view.

The engineer approval file is **`virtual_sms/engineer.txt`**:

```sh
cat virtual_sms/engineer.txt
./target/debug/secure_voting_machine --inbox engineer
```

If `.env` sets `VOTING_SMS_DIR`, that directory replaces `virtual_sms`. For example,
`VOTING_SMS_DIR=messages` means `messages/+15550000001.txt`. Relative paths are
relative to the directory you launch from. In separate-server mode, inbox files
are written on the server machine, not automatically copied to the client.

Older versions used hash-named files. Those files are left untouched; start a new
login to receive an OTP in the phone-named file. Never copy inboxes into GitHub.
Anyone with access to the same OS account can read them: this is simulated delivery,
not independent real-world MFA.

## 2. First run: let's make the staff accounts

Open a terminal in the extracted `secure_voting_machine` folder, where `Cargo.toml`
lives. Install Rust, a C toolchain and Python as described in the README.

```sh
cargo build --locked
```

**Known build issue in this verification environment:** the fresh registry could
not resolve the existing lockfile's `num-traits 0.2.29`. The updated app was tested
using retained compiled dependencies. If you hit that error, it is a dependency
resolution problem, not an OTP or database problem. See `docs/TESTING.md`; a clean
Cargo build has not been established for this release. Do not delete your database
or pepper to try to fix it.

Next we need a private secret called a **pepper**, saved in `.env`. The app uses it
when checking credentials. For a brand-new setup, copy this command once. If you
already have a `.env` from the previous version, keep that one:

```sh
umask 077
python3 -c 'import secrets; open(".env", "x").write("APP_PEPPER=" + secrets.token_hex(16) + "\n")'
```

If Python says the file already exists, don't panic—that prevents us from
overwriting your old secret. Keep the same pepper with the same database.
Changing it means existing passwords and PINs won't verify correctly.

After building successfully:

```sh
./target/debug/secure_voting_machine --signup
```

1. Leave this terminal open at the engineer approval prompt.
2. In a second terminal, open `virtual_sms/engineer.txt` and read its latest Setup OTP.
3. Enter that OTP in the first terminal.
4. Fill in email, password, full name and phone number.
5. Use Tab to move fields. At Role, use Left/Right or Space to choose Administrator
   or District Official. Press Enter to submit.
6. Signup creates the account; it does not log you in. Create the other staff role
   through the signup option, or launch `--signup` again and approve setup again.
7. Exit with Ctrl+C. Start the normal app with the command below.

Staff signup is an explicit local engineer capability. An ordinary voter cannot
choose a privileged role during registration. Voters are registered by an admin.

```sh
./target/debug/secure_voting_machine
```

Choose your role with Up/Down and Enter. Staff use email/password, then the OTP
from their phone-named file. Voters use PIN, then their phone's login OTP.

## 3. Admin: create, edit, register, audit

After login, **wat do? (what to do)** has four options:

| Option | What happens |
| --- | --- |
| Create new election | Enter a name; persist a draft with President, Vice President and Secretary |
| Edit existing election | List your elections and load saved positions/candidates from the DB |
| Register a voter | Choose your election, then use R on its voter screen |
| View audit log | Choose your election and read lifecycle/registration/participation events |

Each default position has two editable candidates, Hamberger Red and Blue Sandwich,
with names and political parties. The defaults share candidate identities across
positions: editing one shared candidate changes it in every linked position.

Election editor controls:

| Key | Action |
| --- | --- |
| Arrows | Select a row and Position/Candidate/Party column |
| P | Add a position, with two default candidates |
| C | Add a candidate to the selected position |
| E | Edit selected position, or candidate name and party |
| D | Delete selected draft position or remove candidate assignment; confirm first |
| L | Link an existing candidate ID from this election to the selected position |
| F2 | Rename election |
| X | Cancel draft election; audit history remains |
| R | Open the voter screen |
| Esc | Return to the admin menu |

The backend keeps at least three positions and two candidates per position.
Configuration edits are limited to your own drafts. Once the DO opens voting,
configuration is frozen. After changing data, the client reloads server state.

On the voter screen:

- **R:** register a new voter. Enter full name, DOB (`YYYY-MM-DD`), NID/passport ID,
  phone and optional email. The PIN goes into that voter's phone-named text file.
- **I:** enroll an existing voter by NID; their PIN stays the same.
- **E:** edit permitted profile fields for an unvoted draft enrollment. NID and
  OTP phone cannot be redirected here.
- **D:** remove a draft enrollment. It does not erase credentials or voting history.
- **Esc:** return to the menu. N is not a second registration action.

Registration checks local NID/email duplicates and valid dates. It does not contact
a government identity service. An invalid registration must not leave partial rows.

Audit: Up/Down scroll, R refreshes, F flags a concern, Esc returns. The admin sees
who participated, event times and actors, but **no PIN, ballot contents or choices**.

## 4. District Official: run the election

Log in as District Official with password and OTP. Select an election:

| Key | Action and rule |
| --- | --- |
| S | Start/open a draft with valid minimum positions/candidates |
| F | Finish/close an open election |
| T | Tally a closed election; persist candidate and party totals |
| A | Announce tallied results and display winners |
| R | Show tallies and winners |
| Esc | Choose another election |

Confirm lifecycle actions with Enter/Y; cancel with Esc/N. Open/close timestamps
and the responsible official are saved. Results handle ties and no-vote positions.
The DO sees anonymous results, not voter names or who voted.

## 5. Voter: one final ballot

1. Get the Voting PIN from your phone-named file after registration.
2. Choose Voter, enter that PIN, then read and enter the new Login OTP.
3. Select an eligible **open** election. Draft/closed elections are unavailable;
   an election you already voted in is also unavailable.
4. Use Up/Down to reach a candidate and Space to select them. Select one candidate
   for every position. You may change these local selections before submission.
5. Press S, review the confirmation, then Enter/Y to submit the complete ballot.
6. After acceptance, you cannot edit the vote or submit another ballot for that
   election. Esc before submitting returns without casting.

The backend verifies every position/candidate pair. It writes the ballot choices
and participation flag in one transaction. Failure rolls everything back; two
simultaneous submissions cannot both succeed. Ballot tables contain no voter ID,
PIN or session token. This is not a claim of cryptographic anonymity against an
OS/database owner.

## 6. Where did everything go?

Paths below are relative to the launch directory unless configured otherwise.

| File/folder | Contents | Who should handle it? |
| --- | --- | --- |
| `voting.db` | SQLite users, credentials, elections, ballots, logs and outboxes | Backend; trusted OS operator only |
| `.env` | APP_PEPPER and optional configuration | Trusted operator; never commit |
| `virtual_sms/<phone>.txt` | That phone's simulated OTP/PIN messages | Recipient in the demo; local OS access remains trusted |
| `virtual_sms/engineer.txt` | Setup approval OTP | Engineer/operator |
| `virtual_sms/emails/systemadmin@gmail.com.txt` | Default system error alerts | System operator |
| `virtual_sms/emails/<admin-email>.txt` | Relevant administrator error alerts | That administrator in the demo |
| `tls-dev/` | Persistent TLS material for separate-server mode | Operator; keep private keys private |
| `tests/latest-*-run.txt` | Verification transcripts | Safe synthetic test evidence |
| `target/` | Generated binaries/build files | Cargo; excluded from the handoff and Git |

Error emails are local files, not messages sent to Gmail. They appear when a relevant
error is logged/delivered. `VOTING_SYSTEM_EMAIL` changes the system recipient.
`VOTING_DB`, `VOTING_SMS_DIR`, `VOTING_TLS_DIR` and `VOTING_PORT` change paths/ports.
Examples are in `.env.example`. DB audit records remain authoritative; delivery
outboxes allow retries. Logs omit submitted secrets and vote choices.

Main table groups in `src/database/schema.sql`:

| Tables | Purpose |
| --- | --- |
| `users`, `user_credentials`, `voter_pin_credentials` | Identity/roles and password/PIN verifiers |
| `elections`, `election_positions`, `candidates`, `position_candidates` | Election configuration and candidate assignments |
| `election_voters` | Enrollment and whether the voter voted |
| `ballots`, `ballot_choices` | Anonymous ballot records and selected candidates |
| `candidate_results`, `party_results` | Stored totals |
| `audit_events`, `security_events`, `auth_attempts` | Lifecycle/security evidence and retry state |
| Notification/email outboxes, `system_settings` | Queued local delivery and system configuration |

Do not manually change tables to bypass a failed login or accepted-vote constraint.

## 7. Okay, but which file does what?

| File/folder | What to read it for |
| --- | --- |
| `src/main.rs` | Startup, --signup, --inbox and server/client modes |
| `src/models.rs` | Roles and domain data passed between layers |
| `src/ui/mod.rs` | Screen state, selected election, login session, navigation helpers |
| `src/ui/draw.rs` | What the TUI renders; labels, fields, menus, masking |
| `src/ui/events.rs` | Which action each key triggers |
| `src/ui/forms.rs` | Form validation, submissions and confirmed mutations |
| `src/network/client/` | Typed calls from UI to TLS endpoints |
| `src/network/tls.rs` | Certificates, TLS configuration and framed encrypted transport |
| `src/network/server.rs` | Listeners, worker limits and request deadlines |
| `src/network/state.rs` | Server sessions, OTP challenges and authentication dispatch |
| `src/network/dispatch.rs` | Explicit allowlist of election/voter endpoints |
| `src/network/protocol.rs` | Serialized request/response structures |
| `src/network/monitor.rs` | Transport incidents and timeout reporting |
| `src/auth/login.rs` | Password/PIN/OTP checks, retry limits, delivery and mailbox_path |
| `src/services/election.rs` | Election CRUD, ownership, voting, lifecycle and results |
| `src/services/voters.rs` | Registration, PIN creation/enrollment, profile rules and staff provisioning |
| `src/services/mod.rs` | Role checks, audit writing and finish_change transaction helper |
| `src/database/mod.rs` | Open/init/migrate SQLite; foreign keys and file permissions |
| `src/database/schema.sql` | Tables, relationships, immutability and state triggers |
| `src/validation.rs` | Shared email, phone, text, date, password and code checks |
| `src/security.rs` | Ten-second deadline helpers and structured security events |
| `src/test_support.rs`, module `tests.rs` files | Disposable fixtures and Rust tests |
| `tests/terminal_smoke.py` | Starts real server/TUI processes, presses keys and checks SQLite |
| `scripts/generate_docs.py` | Rebuilds the Markdown function inventory from source comments |

Example: R registration → `events.rs` opens form → `forms.rs` validates → typed
client sends `voters.register` over TLS → admin gateway forwards over mTLS → DB
service verifies session/gateway role → voter service checks ownership/duplicates →
transaction inserts identity/credential/enrollment/audit and delivers PIN → commit →
client refreshes. `mailbox_path` is where the phone-number filename is constructed.

`finish_change` records an election change, checks the deadline, then commits.
Taking ownership of the transaction makes early errors roll back data and audit.

## 8. Why all the TLS stuff?

The UI talks to the selected role gateway over TLS. Each gateway talks to the DB
service over mutual TLS, using its own role certificate/private key. SQLite itself
is a local file accessed by that service; it is not a TLS-speaking database server.

A certificate identifies a gateway. A person's session is issued only after their
password/PIN and OTP succeed. The backend checks gateway role, session role, current
DB role, and election ownership where required. Choosing Admin in a menu grants no
permissions. SQLite has no automatic per-user table grants; our service enforces
access, while the OS protects its files.

Default mode runs TUI and services together with temporary certificates. For a
separate local server/client, stop the combined app first, then initialize persisted
certificates once and run these from the same configured project directory:

```sh
./target/debug/secure_voting_machine --init-tls
./target/debug/secure_voting_machine --serve
```

In another terminal:

```sh
./target/debug/secure_voting_machine --client
```

Default ports: DB service 7440, admin 7441, DO 7442, voter 7443, all loopback.
Clients need the trusted CA, not the DB, password pepper or gateway private keys.
Do not regenerate certificates while an existing server/client pair is running.

## 9. When it says nope

| Symptom | What to check |
| --- | --- |
| OTP file missing | Start login first; correct phone, launch folder, VOTING_SMS_DIR and server machine? |
| OTP invalid | Latest Login OTP, six digits, unexpired, not already used? Do not enter Voting PIN here |
| Login locked | Wait for cooldown; do not restart or edit the DB to bypass it |
| Voter already exists | Use I to enroll their existing NID; do not issue another identity/PIN |
| No election available | Correct owner/enrollment? Open for voter? Already voted? |
| Cannot edit | Own draft required; open/closed configurations are frozen |
| Incomplete ballot | Select one candidate for every position |
| Request timed out | Refresh state before retrying; a committed operation may have lost its response |
| TLS connection refused/EOF | Correct binary/server, matching CA and ports? See README troubleshooting |
| Terminal too small | Resize to at least 60 columns by 20 rows |
| Missing APP_PEPPER | Launch from the folder containing the original .env, or configure the environment |

Five wrong passwords cause a five-minute cooldown. Three wrong OTPs invalidate the
challenge and cause a five-minute cooldown. Ten wrong guesses at the same PIN value
block that lookup for five minutes. Retry events are logged without the secrets.
Requests have a ten-second budget; timeout closes that connection, not the whole app.

## 10. Before we demo: does it actually work?

Once Cargo dependency resolution is working:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
python3 tests/terminal_smoke.py
```

The terminal script uses synthetic accounts and a temporary database; it cleans up
afterward. The latest verification had 44 passing Rust tests and a passing real-TUI
journey using retained compiled dependencies. Fresh Cargo resolution was blocked as
noted above; see [docs/TESTING.md](docs/TESTING.md) for the exact qualification.

Use [STRIDE.md](STRIDE.md) for threats/defenses and [docs/DEMO.md](docs/DEMO.md) for
oral-exam preparation. Be ready to explain validation at both client and server,
role separation, atomic ballots, audit privacy, retries and TLS. Describe your real
contributions using commits/reviews/tests; do not invent teammates' contributions.

Rust survival kit: `Result` means success/error; `?` propagates an error; `Option`
means a value may be absent; `&` borrows; `enum` lists alternatives; `mod` organizes
code; `#[cfg(test)]` limits code to tests; `///` documents a function.

If you get stuck, tell your teammate the command you ran, the screen you reached,
and the error text. Please don't paste your password, PIN, OTP or `.env` into the
group chat.

all best (all the best) — see you at the demo 🫡
