# Security scope and limits

This is a classroom voting-system implementation, not a certified election product.
The controls below are executable and tested; they do not establish production
security or coercion-resistant, end-to-end verifiable voting.

| STRIDE concern | Implemented control | Remaining boundary |
| --- | --- | --- |
| Spoofing | bcrypt passwords/PIN verifiers, random PINs, OTP expiry/retry limits, server-held sessions | Same-host virtual inbox is not an independent authentication factor; no national-ID verification |
| Tampering | TLS 1.3, gateway mTLS, role/owner checks, transactions, foreign keys and immutable ballot triggers | A privileged OS/database operator can bypass application controls |
| Repudiation | Actor/subject timestamps, append-only audit through the application, queued notifications | No remote signed log, independent witness or cryptographic nonrepudiation |
| Information disclosure | TLS on both hops; no identity column in ballots; no admin choice/result endpoint | DB file is not encrypted; small elections and ordering/timing correlation can reveal choices |
| Denial of service | Bounded frames/workers, connection/read/write deadlines, authentication cooldowns and transport-flood logging | No volumetric protection, per-IP distributed limiter or external uptime monitor |
| Elevation of privilege | Server-issued sessions, current role checks, exact gateway certificate binding and local engineer setup capability | Someone controlling the server OS can run setup or read its private files |

## TLS and deployment assumptions

Only TLS 1.3 is accepted. Clients check the configured CA and the `localhost` server
name. The database endpoint requires a trusted client certificate and recognizes only
the three generated gateway certificates. Each gateway uses a different client key.
Unknown CAs, wrong names, missing client certificates and role mismatches are tested.
No “accept invalid certificate” or plaintext fallback is provided.

Development certificates are generated with rcgen; no private keys are bundled in the
source ZIP. The saved CA signing key is discarded after generating the identities.
Rotate the whole development bundle and distribute the new public CA when necessary.
Do not distribute gateway/server keys to terminal users. The bundle directory and
keys use owner-only POSIX permissions. On other platforms, set equivalent ACLs.

The default classroom topology shares an OS process for the four listeners; it does
not provide process/container isolation. `--client` runs the UI separately and does
not open the database. Separate OS users/hosts and an operational PKI would be needed
for stronger deployment separation. The implementation binds loopback only.

Frames are limited to 1 MiB. Listeners allow at most 32 concurrent connections each;
TCP connect timeouts are three seconds. Handshake/request-reading idle timeouts are
five seconds, with a ten-second total request budget. The deadline is not reset for responses. Gateways clamp the forwarded deadline to
their remaining budget; the database checks it after acquiring its state lock.
A watchdog closes an accepted connection after ten seconds even when its worker is
still computing. SQLite progress callbacks interrupt long queries, and mutating
services check the deadline before commit. Each listener admits at most 64 connections
per second and at most 32 active workers. These are global per-service limits, not
proof of attacker identity or a source-IP firewall.
Both sides send TLS close_notify before closing their write side. Rejected TLS/protocol attempts are aggregated into
severity-3 events; more than ten accumulated rejections produce a severity-2 flood
event. This is an alert, not proof that an attack occurred, and not a per-IP ban.
The limits bound network work but cannot prevent every resource-exhaustion attack.

A timed-out mutation may already have committed on the server. The client does not
automatically retry mutations; refresh before retrying. Ballot duplicate protection
makes a repeated cast fail safely. Long SQLite statements are interrupted cooperatively; bcrypt itself is not preempted.
A deadline check prevents subsequent successful credential issuance or a late commit;
a commit that finished before timeout remains committed.

## Ballot privacy and audit

An application administrator can see enrollment and participation, but cannot query
ballot choices, PIN verifiers or results through endpoints. District officials see
aggregates. The service never returns raw credential tables. Ballots have random IDs
and no direct identity link. They are committed together with the participation flag.

This does not defeat transaction-order/timing correlation, filesystem inspection,
single-voter inference, collusion or a compromised host. An OS administrator who can
read the SQLite file can read choices despite application roles. Real unlinkability
would require a different cryptographic protocol and independent trust domains.

Audit triggers reject updates/deletes through normal SQL; an OS owner can drop the
triggers or replace the database. Audit notifications use an outbox and may duplicate
on a crash; they do not disappear from the underlying audit history when delivery fails.
PIN delivery happens before registration commit so a delivery failure rolls back the
registration. A crash between delivery and commit can leave an unusable PIN message;
filesystem delivery and SQLite cannot form a single atomic transaction here.

Do not put real voter information into tests. The supplied tests use temporary data,
synthetic accounts and a test-only lower bcrypt cost. Release paths use DEFAULT_COST.
