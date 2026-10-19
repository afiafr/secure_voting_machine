# Validation report — v0.2.2

Validated on Linux with rustc 1.99.1 and the included Cargo.lock. This is functional
and negative-path testing, not certification, formal verification or a penetration test.
Native macOS/Windows execution was not available. The earlier macOS accepted-socket
blocking-mode fix is retained and covered by a portable regression test.

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| Fresh Cargo Clippy | Blocked by dependency resolution |
| Tests built with rustc and retained compiled dependencies | 44 passed, 0 failed |
| Fresh `cargo build --locked` | Blocked by pinned num-traits 0.2.29; existing compiled dependencies used to rebuild binary |
| Rustdoc with retained dependencies | Passed |
| `python3 tests/terminal_smoke.py` | Passed; full real-TUI workflow including empty forms |

Run transcripts are included in `tests/latest-*-run.txt`. Tests use synthetic temporary
accounts and databases. Rust fixtures use lower test-only bcrypt cost; the real compiled
TUI smoke run uses normal bcrypt cost. Slow-response tests take about eleven seconds
and verify rejection/connection closure at the ten-second budget, not acceptance of
an eleven-second response.

## Incorrect-input coverage (oral-exam question 10)

| Layer | Rejected cases | Evidence |
| --- | --- | --- |
| Shared validation | Empty/malformed/oversized emails, path separators, invalid plus signs, short/long phone, nondigit/Unicode OTP, short signup password, oversized password, controls, impossible/future/old DOB | `validation::tests` |
| TUI | Invalid signup remains open, secrets masked, no user row inserted; failed edits do not create local placeholders | `ui::tests` |
| Direct endpoints | Wrong payload shape, blank/oversized name, impossible DOB, unknown endpoint, cross-role voter/audit/result reads; forged log text discarded | `direct_endpoints_reject_bad_fields_and_cross_role_reads` |
| Authentication | Wrong passwords, OTP failure threshold/expiry/replay, cooldown, inactive user, forged/expired session, wrong role gateway, disabled setup | Authentication and network tests |
| Database | Duplicate identity, wrong-election candidate, frozen configuration, invalid lifecycle, duplicate vote, ballot rewrite/delete, delivery/write failure rollback | Service tests; concurrent duplicate submission uses two SQLite connections |
| Transport | Unknown CA, wrong hostname, absent internal client certificate, plaintext, oversized/truncated framing, nonblocking accepted socket | Network TLS/framing tests |
| Deadline | Expired request cannot create election; expired mutation does not commit; slow response rejected; watchdog closes stalled handler | Four deadline tests |
| Notifications | Failure counts and lockout events, secret exclusion, DO not receiving admin authentication alerts, delivery failure remains queued and retries | Security event and email retry tests |

SQL parameters are bound. UI format checks are not treated as authorization. Cross-role
requests are sent directly to the endpoint to show that hiding menu items is not the
security boundary. A denied admin `ballots.list` also verifies there is no generic raw
ballot endpoint; DO tally/result access is tested through supported operations.

## Actual compiled TUI journey

The Python script starts the binary in a POSIX pseudo-terminal and sends keyboard input:

1. Read engineer setup OTP, reject empty signup, create Administrator and DO accounts.
2. Generate the CA/server/distinct gateway certificates and start a separate TLS server.
3. Start a client without APP_PEPPER, log in as admin with password and delivered OTP.
4. Create default positions/candidates; add, rename and delete an extra position;
   add, edit (name/party) and remove a candidate; reopen the saved election.
5. Reject empty and duplicate registration, register a voter, inspect private PIN
   delivery and the admin's registration audit screen.
6. Log in as voter before opening; verify no election is selectable.
7. Log in as DO and open election; verify state/timestamp in SQLite.
8. Log in as voter, reject an incomplete ballot, then cast one choice per position.
   Log back in and verify the election cannot be selected for another ballot.
9. Close, tally and announce as DO; show winners without voter identities.
10. Read admin participation audit without candidate/party choices; check actor IDs,
    timestamps and database foreign-key integrity. Clean up all disposable data.

The fixed-size terminal harness reconstructs cursor-addressed text for assertions;
raw ANSI output is not a complete screen. Renderer tests additionally cover small
terminals, masking, role switch/logout and ties/zero-vote results. Service/TLS tests
exercise malicious requests that the UI does not expose, including post-close
voting, duplicate submissions and unauthorized reads. A new audit-failure test
checks rollback of a draft edit when its audit insert fails.

## Remaining boundaries

No real email/SMS/national-ID provider was contacted. No claim is made about independent
MFA, OS-root resistance, cryptographic ballot unlinkability, distributed flood protection
or performance under election-scale load. See STRIDE.md. Failure to receive a response
does not prove an earlier commit was rolled back. Email delivery can be duplicated after
a crash; Event ID is the deduplication key.


### Observed intermittent smoke failure

One terminal run did not observe voter OTP delivery within the test wait. A diagnostic
rerun passed the complete journey; the underlying failure was not reproduced. The
harness now waits for the main screen before entering credentials rather than relying
only on a fixed startup delay, and prints synthetic terminal output if OTP delivery
fails. This is recorded as an unresolved intermittent observation, not a proven backend
fix. Final run output is retained in `tests/latest-terminal-run.txt`.

Phone-named inbox regression: `inbox_uses_phone_filename_and_rejects_paths`
checks exact delivery filename/content, engineer alias, and rejected traversal
inputs. The real TUI script reads the new filenames during signup and login.

For v0.2.2, the fresh registry offered num-traits 0.2.19 but not the lockfile
version 0.2.29. The dependency lock was not changed as part of this filename fix.
The 44 tests and terminal run use the updated application compiled with retained
dependency artifacts. This does not establish a working clean Cargo build.
