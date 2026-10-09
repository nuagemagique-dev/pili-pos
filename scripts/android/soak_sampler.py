#!/usr/bin/env python3
"""
pili-pos - soak sampler and verdicts (P7-T5, P7D12)

Host side of the soak of the Android port. Python 3 standard library only.

    soak_sampler.py run    --dir RUN_DIR      sample the device until the driver ends
    soak_sampler.py report --dir RUN_DIR [--min-hours H]
                                              print 'SOAK | <check> | PASS|FAIL | <detail>'
                                              lines and 'SOAK | total | PASS|FAIL | ...'
    soak_sampler.py status --dir RUN_DIR      one line: the last checkpoint
    soak_sampler.py state  FILE set k=v ...   read and write the state.json of a run
    soak_sampler.py state  FILE get KEY

The run directory (target/android-soak/<run-id>/) holds state.json (written by
p7_soak.sh: serial, package, fdm_port, start, hours, day_minutes, ...), and what the
sampler writes: samples.jsonl, checkpoints.jsonl, fdm_journal.jsonl, journal_meta.json,
driver.jsonl (a mirror of <externalFilesDir>/soak/driver.jsonl on the device),
logcat_app.txt and report.txt.

Cadence of `run`: a sample every 60 s (app pid, dumpsys meminfo TOTAL PSS and Native
Heap, VmRSS and Threads of /proc/<pid>/status, open fds through run-as, free space of
/data, sizes of pos.db, its WAL and the backups directory, crash markers seen in logcat),
a poll of the FDM mock request log every 20 s (the mock keeps only 100 calls, new calls
are appended to fdm_journal.jsonl), a read of the driver file every 10 s, and a checkpoint
with the verdicts every 15 minutes. The run ends when the driver wrote its 'end' record,
when the app process is gone for 5 samples, on SIGTERM, or when the hours plus 30 minutes
have passed; it then writes a final sample, a final checkpoint and report.txt.

Verdicts (P7D12), in the order they are printed:
    pid-stable          one pid in every sample, the driver's
    no-crash            no 'FATAL EXCEPTION', 'ANR in', 'Fatal signal' or engine Fatal marker
    sales-signed        driver ok sales equal the signSale calls the FDM signed, 0 failures
    counters-continuous eventCounter per label and totalCounter each go up by exactly 1
    z-done              one Z per elapsed compressed day, none failed
    memory              slope of PSS and native heap after the first hour <= 5 MB/h
    threads             max minus baseline <= 20
    fds                 max minus baseline <= 50
    beats               largest gap between two heartbeats <= 120 s
    storage             free /data >= 1 GiB (db bytes per sale and backup bytes per Z reported)
    min-hours           the run covers at least --min-hours
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import urllib.request
from dataclasses import dataclass, field

sys.dont_write_bytecode = True

SAMPLE_EVERY_S = 60
JOURNAL_EVERY_S = 20
DRIVER_EVERY_S = 10
CHECKPOINT_EVERY_S = 900
TICK_S = 2
MISSING_LIMIT = 5
DEADLINE_SLACK_S = 1800
JOURNAL_SIZE = 100

SLOPE_LIMIT_MB_H = 5.0
SLOPE_WARMUP_S = 3600
SLOPE_MIN_SPAN_S = 3600
THREADS_LIMIT = 20
FDS_LIMIT = 50
BEAT_GAP_LIMIT_S = 120
FREE_MIN_KB = 1024 * 1024
BASELINE_SAMPLES = 3
IN_FLIGHT_MS = 5000

MARKER_FATAL = "FATAL EXCEPTION"
MARKER_ANR = "ANR in"
MARKER_SIGNAL = "Fatal signal"
MARKER_ENGINE = "state=Fatal"
FATAL_CONTEXT_LINES = 8


# ── small helpers ──────────────────────────────────────────────────────────────


def read_jsonl(path: str) -> list[dict]:
    """The JSON objects of a JSON-lines file; a missing file, a blank or a broken line is skipped."""
    rows: list[dict] = []
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                try:
                    value = json.loads(line)
                except ValueError:
                    continue
                if isinstance(value, dict):
                    rows.append(value)
    except OSError:
        return []
    return rows


def append_jsonl(path: str, row: dict) -> None:
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(json.dumps(row, sort_keys=True) + "\n")


def read_json(path: str) -> dict:
    try:
        with open(path, encoding="utf-8") as handle:
            value = json.load(handle)
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}


def write_json(path: str, value: dict) -> None:
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.replace(tmp, path)


def fmt_duration(seconds: float) -> str:
    seconds = max(0, int(seconds))
    return f"{seconds // 3600}:{seconds % 3600 // 60:02d}:{seconds % 60:02d}"


def median(values: list[float]) -> float:
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return float(ordered[middle])
    return (ordered[middle - 1] + ordered[middle]) / 2.0


# ── parsers of device output ──────────────────────────────────────────────────


def parse_meminfo(text: str) -> dict:
    """TOTAL PSS and Native Heap PSS in kB from `dumpsys meminfo <pid>` (both layouts of the TOTAL line)."""
    out: dict = {"pss_kb": None, "native_kb": None}
    match = re.search(r"TOTAL PSS:\s+(\d+)", text)
    if match is None:
        match = re.search(r"^\s*TOTAL\s+(\d+)", text, re.MULTILINE)
    if match is not None:
        out["pss_kb"] = int(match.group(1))
    match = re.search(r"^\s*Native Heap\s+(\d+)", text, re.MULTILINE)
    if match is not None:
        out["native_kb"] = int(match.group(1))
    return out


def parse_status(text: str) -> dict:
    """VmRSS (kB) and Threads from the text of /proc/<pid>/status."""
    out: dict = {"rss_kb": None, "threads": None}
    match = re.search(r"VmRSS:\s+(\d+)", text)
    if match is not None:
        out["rss_kb"] = int(match.group(1))
    match = re.search(r"Threads:\s+(\d+)", text)
    if match is not None:
        out["threads"] = int(match.group(1))
    return out


def parse_df(text: str) -> int | None:
    """The 'Available' column (kB) of the last line of `df -k`."""
    lines = [line for line in text.splitlines() if line.strip()]
    if not lines:
        return None
    fields = lines[-1].split()
    for index, value in enumerate(fields):
        if re.fullmatch(r"\d+%", value) and index >= 1 and fields[index - 1].isdigit():
            return int(fields[index - 1])
    return None


def parse_kv(text: str) -> dict:
    """`key=value` lines to a dict of ints (a non-numeric or empty value is left out)."""
    out: dict = {}
    for line in text.splitlines():
        key, sep, value = line.strip().partition("=")
        if sep and re.fullmatch(r"\d+", value.strip()):
            out[key.strip()] = int(value.strip())
    return out


# ── crash markers ─────────────────────────────────────────────────────────────


class MarkerScanner:
    """Finds the crash markers of the app in a stream of logcat lines (threadtime format)."""

    def __init__(self, package: str, pids: set[int] | None = None) -> None:
        self.package = package
        self.pids: set[int] = set(pids or ())
        self._fatal_left = 0
        self._fatal_line = ""

    def feed(self, line: str) -> str | None:
        """The marker text when [line] shows a crash of the app, else None."""
        marker = None
        if self._fatal_left > 0:
            self._fatal_left -= 1
            if f"Process: {self.package}" in line:
                self._fatal_left = 0
                marker = f"{MARKER_FATAL}: {self._fatal_line.strip()[:160]}"
        if MARKER_FATAL in line:
            self._fatal_left = FATAL_CONTEXT_LINES
            self._fatal_line = line
        if MARKER_ANR in line and self.package in line:
            marker = marker or line.strip()[:200]
        if MARKER_SIGNAL in line and self._is_ours(line):
            marker = marker or line.strip()[:200]
        if "PiliShell" in line and MARKER_ENGINE in line:
            marker = marker or line.strip()[:200]
        return marker

    def _is_ours(self, line: str) -> bool:
        if "dev.nuagemagique" in line or "pilipos" in line:
            return True
        return any(f"pid {pid} " in line or f"pid {pid}," in line for pid in self.pids)


def relevant_line(line: str, package: str) -> bool:
    """Whether a logcat line goes to logcat_app.txt."""
    return any(tag in line for tag in ("PiliSoak", "PiliShell", "AndroidRuntime", package, "pilipos", "Fatal signal", "ANR in"))


# ── statistics ────────────────────────────────────────────────────────────────


def slope_per_hour(points: list[tuple[float, float]]) -> float | None:
    """Least-squares slope of value against time. points: (seconds, value); the result is value units per hour."""
    count = len(points)
    if count < 2:
        return None
    mean_t = sum(t for t, _ in points) / count
    mean_v = sum(v for _, v in points) / count
    var = sum((t - mean_t) ** 2 for t, _ in points)
    if var == 0:
        return None
    cov = sum((t - mean_t) * (v - mean_v) for t, v in points)
    return cov / var * 3600.0


def largest_gap(times: list[float]) -> float:
    """The largest difference between two consecutive values of a sorted list (0 for fewer than two)."""
    ordered = sorted(times)
    return max((b - a for a, b in zip(ordered, ordered[1:])), default=0.0)


@dataclass
class FdmEvent:
    operation: str
    label: str
    event_counter: int
    total_counter: int
    pos_id: str
    ticket_no: int | None


def fdm_events(journal: list[dict]) -> list[FdmEvent]:
    """The signed events of the journal (Status calls and refusals have no fdmRef), each counted once."""
    seen: set[tuple] = set()
    events: list[FdmEvent] = []
    for call in journal:
        if call.get("operation") == "Status":
            continue
        response = call.get("response")
        data = response.get("data") if isinstance(response, dict) else None
        if not isinstance(data, dict):
            continue
        for name, result in data.items():
            if not isinstance(result, dict) or not isinstance(result.get("fdmRef"), dict):
                continue
            ref = result["fdmRef"]
            key = (result.get("posId"), ref.get("eventLabel"), ref.get("eventCounter"))
            if key in seen:
                continue
            seen.add(key)
            events.append(
                FdmEvent(
                    operation=name,
                    label=str(ref.get("eventLabel")),
                    event_counter=int(ref.get("eventCounter") or 0),
                    total_counter=int(ref.get("totalCounter") or 0),
                    pos_id=str(result.get("posId")),
                    ticket_no=result.get("posFiscalTicketNo"),
                )
            )
    return events


def counter_breaks(events: list[FdmEvent]) -> list[str]:
    """Where a counter does not go up by exactly 1: the total counter, and the event counter of each label."""
    problems: list[str] = []
    for pos_id in sorted({e.pos_id for e in events}):
        mine = [e for e in events if e.pos_id == pos_id]
        totals = sorted(e.total_counter for e in mine)
        for a, b in zip(totals, totals[1:]):
            if b != a + 1:
                problems.append(f"total {a} -> {b}")
        for label in sorted({e.label for e in mine}):
            counters = sorted(e.event_counter for e in mine if e.label == label)
            for a, b in zip(counters, counters[1:]):
                if b != a + 1:
                    problems.append(f"{label} {a} -> {b}")
    return problems


# ── verdicts ──────────────────────────────────────────────────────────────────


@dataclass
class Check:
    name: str
    ok: bool
    detail: str


@dataclass
class Run:
    """Everything a verdict reads, loaded from a run directory."""

    state: dict = field(default_factory=dict)
    samples: list[dict] = field(default_factory=list)
    driver: list[dict] = field(default_factory=list)
    journal: list[dict] = field(default_factory=list)
    meta: dict = field(default_factory=dict)

    @classmethod
    def load(cls, run_dir: str) -> "Run":
        return cls(
            state=read_json(os.path.join(run_dir, "state.json")),
            samples=read_jsonl(os.path.join(run_dir, "samples.jsonl")),
            driver=read_jsonl(os.path.join(run_dir, "driver.jsonl")),
            journal=read_jsonl(os.path.join(run_dir, "fdm_journal.jsonl")),
            meta=read_json(os.path.join(run_dir, "journal_meta.json")),
        )

    @property
    def live_samples(self) -> list[dict]:
        """Samples taken while the driver ran (the process restarts after the instrumentation ends)."""
        return [s for s in self.samples if not s.get("post_end")]

    @property
    def finished(self) -> bool:
        return any(r.get("type") == "end" for r in self.driver)

    def of_type(self, kind: str) -> list[dict]:
        return [r for r in self.driver if r.get("type") == kind]

    def driver_pid(self) -> int | None:
        for row in self.of_type("start"):
            if isinstance(row.get("pid"), int):
                return row["pid"]
        return None

    def now_device_s(self) -> float:
        """The latest device time (s) the run knows."""
        stamps = [s["dev_t"] for s in self.samples if isinstance(s.get("dev_t"), (int, float))]
        stamps += [r["ts"] / 1000.0 for r in self.driver if isinstance(r.get("ts"), (int, float))]
        return max(stamps, default=0.0)

    def span_s(self) -> float:
        times = [s["t"] for s in self.samples if isinstance(s.get("t"), (int, float))]
        return max(times) - min(times) if times else 0.0


def check_pid_stable(run: Run) -> Check:
    samples = run.live_samples
    if not samples:
        return Check("pid-stable", False, "no samples")
    pids = {s["pid"] for s in samples if s.get("pid")}
    missing = sum(1 for s in samples if not s.get("pid") and not s.get("adb_error"))
    expected = run.driver_pid()
    if len(pids) != 1:
        return Check("pid-stable", False, f"pids seen: {sorted(pids)} ({missing} samples without a process)")
    pid = next(iter(pids))
    if missing:
        return Check("pid-stable", False, f"pid {pid}, but {missing} samples found no process")
    if expected is not None and expected != pid:
        return Check("pid-stable", False, f"pid {pid} is not the driver's pid {expected}")
    return Check("pid-stable", True, f"pid {pid} in {len(samples)} samples")


def check_no_crash(run: Run) -> Check:
    markers = [m for s in run.samples for m in s.get("markers", [])]
    errors = [r for r in run.of_type("error")]
    if markers:
        return Check("no-crash", False, f"{len(markers)} marker(s), first: {markers[0]}")
    if errors:
        return Check("no-crash", False, f"{len(errors)} driver error(s), first: {errors[0].get('error')}")
    return Check("no-crash", True, f"0 markers in {len(run.samples)} samples, 0 driver errors")


def check_sales_signed(run: Run) -> Check:
    sales = run.of_type("sale")
    ok_count = sum(1 for r in sales if r.get("ok"))
    failed = len(sales) - ok_count
    signed = sum(1 for e in fdm_events(run.journal) if e.operation == "signSale")
    detail = f"driver ok {ok_count}, failed {failed}, FDM signSale {signed}"
    if failed:
        return Check("sales-signed", False, detail)
    if run.finished:
        return Check("sales-signed", signed == ok_count, detail)
    polled = run.meta.get("last_poll_dev_ms")
    pending = 0
    if isinstance(polled, (int, float)):
        pending = sum(1 for r in sales if r.get("ok") and r.get("ts", 0) >= polled - IN_FLIGHT_MS)
    low, high = ok_count - pending, ok_count + 1
    return Check("sales-signed", low <= signed <= high, f"{detail} (running: {pending} not yet in the journal)")


def check_counters(run: Run) -> Check:
    events = fdm_events(run.journal)
    if not events:
        return Check("counters-continuous", False, "no signed event in the FDM journal")
    problems = counter_breaks(events)
    overflows = int(run.meta.get("overflows") or 0)
    detail = f"{len(events)} events, total counter {min(e.total_counter for e in events)}..{max(e.total_counter for e in events)}"
    if overflows:
        problems.append(f"{overflows} journal poll(s) lost calls")
    if problems:
        return Check("counters-continuous", False, f"{detail}; {'; '.join(problems[:5])}")
    return Check("counters-continuous", True, detail)


def check_z_done(run: Run) -> Check:
    day_s = float(run.state.get("day_minutes") or 60) * 60.0
    z_rows = run.of_type("z")
    z_ok = sum(1 for r in z_rows if r.get("ok"))
    z_failed = len(z_rows) - z_ok
    days = sum(1 for r in run.of_type("day") if r.get("phase") == "begin")
    starts = run.of_type("start")
    if not starts:
        return Check("z-done", False, "no driver start record")
    elapsed = run.now_device_s() - starts[0]["ts"] / 1000.0
    full_days = int(elapsed // day_s)
    fdm_z = sum(1 for e in fdm_events(run.journal) if e.operation == "signReportTurnoverZ")
    detail = f"{z_ok} Z ok, {z_failed} failed, {days} days started, {full_days} full days elapsed, FDM Z signed {fdm_z}"
    if z_failed or fdm_z < z_ok:
        return Check("z-done", False, detail)
    if run.finished:
        return Check("z-done", z_ok >= full_days and z_ok == days, detail)
    return Check("z-done", z_ok >= full_days - 1, detail)


def _series(run: Run, key: str, after_s: float = 0.0) -> list[tuple[float, float]]:
    samples = run.live_samples
    if not samples:
        return []
    first = min(s["t"] for s in samples)
    return [(s["t"], float(s[key])) for s in samples if isinstance(s.get(key), (int, float)) and s["t"] - first >= after_s]


def check_memory(run: Run, warmup_s: float = SLOPE_WARMUP_S) -> Check:
    pss = _series(run, "pss_kb", warmup_s)
    native = _series(run, "native_kb", warmup_s)
    span = (pss[-1][0] - pss[0][0]) if len(pss) >= 2 else 0.0
    if span < SLOPE_MIN_SPAN_S:
        every = _series(run, "pss_kb")
        info = slope_per_hour(every)
        note = f"slope over the whole run {info / 1024:.1f} MB/h (information)" if info is not None else "too few samples"
        return Check("memory", True, f"not evaluated: {span / 60:.0f} min of samples after the first hour, {SLOPE_MIN_SPAN_S // 60} needed; {note}")
    pss_slope = slope_per_hour(pss)
    native_slope = slope_per_hour(native)
    if pss_slope is None or native_slope is None:
        return Check("memory", False, "no slope (missing meminfo samples)")
    pss_mb, native_mb = pss_slope / 1024.0, native_slope / 1024.0
    ok = pss_mb <= SLOPE_LIMIT_MB_H and native_mb <= SLOPE_LIMIT_MB_H
    return Check("memory", ok, f"PSS {pss_mb:+.2f} MB/h, native heap {native_mb:+.2f} MB/h over {span / 3600:.1f} h (limit {SLOPE_LIMIT_MB_H:g})")


def _growth(run: Run, key: str, limit: int, name: str) -> Check:
    values = [int(s[key]) for s in run.live_samples if isinstance(s.get(key), int)]
    if not values:
        return Check(name, False, f"no {key} samples")
    baseline = median(values[:BASELINE_SAMPLES])
    peak = max(values)
    return Check(name, peak - baseline <= limit, f"baseline {baseline:g}, max {peak}, growth {peak - baseline:g} (limit {limit})")


def check_threads(run: Run) -> Check:
    return _growth(run, "threads", THREADS_LIMIT, "threads")


def check_fds(run: Run) -> Check:
    return _growth(run, "fds", FDS_LIMIT, "fds")


def check_beats(run: Run) -> Check:
    stamps = [r["ts"] / 1000.0 for r in run.driver if r.get("type") in ("start", "beat") and isinstance(r.get("ts"), (int, float))]
    beats = sum(1 for r in run.driver if r.get("type") == "beat")
    if beats == 0:
        return Check("beats", False, "no heartbeat")
    gap = largest_gap(stamps)
    if not run.finished:
        gap = max(gap, run.now_device_s() - max(stamps))
    return Check("beats", gap <= BEAT_GAP_LIMIT_S, f"{beats} beats, largest gap {gap:.0f} s (limit {BEAT_GAP_LIMIT_S})")


def check_storage(run: Run) -> Check:
    samples = run.samples
    free = [int(s["free_kb"]) for s in samples if isinstance(s.get("free_kb"), int)]
    if not free:
        return Check("storage", False, "no df samples")
    sales_ok = sum(1 for r in run.of_type("sale") if r.get("ok"))
    z_ok = sum(1 for r in run.of_type("z") if r.get("ok"))
    dbs = [int(s["db_bytes"]) for s in samples if isinstance(s.get("db_bytes"), int)]
    backups = [int(s["backup_kb"]) for s in samples if isinstance(s.get("backup_kb"), int)]
    per_sale = f"{(dbs[-1] - dbs[0]) / sales_ok:.0f} B/sale" if dbs and sales_ok else "n/a"
    per_z = f"{backups[-1] * 1024 / z_ok:.0f} B/Z" if backups and z_ok else "n/a"
    low = min(free)
    detail = f"free /data min {low / 1048576:.2f} GiB (limit 1); pos.db growth {per_sale}; backups {per_z}"
    return Check("storage", low >= FREE_MIN_KB, detail)


def check_min_hours(run: Run, min_hours: float | None) -> Check:
    hours = run.span_s() / 3600.0
    if min_hours is None:
        return Check("min-hours", True, f"{hours:.2f} h covered, no minimum asked")
    return Check("min-hours", hours >= min_hours, f"{hours:.2f} h covered, {min_hours:g} h asked")


def evaluate(run: Run, min_hours: float | None = None, warmup_s: float = SLOPE_WARMUP_S) -> list[Check]:
    """Every verdict of P7D12, in report order."""
    return [
        check_pid_stable(run),
        check_no_crash(run),
        check_sales_signed(run),
        check_counters(run),
        check_z_done(run),
        check_memory(run, warmup_s),
        check_threads(run),
        check_fds(run),
        check_beats(run),
        check_storage(run),
        check_min_hours(run, min_hours),
    ]


def report_lines(checks: list[Check]) -> list[str]:
    """`SOAK | <check> | PASS|FAIL | <detail>` per check, then the total line."""
    lines = [f"SOAK | {c.name} | {'PASS' if c.ok else 'FAIL'} | {c.detail}" for c in checks]
    failed = sum(1 for c in checks if not c.ok)
    lines.append(f"SOAK | total | {'FAIL' if failed else 'PASS'} | {failed} failed ({len(checks)} checks)")
    return lines


def checkpoint_row(run: Run, checks: list[Check], now: float) -> dict:
    return {
        "t": now,
        "elapsed_s": round(run.span_s()),
        "ok": all(c.ok for c in checks),
        "checks": [{"check": c.name, "ok": c.ok, "detail": c.detail} for c in checks],
    }


# ── the device ────────────────────────────────────────────────────────────────


class Device:
    """The adb calls of the sampler. Tests replace this class with a fake that has the same methods."""

    def __init__(self, serial: str, package: str, adb: str) -> None:
        self.serial = serial
        self.package = package
        self.adb = adb
        self.soak_dir = f"/sdcard/Android/data/{package}/files/soak"

    def _run(self, args: list[str], timeout: int = 25) -> tuple[int, str]:
        try:
            done = subprocess.run(
                [self.adb, "-s", self.serial, *args],
                capture_output=True,
                timeout=timeout,
                check=False,
            )
        except (OSError, subprocess.SubprocessError):
            return 255, ""
        return done.returncode, done.stdout.decode("utf-8", errors="replace").replace("\r", "")

    def probe(self) -> dict | None:
        """pid, device clock, VmRSS, Threads and df of /data in one call; None when adb fails."""
        script = (
            f"set -- $(pidof {self.package}); pid=$1; echo pid=$pid; echo date=$(date +%s); "
            "if [ -n \"$pid\" ]; then grep -E '^(VmRSS|Threads):' /proc/$pid/status; fi; echo df=; df -k /data | tail -n 1"
        )
        code, out = self._run(["shell", script])
        if code != 0 or "date=" not in out:
            return None
        head, _, tail = out.partition("df=")
        kv = parse_kv(head)
        return {"pid": kv.get("pid"), "dev_t": kv.get("date"), **parse_status(head), "free_kb": parse_df(tail)}

    def meminfo(self, pid: int) -> dict:
        _, out = self._run(["shell", f"dumpsys meminfo {pid}"], timeout=40)
        return parse_meminfo(out)

    def app_files(self, pid: int) -> dict:
        """Open fds, pos.db and WAL sizes (bytes), backups size (kB) and file count, through run-as."""
        inner = (
            "echo db=$(stat -c %s files/engine/pos.db 2>/dev/null); echo wal=$(stat -c %s files/engine/pos.db-wal 2>/dev/null); "
            "echo backup_kb=$(du -sk files/engine/backups 2>/dev/null | cut -f1); "
            "echo backup_files=$(ls files/engine/backups 2>/dev/null | wc -l); "
            f"echo fds=$(ls /proc/{pid}/fd 2>/dev/null | wc -l)"
        )
        _, out = self._run(["shell", f"run-as {self.package} sh -c '{inner}'"])
        kv = parse_kv(out)
        return {
            "db_bytes": kv.get("db"),
            "wal_bytes": kv.get("wal"),
            "backup_kb": kv.get("backup_kb"),
            "backup_files": kv.get("backup_files"),
            "fds": kv.get("fds"),
        }

    def driver_tail(self, offset: int) -> bytes | None:
        """The bytes of driver.jsonl after [offset]; None when adb fails."""
        try:
            done = subprocess.run(
                [self.adb, "-s", self.serial, "exec-out", f"tail -c +{offset + 1} {self.soak_dir}/driver.jsonl 2>/dev/null"],
                capture_output=True,
                timeout=40,
                check=False,
            )
        except (OSError, subprocess.SubprocessError):
            return None
        return done.stdout if done.returncode == 0 else None

    def logcat_process(self) -> subprocess.Popen | None:
        try:
            return subprocess.Popen(
                [self.adb, "-s", self.serial, "logcat", "-v", "threadtime", "-T", "1"],
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                text=True,
                errors="replace",
            )
        except OSError:
            return None


class FdmMock:
    """The request log of the host FDM mock."""

    def __init__(self, port: int) -> None:
        self.url = f"http://127.0.0.1:{port}/mock/requests"

    def requests(self) -> list[dict] | None:
        try:
            with urllib.request.urlopen(self.url, timeout=5) as response:
                value = json.load(response)
        except (OSError, ValueError):
            return None
        return value if isinstance(value, list) else None


def call_key(call: dict) -> str:
    return json.dumps([call.get("at"), call.get("operation"), call.get("request")], sort_keys=True)[:4000]


# ── the sampler ───────────────────────────────────────────────────────────────


class LogcatTail(threading.Thread):
    """Reads logcat in the background, keeps the lines of the app and collects the crash markers."""

    def __init__(self, device: Device, scanner: MarkerScanner, out_path: str) -> None:
        super().__init__(daemon=True)
        self.device = device
        self.scanner = scanner
        self.out_path = out_path
        self.markers: list[str] = []
        self._lock = threading.Lock()
        self._stop = threading.Event()
        self._process: subprocess.Popen | None = None

    def take(self) -> list[str]:
        with self._lock:
            taken, self.markers = self.markers, []
        return taken

    def stop(self) -> None:
        self._stop.set()
        if self._process is not None:
            try:
                self._process.terminate()
            except OSError:
                pass

    def run(self) -> None:
        while not self._stop.is_set():
            self._process = self.device.logcat_process()
            if self._process is None or self._process.stdout is None:
                self._stop.wait(5)
                continue
            with open(self.out_path, "a", encoding="utf-8") as out:
                for line in self._process.stdout:
                    marker = self.scanner.feed(line)
                    if marker is not None:
                        with self._lock:
                            self.markers.append(marker)
                    if marker is not None or relevant_line(line, self.scanner.package):
                        out.write(line if line.endswith("\n") else line + "\n")
                        out.flush()
                    if self._stop.is_set():
                        break
            self._stop.wait(3)


class Sampler:
    def __init__(self, run_dir: str, device, mock: FdmMock, clock=time.time, sleep=time.sleep) -> None:
        self.dir = run_dir
        self.device = device
        self.mock = mock
        self.clock = clock
        self.sleep = sleep
        self.state = read_json(os.path.join(run_dir, "state.json"))
        self.package = str(self.state.get("package") or "dev.nuagemagique.pilipos.debug")
        self.hours = float(self.state.get("hours") or 0)
        self.warmup_s = float(self.state.get("warmup_s") or SLOPE_WARMUP_S)
        self.min_hours = self.state.get("min_hours")
        self.samples_path = os.path.join(run_dir, "samples.jsonl")
        self.checkpoints_path = os.path.join(run_dir, "checkpoints.jsonl")
        self.journal_path = os.path.join(run_dir, "fdm_journal.jsonl")
        self.meta_path = os.path.join(run_dir, "journal_meta.json")
        self.driver_path = os.path.join(run_dir, "driver.jsonl")
        self.offset = os.path.getsize(self.driver_path) if os.path.exists(self.driver_path) else 0
        self.seen: set[str] = set()
        self.meta: dict = {"polls": 0, "overflows": 0, "status_calls": 0, "last_poll_dev_ms": None}
        self.clock_offset = 0.0
        self.driver_ended = False
        self.missing = 0
        self.stop_requested = False
        self.scanner = MarkerScanner(self.package)
        self.tail: LogcatTail | None = None
        self.started = self.clock()

    # one step of each kind
    def poll_journal(self) -> None:
        """Appends the calls of the mock log that were not seen before; counts a full window with no known call."""
        calls = self.mock.requests()
        if calls is None:
            return
        known = set(self.seen)
        if len(calls) >= JOURNAL_SIZE and known and not any(call_key(c) in known for c in calls):
            self.meta["overflows"] += 1
        for call in calls:
            key = call_key(call)
            if key in self.seen:
                continue
            self.seen.add(key)
            if call.get("operation") == "Status":
                self.meta["status_calls"] += 1
            else:
                append_jsonl(self.journal_path, call)
        self.meta["polls"] += 1
        self.meta["last_poll_dev_ms"] = int((self.clock() + self.clock_offset) * 1000)
        write_json(self.meta_path, self.meta)

    def read_driver(self) -> None:
        data = self.device.driver_tail(self.offset)
        if not data:
            return
        end = data.rfind(b"\n")
        if end < 0:
            return
        chunk = data[: end + 1]
        with open(self.driver_path, "ab") as handle:
            handle.write(chunk)
        self.offset += len(chunk)
        for row in read_jsonl_bytes(chunk):
            if row.get("type") == "start" and isinstance(row.get("pid"), int):
                self.scanner.pids.add(row["pid"])
            if row.get("type") == "end":
                self.driver_ended = True

    def sample(self) -> dict:
        row: dict = {"t": self.clock(), "pid": None, "adb_error": False, "post_end": self.driver_ended}
        probe = self.device.probe()
        if probe is None:
            row["adb_error"] = True
        else:
            row.update({k: v for k, v in probe.items() if v is not None})
            row["pid"] = probe.get("pid")
            if isinstance(probe.get("dev_t"), int):
                self.clock_offset = probe["dev_t"] - row["t"]
        pid = row.get("pid")
        if pid:
            self.scanner.pids.add(pid)
            row.update({k: v for k, v in self.device.meminfo(pid).items() if v is not None})
            row.update({k: v for k, v in self.device.app_files(pid).items() if v is not None})
        row["markers"] = self.tail.take() if self.tail is not None else []
        if not row["post_end"]:
            if pid or row["adb_error"]:
                self.missing = 0
            else:
                self.missing += 1
        append_jsonl(self.samples_path, row)
        return row

    def checkpoint(self) -> list[Check]:
        run = Run.load(self.dir)
        checks = evaluate(run, self.min_hours, self.warmup_s)
        append_jsonl(self.checkpoints_path, checkpoint_row(run, checks, self.clock()))
        return checks

    def finish(self) -> list[Check]:
        self.poll_journal()
        self.read_driver()
        self.sample()
        checks = self.checkpoint()
        with open(os.path.join(self.dir, "report.txt"), "w", encoding="utf-8") as handle:
            handle.write("\n".join(report_lines(checks)) + "\n")
        return checks

    def run(self) -> int:
        deadline = self.started + self.hours * 3600 + DEADLINE_SLACK_S
        due = {"journal": 0.0, "driver": 0.0, "sample": 0.0, "checkpoint": self.started + CHECKPOINT_EVERY_S}
        self.tail = LogcatTail(self.device, self.scanner, os.path.join(self.dir, "logcat_app.txt"))
        self.tail.start()
        try:
            while not self.stop_requested:
                now = self.clock()
                if now >= due["journal"]:
                    self.poll_journal()
                    due["journal"] = now + JOURNAL_EVERY_S
                if now >= due["driver"]:
                    self.read_driver()
                    due["driver"] = now + DRIVER_EVERY_S
                if now >= due["sample"]:
                    self.sample()
                    due["sample"] = now + SAMPLE_EVERY_S
                if now >= due["checkpoint"]:
                    self.checkpoint()
                    due["checkpoint"] = now + CHECKPOINT_EVERY_S
                if self.driver_ended or self.missing >= MISSING_LIMIT or now >= deadline:
                    break
                self.sleep(TICK_S)
        finally:
            checks = self.finish()
            if self.tail is not None:
                self.tail.stop()
        return 0 if all(c.ok for c in checks) else 1


def read_jsonl_bytes(chunk: bytes) -> list[dict]:
    rows = []
    for line in chunk.decode("utf-8", errors="replace").splitlines():
        try:
            value = json.loads(line)
        except ValueError:
            continue
        if isinstance(value, dict):
            rows.append(value)
    return rows


# ── command line ──────────────────────────────────────────────────────────────


def find_adb() -> str:
    explicit = os.environ.get("ADB")
    if explicit:
        return explicit
    home = os.environ.get("ANDROID_HOME", os.path.expanduser("~/Library/Android/sdk"))
    candidate = os.path.join(home, "platform-tools", "adb")
    return candidate if os.path.exists(candidate) else (shutil.which("adb") or "adb")


def cmd_run(args: argparse.Namespace) -> int:
    state = read_json(os.path.join(args.dir, "state.json"))
    device = Device(str(state["serial"]), str(state.get("package") or "dev.nuagemagique.pilipos.debug"), find_adb())
    sampler = Sampler(args.dir, device, FdmMock(int(state["fdm_port"])))

    def stop(_signum, _frame) -> None:
        sampler.stop_requested = True

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    return sampler.run()


def cmd_report(args: argparse.Namespace) -> int:
    run = Run.load(args.dir)
    if not run.samples:
        print(f"SOAK | total | FAIL | no samples in {args.dir}")
        return 1
    warmup = float(run.state.get("warmup_s") or SLOPE_WARMUP_S)
    checks = evaluate(run, args.min_hours, warmup)
    lines = report_lines(checks)
    print("\n".join(lines))
    return 0 if all(c.ok for c in checks) else 1


def cmd_status(args: argparse.Namespace) -> int:
    rows = read_jsonl(os.path.join(args.dir, "checkpoints.jsonl"))
    if not rows:
        print("no checkpoint yet")
        return 0
    last = rows[-1]
    failing = [c["check"] for c in last.get("checks", []) if not c.get("ok")]
    verdict = "all checks pass" if last.get("ok") else "failing: " + ", ".join(failing)
    print(f"checkpoint at {fmt_duration(last.get('elapsed_s', 0))} of sampling: {verdict}")
    return 0


def cmd_state(args: argparse.Namespace) -> int:
    state = read_json(args.file)
    if args.action == "get":
        value = state.get(args.items[0])
        print("" if value is None else value)
        return 0
    for item in args.items:
        key, _, raw = item.partition("=")
        try:
            state[key] = json.loads(raw)
        except ValueError:
            state[key] = raw
    write_json(args.file, state)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="soak_sampler.py", description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("--dir", required=True)
    run.set_defaults(func=cmd_run)
    report = sub.add_parser("report")
    report.add_argument("--dir", required=True)
    report.add_argument("--min-hours", type=float, default=None)
    report.set_defaults(func=cmd_report)
    status = sub.add_parser("status")
    status.add_argument("--dir", required=True)
    status.set_defaults(func=cmd_status)
    state = sub.add_parser("state")
    state.add_argument("file")
    state.add_argument("action", choices=["set", "get"])
    state.add_argument("items", nargs="+")
    state.set_defaults(func=cmd_state)
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
