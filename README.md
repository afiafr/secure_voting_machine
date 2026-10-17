# Secure-Ish Voting Machine

[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![TLS](https://img.shields.io/badge/TLS-1.3-blue)](docs/SECURITY.md)

**A terminal-based election system built with Rust, Ratatui, SQLite, and rustls.**

Create elections, register voters, cast immutable ballots, and announce results.
Administrator and District Official accounts use password + OTP login; voters use
PIN + OTP login. All client operations pass through encrypted role endpoints.

> **Classroom project:** OTP delivery uses a local virtual inbox. Network services
> bind to localhost. See [Security scope](docs/SECURITY.md) for the trust boundaries.

[Installation](#installation) · [Staff signup](#sign-up-administrators-and-district-officials) ·
[OTP & PIN](#otp-and-pin-delivery) · [Workflows](#role-workflows) ·
[Tests](#tests) · [Architecture](docs/IMPLEMENTATION.md)

## Features

- Separate Administrator, District Official, and Voter interfaces.
- Persistent election, position, candidate, and voter management.
- One final ballot per enrolled voter, with one candidate per position.
- Password/PIN verification followed by expiring, single-use OTPs.
- TLS 1.3 client connections and mutual TLS between gateways and the database service.
- Lifecycle and participation audit logs, without exposing voter choices to administrators.
- Candidate and party tallies, including ties and zero-vote results.

## Installation

### 1. Install prerequisites

| Requirement | Installation / notes |
| --- | --- |
| Rust and Cargo | Install the stable toolchain using the [official Rust installer](https://rust-lang.org/learn/get-started/) |
| C compiler / linker | Needed by bundled SQLite and cryptographic dependencies; see commands below |
| Python 3 | Used below to generate a secret and by the optional terminal smoke test |
| Terminal | At least 60 columns × 20 rows; 120 × 36 is comfortable |

<details>
<summary><strong>macOS prerequisites</strong></summary>

Install Apple's command-line developer tools if they are not already installed:

```sh
xcode-select --install
```

Install Rust using the official installer above, and install Python 3 if
`python3 --version` is unavailable.

</details>

<details>
<summary><strong>Ubuntu / Debian prerequisites</strong></summary>

```sh
sudo apt update
sudo apt install build-essential curl python3
```

Install Rust using the official installer above. These package commands apply to
Ubuntu/Debian; other distributions use their own package manager.

</details>

<details>
<summary><strong>Windows notes</strong></summary>

The commands below use a POSIX shell. You can follow the Linux instructions inside
WSL. For native Windows, follow the official Rust installer's MSVC/C++ prerequisites.
The Rust code has been validated on Linux; native Windows and macOS builds have not
been verified. The supplied Python terminal smoke test requires POSIX (macOS/Linux).

</details>

Open a new terminal after installation and check:

```sh
rustc --version
cargo --version
python3 --version
```

### 2. Get the source

Download and extract the project ZIP, then open the extracted project:

```sh
cd secure_voting_machine
```

If using a GitHub checkout, open the repository directory containing `Cargo.toml`.

### 3. Build

```sh
cargo build --locked
```

Cargo downloads the free, open-source dependencies. SQLite is bundled; there is no
separate SQLite server to install. The resulting executable is
`target/debug/secure_voting_machine` (`.exe` on native Windows).

### 4. Configure the password pepper

From the project directory, create `.env` **once**:

```sh
umask 077
python3 -c 'import secrets; open(".env", "x").write("APP_PEPPER=" + secrets.token_hex(16) + "\n")'
```

This creates a random 32-character secret and refuses to overwrite an existing file.
Keep `.env` private and preserve its value for this database. It is excluded by
`.gitignore`. Configuration examples are in [`.env.example`](.env.example).

The application accepts a 16–40 byte pepper. Passwords must be at least 10
characters; password plus pepper must fit bcrypt's 72-byte limit.

### 5. Create staff accounts and launch

```sh
cargo run --locked -- --signup
```

Create your Administrator and District Official accounts using the instructions
below. For subsequent ordinary use, start with:

```sh
cargo run --locked
```

The default mode starts four local listeners on automatically assigned ports and the
terminal client in one process. Every client operation still uses TLS through a role
gateway, followed by mutual TLS to the database service. No certificate setup or paid
SMS subscription is required for this mode.

## Sign up Administrators and District Officials

Run the dedicated local signup screen:

```sh
cargo run --locked -- --signup
```

1. Enter an **email**, **password**, **full name**, and **phone number**.
2. Press **Tab** to reach the role selector.
3. Use **Left/Right** to choose **Administrator** or **District Official**.
4. Press **Enter** to create the account.
5. To add another account, select **Sign up admin / district official** from the menu.
6. Select the corresponding login role, enter the new email/password, then complete OTP.

| Field | Requirement |
| --- | --- |
| Email | Required; unique across accounts |
| Password | At least 10 characters; stored as a bcrypt hash |
| Full name | Required |
| Phone | Required; used as the virtual OTP inbox address |
| Role | Administrator (`admin`) or District Official (`district_official`) |

Signup saves the account and credential hash in the database; it does **not** log the
new account in or skip OTP. Duplicate email addresses are rejected.
There are no built-in credentials.

**Who can sign up staff?** This is an explicit local engineer/setup action.
It is not exposed to normal clients, voters, or the standalone server. The older
`--setup` command is also supported and opens the main menu with the signup option.
Voters are registered separately by an authenticated administrator using **R**.

## OTP and PIN delivery

The project stays entirely in software: delivery uses virtual phone inboxes.
After entering a password or PIN, open another terminal in the project directory:

```sh
cargo run --locked -- --inbox YOUR_REGISTERED_PHONE
```

Enter the latest six-digit OTP in the voting terminal. OTPs expire after five
minutes, allow three failures before cooldown, and are consumed on success.
PINs are generated during voter registration and delivered to that voter's inbox.
They are not displayed in the administrator's registration screen or audit log.
A PIN identifies a voter account; a separate random ballot ID identifies a ballot.

The inbox simulates delivery; it is **not independent real-world MFA** when every
user shares the same operating-system account. No paid SMS service is needed.

## Separate terminal client and server

Generate development certificates once; the command refuses to overwrite an
existing certificate directory:

```sh
cargo run --locked -- --init-tls
cargo run --locked -- --serve
```

In another terminal:

```sh
cargo run --locked -- --client
```

The server starts the database listener on `127.0.0.1:7440`, Administrator gateway
on `7441`, District Official gateway on `7442`, and Voter gateway on `7443`.
The client needs only `tls-dev/ca.der`, not private keys, a database, or the pepper.
All listeners deliberately bind to loopback. This is a local demonstration, not
an Internet deployment. The role gateways and database listener are separate
network endpoints hosted by one server process.

| Environment variable | Default | Purpose |
| --- | --- | --- |
| `APP_PEPPER` | required on server | Password/PIN hashing secret; preserve it |
| `VOTING_DB` | `voting.db` | Server SQLite file |
| `VOTING_SMS_DIR` | `virtual_sms` | Virtual delivery directory |
| `VOTING_TLS_DIR` | `tls-dev` | Development identities and CA |
| `VOTING_PORT` | `7440` | Base port; gateways use the next three ports |

Ordinary combined mode uses ephemeral certificates and random available ports;
`--serve` / `--client` use the saved certificates and configured base port.

## Role workflows

**Administrator:** After login, choose Create new election, Edit existing election,
Register a voter, or View audit log. Login itself does not create an election.
Creating an election adds President, Vice President and Secretary, with Hamberger
Red and Blue Sandwich as default candidates for each. Editing loads owned elections,
positions and candidates from the database.

| Screen | Controls |
| --- | --- |
| General | Arrow keys; Enter to select; Esc to go back; Ctrl+C to exit |
| Election editor | P add position; C add candidate; E edit selected field; D delete/unlink |
| Election editor | L link an existing candidate ID to a position; F2 rename election; X cancel draft |
| Election editor | R open voter registration |
| Voter registration | R register; I enroll an existing voter by NID; E edit; D remove draft enrollment |
| Audit log | R refresh; F flag an unidentified change; Up/Down scroll |
| District Official | S start; F finish/close; T tally; A announce; R show results |
| Voter ballot | Up/Down navigate; Space select; S submit; Enter confirm |

Voter registration requires full name, DOB (`YYYY-MM-DD`), NID and phone; email is
optional. NID identifies a person across elections. Use I to enroll that person in
another election without changing their PIN. There is only one registration action,
R; N no longer marks or creates a voter.

Only draft elections can be edited. At least three positions and two candidates
per position are preserved. Registration/enrollment is allowed while draft or open;
removal and identity edits are restricted to unused draft enrollments. One candidate
can stand for several positions in the same election. Editing that candidate changes
all their appearances.

A District Official opens a draft, closes an open election, tallies it and announces
results. Tallying stores candidate totals and party totals **per position**. Repeating
a tally cannot add votes twice. Ties and positions with no votes are shown explicitly.

A voter sees open elections in which they are enrolled and have not voted. They
select exactly one candidate per position, then confirm the whole ballot. Submission
is atomic and final; partial submissions, duplicate votes and later edits are denied.
The administrator can see who participated, but has no endpoint for ballot choices
or results. The official can see tallies without voter identities attached to ballots.

## Tests

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --locked
python3 tests/terminal_smoke.py
```

Unit/service tests use isolated SQLite databases and disposable virtual inboxes.
The UI tests simulate keyboard events and render with Ratatui's test backend, while
using real TLS endpoints. Network tests exercise certificate rejection, mutual TLS,
OTP limits/expiry/replay, session expiry, role binding, and complete voting flows.
The Python smoke test drives the actual executable in a pseudo-terminal, starts a
separate server process and checks the resulting database. It needs POSIX
(macOS/Linux) and Python 3, without extra Python packages.

## Source layout

- `src/main.rs`: startup and command-line modes.
- `src/ui/`: application state, keyboard actions, forms, rendering and UI tests.
- `src/auth/`: credential verification, OTP, throttling and virtual delivery.
- `src/database/`: initialization, schema, constraints and migrations.
- `src/models.rs`: domain records transferred between components.
- `src/services/`: election/voter rules, transactions, authorization and database tests.
- `src/network/client/`: typed endpoint calls used by the terminal.
- `src/network/`: wire records, TLS, listeners, server sessions, dispatch and tests.
- `src/test_support.rs`: isolated test fixtures; excluded from release builds.

The uploaded prototype's `users.db`, `.env` and build output are not distributed.
The default `voting.db` avoids changing an old database accidentally. See the
migration notes before explicitly pointing `VOTING_DB` at existing data.

## Troubleshooting

| Problem | What to check |
| --- | --- |
| `cargo` is not found | Restart the terminal after installing Rust; check the installer updated your PATH |
| C compiler/linker error | Install the platform build tools listed under Installation |
| `APP_PEPPER` is missing | Create `.env` in the directory from which you start the program |
| Login fails after changing `.env` | Restore the original pepper for that database |
| No OTP or PIN visible | Run `--inbox` with exactly the registered phone number and the same `VOTING_SMS_DIR` |
| Email already exists | Sign in to that account or use a distinct email; signup does not overwrite accounts |
| Screen says to resize | Increase the terminal to at least 60 × 20 |
| TLS `close_notify` / unexpected EOF during signup | Stop old processes; verify version **0.1.2** using `cargo run --locked -- --version`, then run `cargo run --locked -- --signup` from this source directory. Keep your database and `.env`. See [validation report](docs/TESTING.md). If signup may have committed, try logging in before repeating it |
| Separate client cannot connect | Start `--serve`; use the same base port and public CA certificate |
| Port already in use | Set a free `VOTING_PORT`; the following three ports must also be free |

## Documentation

- [Implementation and database design](docs/IMPLEMENTATION.md)
- [Security controls and limitations](docs/SECURITY.md)
- [Test coverage and validation report](docs/TESTING.md)
- [MIT license](LICENSE)

### If a TLS signup error continues

Version 0.1.2 explicitly restores blocking mode on accepted TLS sockets for
macOS/BSD compatibility. It also sends TLS shutdown notifications and allows 60 seconds for responses
while retaining short inbound request limits. It was tested on Linux, including an
11-second delayed response and real terminal signup at normal bcrypt cost.

From the extracted project directory, run:

```sh
cargo run --locked -- --version
cargo run --locked -- --signup
```

The first command must show `secure_voting_machine 0.1.2`. Stop any older server/client
processes before restarting. Do not remove your database or regenerate its pepper.
If an error continues, record your OS, exact launch command and the full error text.
The error now names the gateway/database endpoint and identifies whether failure
occurred during connection, handshake/request sending, or response receipt. Do not
include your password, PIN, OTP or `.env` contents in a bug report.
