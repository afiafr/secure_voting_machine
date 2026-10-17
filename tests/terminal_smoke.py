#!/usr/bin/env python3
"""Drive the real terminal client and a separate TLS server on an isolated database.
Run after `cargo build`: python3 tests/terminal_smoke.py (POSIX/macOS/Linux).
Only Python's standard library is used. All credentials here are disposable tests.
"""
import hashlib
import os
from pathlib import Path
import pty
import select
import socket
import sqlite3
import struct
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

    def close(self):
        if self.proc.poll() is None:
            self.keys("\x03")
            self.proc.wait(timeout=15)
        os.close(self.master)
        assert self.proc.returncode == 0, self.output[-2000:]


def main():
    assert BINARY.exists(), "Run cargo build first."
    version = subprocess.run([str(BINARY), "--version"], check=True, capture_output=True, text=True)
    assert "secure_voting_machine 0.1.2" in version.stdout
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
            path = root / "sms" / (hashlib.sha256(phone.encode()).hexdigest() + ".txt")
            if not path.exists():
                return None
            values = [line[len(prefix):] for line in path.read_text().splitlines() if line.startswith(prefix)]
            return values[-1] if values else None

        # Provision both accounts by actual keyboard input through the TLS setup capability.
        terminal = Terminal(["--signup"], env, root)
        try:
            assert b"Staff Sign Up" in terminal.output
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
        print("PASS: terminal setup creates admin and district official via TLS")
        subprocess.run([str(BINARY), "--init-tls"], env=env, cwd=root, check=True, capture_output=True)
        server = subprocess.Popen([str(BINARY), "--serve"], env=env, cwd=root,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        client_env = env.copy()
        client_env.pop("APP_PEPPER")  # Remote clients have no password pepper.
        time.sleep(0.3)

        def login(role):
            t = Terminal(["--client"], client_env, root)
            phone = ["+15550000001", "+15550000002", "+15550000003"][role]
            old = inbox(phone, "Login OTP: ")
            t.keys("\x1b[B" * role + "\r")
            credential = (["admin@example.test\tpassword1234", "do@example.test\tpassword1234"][role]
                          if role < 2 else inbox(phone, "Voting PIN: "))
            t.keys(credential + "\r")
            code = wait_for(lambda: (c if (c := inbox(phone, "Login OTP: ")) != old else None), "OTP delivery")
            t.drain(0.3)
            t.keys(code + "\r")
            t.drain(0.3)
            return t

        try:
            t = login(0)
            try:
                assert b"Create new election" in t.output
                t.keys("\rTerminal Election\r")
                wait_for(lambda: query("SELECT election_id FROM elections"), "create election")
                t.drain(0.3)
                t.keys("r")  # Registration screen.
                t.keys("r")  # One unified Register action.
                t.keys("Terminal Voter\t2000-01-02\tSMOKE-NID\t+15550000003\tvoter@example.test\r")
                wait_for(lambda: query("SELECT user_id FROM voter_pin_credentials"), "register voter")
                assert inbox("+15550000003", "Voting PIN: ")
            finally:
                t.close()
            print("PASS: password + OTP login, election defaults, voter registration and PIN delivery")
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
                for index in range(3):
                    t.keys(" ")
                    if index < 2:
                        t.keys("\x1b[B\x1b[B")
                t.keys("s\r")
                wait_for(lambda: query("SELECT user_id FROM election_voters WHERE has_voted=1"), "cast ballot")
                assert query("SELECT COUNT(*) FROM ballot_choices")[0][0] == 3
            finally:
                t.close()
            print("PASS: district opens election; PIN + OTP voter submits one complete ballot")
            t = login(1)
            try:
                t.keys("\r")
                for command, status in [("f", "closed"), ("t", "tallied")]:
                    t.keys(command + "\r")
                    wait_for(lambda: query(f"SELECT election_id FROM elections WHERE status='{status}'"), status)
                t.keys("a\r")
                wait_for(lambda: query("SELECT election_id FROM elections WHERE announced_at IS NOT NULL"), "announce")
                assert query("SELECT SUM(votes) FROM candidate_results")[0][0] == 3
                assert query("PRAGMA foreign_key_check") == []
            finally:
                t.close()
            print("PASS: close, tally, announce and database integrity")
        finally:
            server.terminate()
            server.wait(timeout=10)
    print("All direct terminal smoke checks passed; temporary data removed.")


if __name__ == "__main__":
    main()
