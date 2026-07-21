#!/usr/bin/env python3
"""Run a validation command with macOS-safe no-progress diagnostics and cleanup."""

from __future__ import annotations

import datetime as dt
import atexit
import os
import platform
import shlex
import signal
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path


STALL_EXIT_CODE = 124


def release_validation_target_lease() -> None:
    lease_text = os.environ.get("WAVECRATE_VALIDATION_TARGET_LEASE_DIR")
    owner_text = os.environ.get("WAVECRATE_VALIDATION_TARGET_LEASE_OWNER")
    identity_text = os.environ.get("WAVECRATE_VALIDATION_TARGET_LEASE_IDENTITY")
    if not lease_text or owner_text != str(os.getpid()) or not identity_text:
        return
    lease = Path(lease_text)
    pid_path = lease / "pid"
    try:
        expected = f"{owner_text}\t{identity_text}"
        if pid_path.read_text(encoding="utf-8").strip() != expected:
            return
        pid_path.unlink()
        lease.rmdir()
    except FileNotFoundError:
        pass
    except OSError as error:
        print(f"[validation_watchdog] unable to release target lease {lease}: {error}", file=sys.stderr)


@dataclass(frozen=True)
class Process:
    pid: int
    ppid: int
    cpu_seconds: float
    state: str
    command: str


def env_seconds(name: str, default: float) -> float:
    raw = os.environ.get(name)
    if raw is None:
        return default
    try:
        value = float(raw)
    except ValueError as error:
        raise SystemExit(f"[validation_watchdog] {name} must be numeric") from error
    if value <= 0:
        raise SystemExit(f"[validation_watchdog] {name} must be greater than zero")
    return value


def parse_cpu_time(raw: str) -> float:
    days = 0
    if "-" in raw:
        day_text, raw = raw.split("-", 1)
        days = int(day_text)
    fields = [float(part) for part in raw.split(":")]
    seconds = fields.pop()
    minutes = fields.pop() if fields else 0
    hours = fields.pop() if fields else 0
    return days * 86400 + hours * 3600 + minutes * 60 + seconds


def process_table() -> dict[int, Process]:
    result = subprocess.run(
        ["ps", "-axo", "pid=,ppid=,time=,state=,command="],
        check=False,
        capture_output=True,
        text=True,
    )
    processes: dict[int, Process] = {}
    for line in result.stdout.splitlines():
        fields = line.strip().split(None, 4)
        if len(fields) != 5:
            continue
        try:
            pid, ppid = int(fields[0]), int(fields[1])
            cpu_seconds = parse_cpu_time(fields[2])
        except ValueError:
            continue
        processes[pid] = Process(pid, ppid, cpu_seconds, fields[3], fields[4])
    return processes


def owned_processes(root_pid: int) -> dict[int, Process]:
    processes = process_table()
    owned: dict[int, Process] = {}
    pending = [root_pid]
    while pending:
        parent = pending.pop()
        process = processes.get(parent)
        if process is not None:
            owned[parent] = process
        children = [pid for pid, candidate in processes.items() if candidate.ppid == parent]
        pending.extend(pid for pid in children if pid not in owned)
    return owned


def progress_signature(processes: dict[int, Process]) -> tuple[frozenset[int], float]:
    return frozenset(processes), sum(process.cpu_seconds for process in processes.values())


def made_progress(
    previous: tuple[frozenset[int], float], current: tuple[frozenset[int], float]
) -> bool:
    previous_pids, previous_cpu = previous
    current_pids, current_cpu = current
    return previous_pids != current_pids or current_cpu >= previous_cpu + 0.05


def run_and_record(path: Path, command: list[str], deadline: float) -> None:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        path.write_text("diagnostic collection budget exhausted\n", encoding="utf-8")
        return
    try:
        result = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=min(20, remaining),
        )
        content = result.stdout + result.stderr
    except (OSError, subprocess.TimeoutExpired) as error:
        content = f"unable to run {shlex.join(command)}: {error}\n"
    path.write_text(content, encoding="utf-8")


def write_diagnostics(
    root_pid: int,
    command: list[str],
    processes: dict[int, Process],
    root: Path,
    collection_seconds: float,
) -> Path:
    deadline = time.monotonic() + collection_seconds
    timestamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    destination = root / f"{timestamp}-{root_pid}"
    destination.mkdir(parents=True, exist_ok=False)
    (destination / "command.txt").write_text(shlex.join(command) + "\n", encoding="utf-8")
    process_lines = [
        f"{item.pid}\t{item.ppid}\t{item.cpu_seconds:.2f}\t{item.state}\t{item.command}"
        for item in processes.values()
    ]
    (destination / "process-tree.tsv").write_text(
        "pid\tppid\tcpu_seconds\tstate\tcommand\n" + "\n".join(process_lines) + "\n",
        encoding="utf-8",
    )
    run_and_record(destination / "rustc-version.txt", ["rustc", "-Vv"], deadline)
    run_and_record(destination / "cargo-version.txt", ["cargo", "-Vv"], deadline)
    diagnostic_platform = os.environ.get(
        "WAVECRATE_VALIDATION_TEST_PLATFORM", platform.system()
    )
    if diagnostic_platform == "Darwin":
        run_and_record(destination / "macos-version.txt", ["sw_vers"], deadline)
        sample_seconds = int(
            float(os.environ.get("WAVECRATE_VALIDATION_SAMPLE_SECONDS", "3"))
        )
        sample_command = os.environ.get("WAVECRATE_VALIDATION_SAMPLE_COMMAND", "sample")
        if sample_seconds > 0:
            for item in processes.values():
                executable = Path(item.command.split(None, 1)[0]).name
                if executable not in {"cargo", "rustc", "clang", "cc", "ld"}:
                    continue
                sample_path = destination / f"sample-{item.pid}-{executable}.txt"
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    sample_path.write_text(
                        "diagnostic collection budget exhausted\n", encoding="utf-8"
                    )
                    break
                try:
                    subprocess.run(
                        [
                            sample_command,
                            str(item.pid),
                            str(sample_seconds),
                            "1",
                            "-file",
                            str(sample_path),
                        ],
                        check=False,
                        capture_output=True,
                        timeout=min(sample_seconds + 20, remaining),
                    )
                except (OSError, subprocess.TimeoutExpired) as error:
                    sample_path.write_text(f"unable to sample pid {item.pid}: {error}\n")
    return destination


def process_group_exists(process_group: int) -> bool:
    result = subprocess.run(
        ["ps", "-axo", "pgid=,state="], check=False, capture_output=True, text=True
    )
    for line in result.stdout.splitlines():
        fields = line.split()
        if len(fields) != 2:
            continue
        try:
            pgid = int(fields[0])
        except ValueError:
            continue
        if pgid == process_group and not fields[1].startswith("Z"):
            return True
    return False


def terminate_owned_group(child: subprocess.Popen[bytes], grace_seconds: float) -> None:
    if child.poll() is not None and not process_group_exists(child.pid):
        return
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    deadline = time.monotonic() + grace_seconds
    while time.monotonic() < deadline:
        if not process_group_exists(child.pid):
            return
        time.sleep(0.05)
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


def main() -> int:
    if len(sys.argv) < 2:
        print("Usage: run_with_progress_watchdog.py <command> [args...]", file=sys.stderr)
        return 2

    atexit.register(release_validation_target_lease)
    command = sys.argv[1:]
    idle_seconds = env_seconds("WAVECRATE_VALIDATION_IDLE_SECONDS", 300)
    diagnostic_grace = env_seconds("WAVECRATE_VALIDATION_DIAGNOSTIC_GRACE_SECONDS", 120)
    diagnostic_collection = env_seconds(
        "WAVECRATE_VALIDATION_DIAGNOSTIC_COLLECTION_SECONDS", 30
    )
    term_grace = env_seconds("WAVECRATE_VALIDATION_TERM_GRACE_SECONDS", 10)
    poll_seconds = env_seconds("WAVECRATE_VALIDATION_POLL_SECONDS", 5)
    diagnostics_root = Path(
        os.environ.get(
            "WAVECRATE_VALIDATION_DIAGNOSTICS_DIR",
            str(Path.cwd() / "target" / "validation-diagnostics"),
        )
    )

    interrupted_signal = 0
    child: subprocess.Popen[bytes] | None = None

    def record_signal(received: int, _frame: object) -> None:
        nonlocal interrupted_signal
        interrupted_signal = received

    signal.signal(signal.SIGINT, record_signal)
    signal.signal(signal.SIGTERM, record_signal)

    try:
        ready_file = os.environ.get("WAVECRATE_VALIDATION_TEST_PRESPAWN_READY_FILE")
        if ready_file:
            Path(ready_file).write_text("ready\n", encoding="utf-8")
            time.sleep(env_seconds("WAVECRATE_VALIDATION_TEST_PRESPAWN_SECONDS", 1))
        if interrupted_signal:
            return 128 + interrupted_signal

        child = subprocess.Popen(command, start_new_session=True)
        last_signature = progress_signature(owned_processes(child.pid))
        last_progress = time.monotonic()
        diagnostic_signature: tuple[frozenset[int], float] | None = None
        diagnostic_time = 0.0

        while child.poll() is None:
            if interrupted_signal:
                terminate_owned_group(child, term_grace)
                child.wait()
                return 128 + interrupted_signal

            time.sleep(poll_seconds)
            processes = owned_processes(child.pid)
            signature = progress_signature(processes)
            now = time.monotonic()
            if made_progress(last_signature, signature):
                last_progress = now
                diagnostic_signature = None
            last_signature = signature

            if now - last_progress < idle_seconds:
                continue
            if diagnostic_signature is None:
                destination = write_diagnostics(
                    child.pid,
                    command,
                    processes,
                    diagnostics_root,
                    diagnostic_collection,
                )
                print(
                    f"[validation_watchdog] no owned-process progress for {idle_seconds:g}s; "
                    f"diagnostics: {destination}",
                    file=sys.stderr,
                    flush=True,
                )
                diagnostic_signature = signature
                diagnostic_time = time.monotonic()
                continue
            if made_progress(diagnostic_signature, signature):
                last_progress = now
                diagnostic_signature = None
                continue
            if now - diagnostic_time >= diagnostic_grace:
                print(
                    f"[validation_watchdog] confirmed no progress for an additional "
                    f"{diagnostic_grace:g}s; terminating owned process group {child.pid}",
                    file=sys.stderr,
                    flush=True,
                )
                terminate_owned_group(child, term_grace)
                child.wait()
                return STALL_EXIT_CODE

        return child.returncode
    finally:
        if child is not None and process_group_exists(child.pid):
            print(
                f"[validation_watchdog] command exited with owned processes still active; "
                f"cleaning process group {child.pid}",
                file=sys.stderr,
                flush=True,
            )
            terminate_owned_group(child, term_grace)


if __name__ == "__main__":
    raise SystemExit(main())
