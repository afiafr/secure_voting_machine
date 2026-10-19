#!/usr/bin/env python3
"""Drive the real terminal client and a separate TLS server on an isolated database.
Run after `cargo build`: python3 tests/terminal_smoke.py (POSIX/macOS/Linux).
Only Python's standard library is used. All credentials here are disposable tests.
"""
import os
from pathlib import Path
import pty
import re
import select
import socket
import sqlite3
import struct
import sys
import subprocess
import tempfile
import termios
import fcntl
import time

PROJECT = Path(__file__).resolve().parents[1]
BINARY = PROJECT / "target/debug/secure_voting_machine"


def wait_for(check, description, timeout=15):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        value = check()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError("Timed out: " + description)


class Terminal:
    def __init__(self, args, env, cwd):
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 120, 0, 0))
        self.proc = subprocess.Popen([str(BINARY), *args], stdin=slave, stdout=slave,
                                     stderr=slave, env=env, cwd=cwd)
        os.close(slave)
        self.output = b""
        self.drain(0.4)

    def drain(self, seconds=0.15):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([self.master], [], [], 0.03)[0]:
                try:
                    self.output += os.read(self.master, 65536)
                except OSError:
                    break

    def keys(self, value):
        os.write(self.master, value.encode())
        self.drain()

    def screen(self):
        """Read the visible grid, accounting for Ratatui's changed-cell output.

        This handles the cursor/erase commands used in our fixed 120x36 test;
        it is not a general terminal emulator. Color and cursor modes are ignored.
        """
        cells = [[" "] * 120 for _ in range(36)]
        row = col = 0
        tokens = re.findall(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b[=>]|[^\x1b]",
                            self.output.decode("utf-8", errors="replace"))
        for token in tokens:
            if token.startswith("\x1b["):
                command, raw = token[-1], token[2:-1]
                if raw.startswith("?"):
                    continue
                args = [int(n or 0) for n in raw.split(";")]
                if command in "Hf":
                    row = (args[0] or 1) - 1
                    col = ((args[1] if len(args) > 1 else 1) or 1) - 1
                elif command == "J" and args[0] in (2, 3):
                    cells = [[" "] * 120 for _ in range(36)]
                elif command == "K" and 0 <= row < 36:
                    start = 0 if args[0] in (1, 2) else col
                    end = col + 1 if args[0] == 1 else 120
                    cells[row][start:end] = [" "] * (end - start)
            elif token == "\r":
                col = 0
            elif token == "\n":
                row += 1
            elif token.isprintable() and not token.startswith("\x1b"):
                if 0 <= row < 36 and 0 <= col < 120:
                    cells[row][col] = token
                col += 1
        return "\n".join("".join(line) for line in cells)

    def close(self):
        if sys.exc_info()[0] is not None:
            self.drain()
            print("Synthetic-test terminal screen:\n", self.screen())
        if self.proc.poll() is None:
            self.keys("\x03")
            self.proc.wait(timeout=15)
        os.close(self.master)
        assert self.proc.returncode == 0, self.output[-2000:]


def main():
    assert BINARY.exists(), "Run cargo build first."
    version = subprocess.run([str(BINARY), "--version"], check=True, capture_output=True, text=True)
    assert "secure_voting_machine 0.2.2" in version.stdout
    with tempfile.TemporaryDirectory(prefix="voting-terminal-") as folder:
        root = Path(folder)
        env = dict(os.environ, TERM="xterm-256color", APP_PEPPER="terminal-test-pepper-12345",
                   VOTING_DB=str(root / "voting.db"), VOTING_SMS_DIR=str(root / "sms"),
                   VOTING_TLS_DIR=str(root / "tls"))
        # Reserve a free adjacent range long enough to pick a non-default test port.
        for port in range(18440, 18540, 4):
            probes = []
            try:
                for offset in range(4):
                    probe = socket.socket()
                    probes.append(probe)
                    probe.bind(("127.0.0.1", port + offset))
                break
            except OSError:
                continue
            finally:
                for probe in probes:
                    probe.close()
        else:
            raise AssertionError("No available test ports")
        env["VOTING_PORT"] = str(port)

        def query(sql):
            with sqlite3.connect(env["VOTING_DB"]) as db:
                return db.execute(sql).fetchall()

        def inbox(phone, prefix):
            path = root / "sms" / (phone + ".txt")
            if not path.exists():
                return None
            values = [line[len(prefix):] for line in path.read_text().splitlines() if line.startswith(prefix)]
            return values[-1] if values else None

        # Provision both accounts by actual keyboard input through the TLS setup capability.
        terminal = Terminal(["--signup"], env, root)
        try:
            code=wait_for(lambda: inbox("engineer","Setup OTP: "),"engineer setup OTP")
            terminal.keys(code+"\n")
            terminal.drain(0.5)
            assert "Staff Sign Up" in terminal.screen()
            terminal.keys("\r")  # Empty signup must stay in the form.
            assert query("SELECT count(*) FROM users")[0][0] == 0
            assert terminal.proc.poll() is None  # TUI remains usable after rejection.
            for email, phone, official in [("admin@example.test", "+15550000001", False),
                                            ("do@example.test", "+15550000002", True)]:
                if official:
                    terminal.keys("\x1b[B" * 3 + "\r")
                terminal.keys(email + "\tpassword1234\tTest User\t" + phone + "\t")
                if official:
                    terminal.keys("\x1b[C")
                terminal.keys("\r")
                wait_for(lambda: query(f"SELECT id FROM users WHERE email='{email}'"), "account setup")
                terminal.drain(0.4)
        finally:
            terminal.close()
        print("PASS: empty signup rejected; engineer OTP and staff signup through real TUI/TLS")
        subprocess.run([str(BINARY), "--init-tls"], env=env, cwd=root, check=True, capture_output=True)
        server = subprocess.Popen([str(BINARY), "--serve"], env=env, cwd=root,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        client_env = env.copy()
        client_env.pop("APP_PEPPER")  # Remote clients have no password pepper.
        time.sleep(0.3)

        def login(role):
            t = Terminal(["--client"], client_env, root)
            def ready():
                t.drain(0.05)
                return "SECURE VOTING MACHINE" in t.screen()
            wait_for(ready, "client main screen")
            phone = ["+15550000001", "+15550000002", "+15550000003"][role]
            old = inbox(phone, "Login OTP: ")
            t.keys("\x1b[B" * role + "\r")
            credential = (["admin@example.test\tpassword1234", "do@example.test\tpassword1234"][role]
                          if role < 2 else inbox(phone, "Voting PIN: "))
            t.keys(credential + "\r")
            try:
                code = wait_for(lambda: (c if (c := inbox(phone, "Login OTP: ")) != old else None), "OTP delivery")
            except AssertionError:
                t.drain()
                print("Synthetic-test terminal output:", repr(t.output[-6000:]))
                t.close()
                raise
            t.drain(0.3)
            t.keys(code + "\r")
            t.drain(0.3)
            return t

        try:
            t = login(0)
            try:
                assert "Create new election" in t.screen()
                t.keys("\rTerminal Election\r")
                wait_for(lambda: query("SELECT election_id FROM elections"), "create election")
                t.drain(0.3)
                assert query("SELECT name FROM election_positions ORDER BY display_order") == [
                    ("President",), ("Vice President",), ("Secretary",)]
                assert query("SELECT count(*) FROM position_candidates")[0][0] == 6
                # Real editor CRUD: extra position, rename, candidate name/party and removal.
                t.keys("pTreasurer\r")
                wait_for(lambda: query("SELECT position_id FROM election_positions WHERE name='Treasurer'"), "add position")
                t.keys("e" + "\x7f" * len("President") + "Chair\r")
                wait_for(lambda: query("SELECT position_id FROM election_positions WHERE name='Chair'"), "rename position")
                t.keys("cThird Candidate\tGreen Party\r")
                wait_for(lambda: query("SELECT candidate_id FROM candidates WHERE full_name='Third Candidate'"), "add candidate")
                t.keys("\x1b[C" + "e" + "\x7f" * len("Hamberger Red") + "Red Burger\t" +
                       "\x7f" * len("Red Party") + "Student Party\r")
                wait_for(lambda: query("SELECT candidate_id FROM candidates WHERE full_name='Red Burger' AND political_party='Student Party'"), "edit candidate")
                t.keys("\x1b[B" * 2 + "d\r")  # Remove third candidate; keep two.
                wait_for(lambda: query("SELECT count(*) FROM candidates WHERE full_name='Third Candidate'")[0][0] == 0, "delete candidate")
                t.keys("\x1b[B" * 4 + "\x1b[D" + "d\r")  # Remove extra position.
                wait_for(lambda: query("SELECT count(*) FROM election_positions")[0][0] == 3, "delete position")
                t.keys("\x1b")
                t.keys("\x1b[B\r")  # Edit existing election.
                t.keys("\r")
                assert "Chair" in t.screen() and "Student Party" in t.screen()
                t.keys("r")  # Registration screen.
                t.keys("r")  # One unified Register action.
                t.keys("\r")  # Invalid empty identity must not register anyone.
                assert query("SELECT count(*) FROM voter_pin_credentials")[0][0] == 0
                t.keys("Terminal Voter\t2000-01-02\tSMOKE-NID\t+15550000003\tvoter@example.test\r")
                wait_for(lambda: query("SELECT user_id FROM voter_pin_credentials"), "register voter")
                pin = inbox("+15550000003", "Voting PIN: ")
                assert pin and len(pin) == 20 and pin.isascii() and pin.isdigit()
                assert pin.encode() not in t.output
                t.drain(0.5)  # Registration also reloads voters after its transaction commits.
                rejected = query("SELECT count(*) FROM security_events WHERE event_type='request_rejected'")[0][0]
                t.keys("rTerminal Voter\t2000-01-02\tSMOKE-NID\t+15550000003\tvoter@example.test\r")
                assert query("SELECT count(*) FROM voter_pin_credentials")[0][0] == 1
                wait_for(lambda: query("SELECT count(*) FROM security_events WHERE event_type='request_rejected'")[0][0] > rejected, "duplicate rejected by backend")
                t.drain(0.3)
                t.keys("\x1b")  # Dismiss duplicate registration form.
                t.keys("\x1b")  # Admin menu.
                t.keys("\x1b[B" * 3 + "\r")
                t.keys("\r")
                assert "voter_registered" in t.screen() and pin.encode() not in t.output
            finally:
                t.close()
            print("PASS: admin login, defaults, position/candidate CRUD, reopen, registration/PIN, duplicate rejection, audit")
            # Voter cannot select a draft election.
            t = login(2)
            try:
                t.keys("\r")
                assert "No election selected" in t.screen()
                assert query("SELECT count(*) FROM ballots")[0][0] == 0
            finally:
                t.close()
            print("PASS: voter cannot vote before election opens")
            t = login(1)
            try:
                t.keys("\r")
                t.keys("s\r")
                wait_for(lambda: query("SELECT election_id FROM elections WHERE status='open' AND started_at IS NOT NULL"), "open election")
            finally:
                t.close()
            t = login(2)
            try:
                t.keys("\r")
                t.keys("s")  # Incomplete ballot must not be submitted.
                assert query("SELECT count(*) FROM ballots")[0][0] == 0
                assert "Choose one candidate" in t.screen()
                for index in range(3):
                    t.keys(" ")
                    if index < 2:
                        t.keys("\x1b[B\x1b[B")
                t.keys("s\r")
                wait_for(lambda: query("SELECT user_id FROM election_voters WHERE has_voted=1"), "cast ballot")
                assert query("SELECT COUNT(*) FROM ballot_choices")[0][0] == 3
            finally:
                t.close()
            # The same voter logs back in, but this election is no longer eligible.
            t = login(2)
            try:
                t.keys("\r")
                assert "No election selected" in t.screen()
                assert query("SELECT count(*) FROM ballots")[0][0] == 1
                assert query("SELECT count(*) FROM ballot_choices")[0][0] == 3
            finally:
                t.close()
            print("PASS: open election, incomplete ballot rejected, one choice per position, repeat vote unavailable")
            t = login(1)
            try:
                t.keys("\r")
                for command, status in [("f", "closed"), ("t", "tallied")]:
                    t.keys(command + "\r")
                    wait_for(lambda: query(f"SELECT election_id FROM elections WHERE status='{status}'"), status)
                t.keys("a\r")
                wait_for(lambda: query("SELECT election_id FROM elections WHERE announced_at IS NOT NULL"), "announce")
                assert query("SELECT SUM(votes) FROM candidate_results")[0][0] == 3
                assert "Winner: Red Burger" in t.screen()
                assert "Terminal Voter" not in t.screen()
                assert query("PRAGMA foreign_key_check") == []
                for event, role in [("election_created", "admin"), ("voter_registered", "admin"),
                                    ("election_opened", "district_official"), ("election_closed", "district_official")]:
                    rows = query(f"SELECT u.role, a.occurred_at FROM audit_events a JOIN users u ON u.id=a.actor_user_id WHERE event_type='{event}'")
                    assert rows and all(r == role and timestamp for r, timestamp in rows)
            finally:
                t.close()
            t = login(0)
            try:
                t.keys("\x1b[B" * 3 + "\r")
                t.keys("\r")
                assert "voter_participated" in t.screen() and "Terminal Voter" in t.screen()
                assert "Red Burger" not in t.screen() and "Student Party" not in t.screen()
            finally:
                t.close()
            print("PASS: close/tally/winners, timestamped actor logs, admin participation privacy, DO anonymity, DB integrity")
        finally:
            server.terminate()
            server.wait(timeout=10)
    print("All direct terminal smoke checks passed; temporary data removed.")


if __name__ == "__main__":
    main()
