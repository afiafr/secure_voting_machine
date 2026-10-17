# Validation report — version 0.1.2

Validated on Linux with rustc 1.99.1 (2026-10-13) and the included Cargo.lock.

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --locked` | 33 passed, 0 failed |
| `cargo build --locked` | Passed |
| `python3 tests/terminal_smoke.py` | Passed |

## macOS socket-mode regression

The listener is nonblocking, but TLS workers must use blocking sockets. Version
0.1.2 explicitly calls `set_nonblocking(false)` on each connected socket, including
accepted gateway and database sockets. This avoids platform-dependent inheritance.
A regression test forces an accepted socket into nonblocking mode before applying
configuration and delays the peer's data. It failed with `WouldBlock` before the fix
and passes afterward. This reproduces the socket-mode defect on Linux; the user's
macOS signup still needs confirmation on their machine.

## TLS signup regression checks

The patched code has now been compiled and tested with the installed Rust toolchain.
Tests verify clean TLS shutdown after successful signup on both hops, rejection of
incomplete frames, and a complete response after an 11-second server delay. The
separate terminal smoke test uses normal bcrypt cost and creates both staff roles.
The reported continuing error was not reproduced on Linux with this version.

The delayed-response test intentionally takes about 11 seconds. Version 0.1.2 also
reports the endpoint and transport phase when a TLS failure occurs, rather than
showing an unexplained EOF alone. Use `cargo run --locked -- --version` to check
which source is being executed.

## What was exercised

Database/service tests cover default election creation, idempotent initialization,
legacy account preservation, position/candidate CRUD and shared assignments, owner
and role checks, duplicate identity rejection, PIN generation, delivery rollback,
OTP and password limits, enrollment changes, lifecycle transitions, timestamps,
idempotent candidate/party tallies, election cancellation and audit retention.

Ballot tests inject a write failure to verify rollback, reject cross-election foreign
keys, test update/delete prohibition, verify frozen ballot structure, and race two
independent SQLite connections: exactly one ballot is accepted.

Staff-signup tests create both roles, reject duplicate emails, check stored roles through
the login endpoint, and confirm signup does not grant a session.

UI tests use Ratatui's test backend and actual TLS calls. They verify login leads to
the four-option admin menu, create/edit reloads database state, failed writes do not
leave local placeholders, R registers a voter while N does nothing, secrets are masked,
small terminals render a resize message, voting is final, ties/zero-vote results are
shown, and logout clears prior role state.

Network tests use real TCP sockets and both TLS hops. They verify an entire election,
OTP expiry/retry/replay rejection, session expiry/logout, forged-token rejection,
gateway role binding, untrusted CA/wrong hostname rejection, missing internal client
certificate rejection, plaintext rejection, bounded framing, and transport-flood
incident logging. No certificate-verification bypass is used in tests.

The direct-usage smoke test starts the compiled program in a POSIX pseudo-terminal.
It creates staff accounts using keyboard input, generates development certificates,
starts a separate server process, and runs clients without a pepper in their environment.
It then logs in with password+OTP, creates an election, registers a voter and reads the
virtual PIN delivery, opens voting as an official, authenticates the voter with PIN+OTP,
casts a complete ballot, closes/tallies/announces and checks SQLite foreign-key integrity.

The recorded smoke output was:

```text
PASS: terminal setup creates admin and district official via TLS
PASS: password + OTP login, election defaults, voter registration and PIN delivery
PASS: district opens election; PIN + OTP voter submits one complete ballot
PASS: close, tally, announce and database integrity
All direct terminal smoke checks passed; temporary data removed.
```

## Limits of this validation

The source was built and exercised on Linux, not on Windows/macOS. The terminal smoke
script specifically requires POSIX; the Rust tests do not require Python. This was
functional and negative-path testing, not a penetration test, load-capacity certification,
formal proof of ballot secrecy or production-election audit. No real SMS, email or
national-ID provider was contacted. See SECURITY.md before interpreting test success
as a security guarantee.
