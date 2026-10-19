# Demo prep — show the receipts 🎤

## 9. Supported functionality / remaining scope

| Requirement from supplied Implementation Details §2.1 | Demonstration |
| --- | --- |
| Admin creates election, >=3 positions and >=2 candidates each | Create new election, then show default President/VP/Secretary and two candidate assignments each |
| Customize positions and candidates | Add/rename/remove/link while draft; reopen Edit existing and confirm persistence |
| Register unique voters with name/DOB and PIN | R; include NID and phone; reject duplicate; inspect private simulated inbox |
| Admin audit log | Show configuration/lifecycle/registration/participation events without choices |
| DO starts/ends/tallies/announces | Open election, vote, close, tally, announce including ties/zero votes |
| Voter authenticates and casts once per position | PIN then OTP; choose one candidate for each position; confirm once; repeated cast rejected |
| Extra security monitoring | Trigger wrong password/OTP, inspect logs and email text inboxes |

The source project guidelines themselves were not supplied in full. Traceability here
uses your Implementation Details PDF's summary of Section 5.3 and later instructions.
National identity verification, real SMS/email, independent monitoring and production
anonymity are not implemented. See STRIDE.md for precise assumptions. Voting submits
one complete ballot atomically; independent partial-position submissions are not a UI feature.

## 10. Incorrect input (this is a demo, not a happy-path montage)

Demonstrate malformed email, misplaced phone `+`, impossible DOB `2001-02-29`, blank
candidate name, duplicate NID/email, wrong password, wrong/expired/reused OTP, incomplete
ballot, repeat vote, closed election, cross-election candidate ID and wrong-role request.

The shared validation module runs in the TUI and server. Backend service rules check
ownership/state/eligibility; SQLite checks foreign keys, uniqueness and immutability.
Errors return `Result::Err`, forms remain available, secrets clear, and transactional
mutations roll back. Security events use fixed descriptions, not raw submitted values.

`cargo test --locked` includes malformed-field tables, endpoint-bypass tests, UI form
rejection, duplicate/conflicting data, transaction rollback, unauthorized reads and
TLS failures. `tests/terminal_smoke.py` drives the real binary. Read docs/TESTING.md.

## 11. Threat model

Use STRIDE.md. Explain stakeholders, assets, trust boundaries, one attack per category,
its defense, its test and a residual limitation. All six categories apply. TLS does
not replace authorization; logs do not prevent privilege escalation; hashes do not
make a predictable PIN secret; an OS owner is outside our application isolation.

## 12. Code organization

The TUI, transport, authentication, services, database and shared validation are separate.
Small modules keep responsibilities visible. We intentionally retain three role gateways
and certificate identities as required. A single shared database mutex is simple and
serializes writes, but reduces throughput; this is an explicit classroom tradeoff.
Tests add lines intentionally. The generated lockfile is not handwritten implementation.

## 13. Installation

Main README includes macOS/Linux prerequisites, Rust/Python checks, extraction, locked
build, one-time pepper generation, engineer approval, admin/DO signup, OTP retrieval,
all roles, separate server/client setup, tests, troubleshooting and Rustdoc. Native
Windows and macOS runtime testing is not claimed by the Linux validation report.

## 14. Contributions — fill this in honestly

The PDF assigned initial tasks, but that is not evidence of completed contributions.
Do not present generated code as proof that a teammate implemented it. Each person
should record actual work and be able to explain, modify and test it.

| Member | Original planned area in PDF | Actual changes / commits / tests / review evidence |
| --- | --- | --- |
| Afia Fahmida Rahman | Authentication, voter flow, shared DO work | Fill in actual evidence |
| Chinnonye Okafor | Admin operations, shared DO work | Fill in actual evidence |
| Jeremy Durr | Logging | Fill in actual evidence |
| Parvez Alam | No implementation assignment shown in supplied task table | Agree a real implementation/test contribution and record it |

Everyone should demonstrate at least one valid journey and one rejected input in the
area they actually worked on. Disclose tool assistance according to the course policy.
