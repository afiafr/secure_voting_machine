# Section 5.3 — wat do? (what to do)

Source: the uploaded `image(2).png`, headed “5.3 E-Voting System”, page 20 and
its continuation. This checklist covers every visible requirement in that image.
The following tests are included; see TESTING.md and transcripts for run results.

| Screenshot requirement | Implementation / demo action | Evidence |
| --- | --- | --- |
| Executable Rust software; SQLite/libraries allowed | `cargo build --locked`; Ratatui terminal, rusqlite database | Build and real binary smoke run |
| Three user groups | Role menu; password+OTP for staff, PIN+OTP for voters | TUI signup/login journeys; cross-role endpoint tests |
| Admin-only interface | Server-issued admin session and DB role/ownership checks | `role_checks_and_cross_admin_ownership_are_enforced_in_services`; TLS tests |
| Create election | Admin → Create new election | Real TUI creates and persists election |
| At least three customizable positions | Defaults: President, Vice President, Secretary; P/E/D editor keys | TUI CRUD/reopen and minimum-position service tests |
| At least two candidates per position | Two defaults per position; C/E/D; cannot remove below two | TUI CRUD; `position_candidate_crud_and_linking_persist` |
| Candidate name and political party | Two editable fields; identity can be linked to multiple positions | TUI edits both fields and reads them after reopening |
| Register full name and DOB | R registration validates name/date; NID/phone also required for identity/delivery | TUI registration; invalid date/empty input service and validation tests |
| Detect existing voters | Unique NID and optional email checks; existing voters enroll without new PIN | TUI duplicate rejection; registration/enrollment service tests |
| Unique numeric PIN, delivered on registration | Random 20-digit PIN; private simulated inbox; stored verifier and unique keyed lookup | TUI delivery/login; `registration_generates_private_unique_pin_and_enrollment` |
| Log creation/open/close/registration with time and actor | Transactional audit events; admin View audit log | Real TUI plus SQLite assertions for four event types, timestamps and roles |
| Voter authenticates using PIN | PIN then OTP; retry limits | Real voter login; authentication tests |
| Vote only in open election | Eligibility list plus backend state check | TUI cannot select draft; service tests reject pre-open and invalid lifecycle requests |
| One vote per position | Whole ballot, exactly one candidate for each position | TUI incomplete rejection then three choices; service duplicate-position checks |
| Cast votes immutable | One final ballot per enrollment; DB update/delete triggers | Re-login has no eligible ballot; concurrent duplicate-vote and immutability tests |
| DO-only interface | Password+OTP; role-bound session/gateway | Real DO login; role/gateway rejection tests |
| Open existing election | DO S then confirm; locks editing | TUI open; timestamp/state and frozen-configuration tests |
| Close ongoing election | DO F then confirm | TUI close; DB status/closed_at checks |
| Tally and calculate winner per position | DO T; store candidate and party totals; handle ties/no votes | TUI tally, expected totals; service and renderer tests for edge cases |
| Display winners for all positions | DO A announces and displays results; R reopens results | TUI result screen, results data; UI tie/no-vote tests |

## Extra requirements from our threat model

Admin audit shows the participating voter but never their ballot, PIN or choices.
DO results show candidates/parties/totals but no voter identities. The terminal
walkthrough checks both screens; endpoint tests check direct unauthorized access.
Different gateway client certificates, password/OTP cooldowns, timeout handling,
error notification files and append-only logs remain in place; see STRIDE.md.

The PIN/OTP inbox and email files simulate delivery on a trusted local machine.
They are not a real SMS/email service or independent second device. Enrollment
checks local NID/email uniqueness; there is no government identity verification.
No code is presented as real-world election certification.
