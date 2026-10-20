# Possible User Journeys

Hey teammate — here is the demo from zero to results. This is a terminal UI
(TUI): you type commands to launch it, then use keys and forms inside it.
Use fictional data. The government registry and SMS/email delivery are simulations.

## 1. wat do? (what to do)

Extract the complete project ZIP. Open Terminal in the extracted
`secure_voting_machine` folder — the one containing `Cargo.toml`.
Follow README.md for Rust and compiler installation. Then:

```sh
cargo build --locked
```

For a **new demo database only**, create the password secret once:

```sh
umask 077
python3 -c 'import secrets; open(".env", "x").write("APP_PEPPER=" + secrets.token_hex(16) + "\n")'
```

Already have `.env`? Keep it. Never change the pepper for an existing database.
For this walkthrough, use a fresh extracted folder with no existing `voting.db`.
Older voters may not match the new demo registry. Back up old data; do not delete it.

## 2. Engineer → create staff accounts

```sh
cargo run --locked -- --signup
```

In a second terminal, open the **same project folder**:

```sh
cat virtual_sms/engineer.txt
```

Copy the newest `Setup OTP:` into the first terminal. It expires in five minutes.
Complete email, password (10+ characters), full name and phone. Tab moves fields;
Left/Right changes the role. Enter registers. Create one Administrator and one
District Official, using different emails. Use your own fictional demo details.
For example, phones `+15550000001` and `+15550000002`.

This setup mode is reserved for the local engineer. Reading an engineer OTP on the
same OS account is a demo approval step, not a barrier against the machine's owner.

## 3. Staff → log in

```sh
cargo run --locked
```

Select the matching role, enter email/password, then find the latest login OTP:

```sh
cat 'virtual_sms/+15550000001.txt'
```

Replace that number with the account's registered number. Enter the six digits.
The message gives sender, recipient, sent time and expiry in New York time
(EST in winter, EDT in summer). It is a **local text file**, not a real SMS.
A successful login consumes the code; old codes are not reusable.

## 4. Admin → create and edit an election

Choose Create new election and enter a name. Defaults include President,
Vice President and Secretary, each with two candidates. Customize before opening.

| Key in editor | Action |
| --- | --- |
| Arrow keys | Move between positions/candidates |
| P | Add position |
| C | Add candidate |
| E | Edit selected position/candidate; candidate form includes party |
| D | Remove selected item, subject to minimum counts |
| R | Open voter registration/list |
| Esc | Return to previous screen |

Keep at least three positions and two candidates per position. Leave and reopen
through Edit existing election to verify persistence. Opening voting freezes edits.
Read the on-screen footer for actions on the current screen.

## 5. Admin → register an eligible voter

Use all four fields from the same row in `demo/voter_ids.txt`. The separate
`demo/statewide_voters.sqlite` database stores voter ID, name, DOB and phone.
The server opens it read-only; `VOTING_REGISTRY_DB` selects a different database.
A missing/unreadable database blocks startup; there is no permissive fallback.
`demo/registry.sql` reproduces the fictional database. Editing the text cheat sheet
alone does not change the database. No real government or mobile service is connected.

| Voter ID | Full name | DOB | Phone |
| --- | --- | --- | --- |
| DEMO-NY-001 | Afia Demo | 2000-01-02 | +15551234567 |
| DEMO-NY-002 | Sam Student | 1999-06-15 | +15551234568 |
| SMOKE-NID | Terminal Voter | 2000-01-02 | +15550000003 |
| DEMO-NY-MINOR | Young Demo | 2020-01-02 | +15551234569 (must be rejected) |

An unknown ID shows: “This user does not have a state-wide identifier to take part
in this election.” A phone/name/DOB mismatch is rejected. Both the entered DOB and
the stored DOB must be adult; forging an adult DOB for the minor record fails.
Underage attempts create `voter_not_eligible_underage` (severity 2), attributed to
the administrator without the applicant's identity. Other categories are
`voter_identifier_not_found`, `voter_registry_mismatch`, and `voter_registry_unavailable`.

After registration, **Did voter receive PIN?** accepts `yes` or `no`. The PIN appears
only in `virtual_sms/<registered-phone>.txt`, never on the admin screen. The answer
is audited without the voter ID; `no` is a warning and keeps the registration saved.
This records the administrator's report, not independent proof of delivery.

Read the PIN from the matching phone file, for example:

```sh
cat 'virtual_sms/+15551234567.txt'
```

Find `Voting PIN:`. Keep this PIN for later logins. It is different from the
six-digit, short-lived OTP. Duplicate registration is rejected; the existing-voter
enrollment action reuses an existing identity/PIN for another election.

## 6. DO → open voting

Log in as District Official, select the draft election, press S and confirm.
Only an election that has never opened can open. Try F or T on a draft:
closing/tallying must fail without changing its state. Try S again after opening:
that must fail too. Tallying while open also fails.

## 7. Voter → choose, review, submit

Log in as Voter with the registered PIN, then the latest OTP in the same phone file.
Before opening, the election is unavailable. After opening, select the election.

1. Use Up/Down to move through candidates.
2. Press Space to select one candidate for each position. Selecting another
   candidate for that position replaces the local choice.
3. Press S to submit. Missing positions are rejected.
4. Read the confirmation; Enter confirms the complete ballot.

Selections stay in client memory until final confirmation. Esc leaves the ballot
and clears selections. Successful submission writes the complete ballot and
participation atomically, queues the receipt, and signs the voter out.
The phone file receives `thank you for your vote!`.
Log in again and attempt the same election: `you already voted bruh` appears.
No second ballot is stored. One ballot per voter **per election**, one choice per
position, and no editing a submitted ballot.

The internal `election_voters.has_voted` flag prevents repeats. Vote events and
choices do not enter the audit log. The admin can see participation in the voter
list, but cannot read ballot choices. The DO sees anonymous results, not identities.
Old vote audit rows from earlier versions are hidden from the admin view, not
silently erased from an existing database.

## 8. DO → close, tally, announce

| Election state | Allowed next lifecycle action | Rejected examples |
| --- | --- | --- |
| Draft | S: open | F: close; T: tally |
| Open | F: close | S: open again; T: tally |
| Closed | T: tally | S: reopen; F: close again |
| Tallied | A: announce; R: view results | S: reopen; T: tally again |

Confirm each change. Results display candidate/party totals and winners for every
position. Ties are shown as ties; zero votes show no winner. Closing ends voting.

## 9. Admin → inspect logs and simulated emails

View audit log shows creation, registration and lifecycle events with time and
actor. It must not reveal ballot IDs, selections, PINs, OTPs or session tokens.
Security failures also generate sanitized notifications. Actual inbox paths are:

```text
virtual_sms/emails/systemadmin@gmail.com.txt
virtual_sms/emails/<administrator-account-email>.txt
```

The system recipient can be configured with `VOTING_SYSTEM_EMAIL`.
An admin registered with `admin@gmail.com` gets that filename; it is not a
hard-coded recipient for every admin. Recipient filtering limits who receives
which alerts. DO notifications do not expose voter identities. Files use email-style
fields such as recipient, subject, timestamp and event ID. Nothing is sent to Gmail.
Queued delivery retries after failure; a crash can duplicate a notification.

## 10. Incorrect inputs — try these in a disposable demo

| Try | Expected |
| --- | --- |
| Empty signup/registration, malformed phone, impossible/future DOB | Validation error; no account/voter created |
| DOB in 2020 | Under-18 rejection |
| Unknown voter ID or another name with a known ID | Registry rejection; no PIN generated |
| Register same identity twice | Rejected; use existing enrollment workflow |
| Incorrect password 5 times | Account cooldown for 5 minutes; failures logged |
| Incorrect OTP 3 times | Challenge invalidated; 5-minute cooldown; fresh login required |
| Vote before opening or after closing | Backend rejects |
| Submit missing positions or repeat a submitted ballot | No partial/extra ballot |
| Admin asks for ballot data; DO asks for voter records | Backend denies, even outside the UI |
| Request exceeds 10 seconds | Connection closed; timeout recorded; response loss does not prove rollback |

The last two are exercised by the Rust endpoint/transport tests; the ordinary
menus deliberately do not expose arbitrary requests or a slow-request button.

## 11. STRIDE — what stops what?

| Threat | Implemented defense | Remaining limit |
| --- | --- | --- |
| Spoofing | Password/PIN + single-use OTP, expiring role-bound sessions, distinct gateway client certificates | Shared OS account can read simulated inboxes; stolen bearer sessions remain a risk |
| Tampering | TLS, bound SQL parameters, transactional ballots, uniqueness and immutable-ballot triggers | OS/database owner can bypass application controls |
| Repudiation | Persistent lifecycle/security logs with actors and timestamps | We intentionally do not log vote events; no cryptographic nonrepudiation claim |
| Information disclosure | Admin/DO authorization, scoped responses/alerts, no identities in ballot tables or secrets in logs | Timing/order and tiny electorates can still reveal information |
| Denial of service | 10-second deadlines, connection/request bounds, retry cooldowns | Targeted lockouts and large floods are not fully prevented |
| Elevation of privilege | Session, current DB role and gateway checks; explicit engineer setup | Local machine owner is trusted; setup OTP is simulated |

All six categories apply. These are defenses, not a claim that nobody can attack
us. Read STRIDE.md for the complete threat model and trust assumptions.

## 12. Run the checks yourself

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
python3 tests/terminal_smoke.py
```

The terminal script launches the actual TUI and separate TLS server, types into
forms, reads synthetic inboxes and checks disposable SQLite state. It covers the
Section 5.3 lifecycle plus underage/unknown-ID rejection, selection-before-submit,
receipt, sign-out, repeat voting and privacy. Tests use temporary data, not your
real database. See docs/USE_CASES.md for requirement mapping and docs/TESTING.md
for run evidence and limits. Native macOS execution has not been verified here.

all best (all the best) — rehearse with fake voters, keep your real pepper private.
