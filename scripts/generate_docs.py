"""Build the Markdown inventory from Rust signatures and doc comments (stdlib only)."""
from pathlib import Path
import re
import ast
ROOT = Path(__file__).resolve().parents[1]
intro = '''# Implementation details — code map

Version 0.2.2. Generated signatures describe parameters and return types; summaries
come from source comments. `Result` errors propagate to the caller, and the TUI shows
a safe message. See STRIDE.md for authorization requirements and security boundaries.

## Folder contracts

| Folder/file | Responsibility and contract |
| --- | --- |
| src/ui | Screens, key events, forms and client validation. No direct production SQLite access. |
| src/network/client | Typed requests; tokens carry identity, not caller-supplied user IDs. |
| src/network | TLS framing, separate gateway identities, database dispatch and session lifecycle. |
| src/auth | Password/PIN verification, OTP, retry counters, local message delivery. |
| src/services | Role/ownership/state rules and transactional domain operations. |
| src/database | Idempotent schema setup, migrations and database constraints. |
| src/validation.rs | Shared format checks; frontend convenience and backend enforcement. |
| src/security.rs | Deadlines and structured security events, without raw credentials. |
| src/models.rs | Serialized role and domain data types. |
| tests | Real terminal workflow script; Rust tests live alongside the modules they test. |
| scripts | Regenerate this inventory after editing doc comments. |

## Database specification

`src/database/schema.sql` defines users, password/PIN credentials, elections, positions,
candidates, position assignments, enrollment, anonymous ballots/choices, candidate/party
results, audit/security events, retry counters, notification/email outboxes and system
settings. Foreign keys constrain election/position/candidate relationships. Triggers
freeze live configuration and prohibit accepted-vote/audit edits. Outboxes provide
retryable delivery, not a transaction spanning SQLite and the filesystem.

## Other files

Cargo.toml declares direct crates/features; Cargo.lock pins the resolved dependency tree.
README.md is the installation/run guide; STRIDE.md is the threat/control/test map;
dummy.md introduces the code; docs/DEMO.md addresses oral-exam questions 9–14;
docs/TESTING.md reports validation. `.env.example` lists configuration names without
real secrets. LICENSE records licensing. Documentation is kept in Markdown and
Rust source comments; generated HTML is not included.

## Function inventory

All Rust functions, including test helpers, appear below. Signatures are authoritative
for inputs and outputs; follow the source link for implementation and error branches.
Tests run only under `cfg(test)` and must not be confused with production endpoints.
'''
pattern=re.compile(r'(?m)^(\s*)(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)[\s\S]*?\{')
parts=[intro]
for path in sorted((ROOT/'src').rglob('*.rs')):
    text=path.read_text(); rel=path.relative_to(ROOT).as_posix()
    module=' '.join(line[3:].strip() for line in text.splitlines() if line.startswith('//!'))
    parts += [f'\n### [{rel}]({rel})\n\n{module}\n']
    for m in pattern.finditer(text):
        signature=' '.join(m.group(0).strip().removesuffix('{').strip().split())
        previous=text[:m.start()].splitlines(); comments=[]
        for line in reversed(previous):
            line=line.strip()
            if line.startswith('///'): comments.insert(0,line[3:].strip())
            elif line.startswith('#[') or not line: continue
            else: break
        summary=' '.join(comments) or ('Test: ' if 'test' in rel else 'Internal helper: ')+m.group(2).replace('_',' ')+'.'
        parts += [f'\n#### `{m.group(2)}`\n\n{summary}\n\n```rust\n{signature}\n```\n']
for rel in ['tests/terminal_smoke.py','scripts/generate_docs.py']:
    source=(ROOT/rel).read_text()
    parts.append(f"\n### [{rel}]({rel})\n\nPython support tool; requires only the standard library.\n")
    for node in ast.walk(ast.parse(source)):
        if isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef)):
            args=', '.join(a.arg for a in node.args.args)
            parts.append(f"\n- `{node.name}({args})`: {ast.get_docstring(node) or node.name.replace('_',' ')+'.'}\n")
(ROOT/'implementation_details.md').write_text(''.join(parts))
print('Wrote implementation_details.md')
