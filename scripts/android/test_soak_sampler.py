#!/usr/bin/env python3
"""
Self-test of scripts/android/soak_sampler.py (P7-T5). Python 3 standard library only, no device.

    python3 scripts/android/test_soak_sampler.py

Every case works on synthetic samples, driver records and FDM journals (or on the real-format
fixtures of scripts/android/testdata/soak_sampler/): the least-squares slope, the heartbeat gaps,
the FDM counter continuity, the crash markers, a pid change, the verdicts of P7D12, the report
and checkpoint lines, the journal poll with its overflow detection, and one whole sampler run
against a fake device and a fake FDM mock.
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import soak_sampler as ss  # noqa: E402

DATA = os.path.join(HERE, "testdata", "soak_sampler")
T0 = 1_800_000_000.0
PACKAGE = "dev.nuagemagique.pilipos.debug"


def fixture(name: str) -> str:
    with open(os.path.join(DATA, name), encoding="utf-8") as handle:
        return handle.read()


def fixture_calls() -> list[dict]:
    with open(os.path.join(DATA, "fdm_calls.json"), encoding="utf-8") as handle:
        return json.load(handle)


def sample(i: int, **over) -> dict:
    row = {
        "t": T0 + i * 60,
        "dev_t": T0 + i * 60,
        "pid": 4321,
        "adb_error": False,
        "post_end": False,
        "pss_kb": 200_000,
        "native_kb": 50_000,
        "rss_kb": 300_000,
        "threads": 40,
        "fds": 100,
        "free_kb": 6_000_000,
        "db_bytes": 300_000,
        "backup_kb": 0,
        "markers": [],
    }
    row.update(over)
    return row


def driver_rows(minutes: int, ok_sales: int = 0, z_ok: int = 0, days: int = 0, end: bool = True, beat_every: int = 10) -> list[dict]:
    rows = [{"type": "start", "ts": T0 * 1000, "pid": 4321}]
    for second in range(beat_every, minutes * 60 + 1, beat_every):
        rows.append({"type": "beat", "ts": (T0 + second) * 1000, "n": second // beat_every})
    for index in range(ok_sales):
        rows.append({"type": "sale", "ts": (T0 + 5 + index) * 1000, "ok": True, "kind": "takeaway"})
    for index in range(days):
        rows.append({"type": "day", "phase": "begin", "ts": (T0 + index) * 1000})
    for index in range(z_ok):
        rows.append({"type": "z", "ok": True, "ts": (T0 + 100 + index) * 1000})
    if end:
        rows.append({"type": "end", "ts": (T0 + minutes * 60) * 1000, "reason": "hours"})
    return rows


def sign(op: str, label: str, ec: int, tc: int, ticket: int = 0) -> dict:
    result = {"posId": "P1", "posFiscalTicketNo": ticket or ec, "fdmRef": {"eventLabel": label, "eventCounter": ec, "totalCounter": tc}}
    return {"at": f"a{tc}", "operation": op, "request": {"n": tc}, "response": {"data": {op: result}}}


def journal(sales: int, zs: int = 0) -> list[dict]:
    calls, tc = [], 0
    for index in range(sales):
        tc += 1
        calls.append(sign("signSale", "N", index + 1, tc))
    for index in range(zs):
        tc += 1
        z = sign("signReportTurnoverZ", "R", 2 * index + 1, tc)
        tc += 1
        z["response"]["data"]["signReportUserZ"] = sign("signReportUserZ", "R", 2 * index + 2, tc)["response"]["data"]["signReportUserZ"]
        calls.append(z)
    return calls


def make_run(samples=None, driver=None, calls=None, meta=None, **state) -> ss.Run:
    base = {"day_minutes": 10, "hours": 0.34}
    base.update(state)
    return ss.Run(state=base, samples=samples or [], driver=driver or [], journal=calls or [], meta=meta or {})


def by_name(checks: list[ss.Check]) -> dict:
    return {c.name: c for c in checks}


class Parsers(unittest.TestCase):
    def test_meminfo_total_and_native_heap(self):
        out = ss.parse_meminfo(fixture("meminfo.txt"))
        self.assertEqual(out["pss_kb"], 287601)
        self.assertEqual(out["native_kb"], 32619)

    def test_meminfo_old_layout_total_row(self):
        text = "  Native Heap     1234     0\n        TOTAL    9999  8888\n"
        self.assertEqual(ss.parse_meminfo(text), {"pss_kb": 9999, "native_kb": 1234})

    def test_status_df_and_kv(self):
        self.assertEqual(ss.parse_status(fixture("status.txt")), {"rss_kb": 412344, "threads": 41})
        self.assertEqual(ss.parse_df(fixture("df.txt")), 6504244)
        self.assertEqual(ss.parse_kv("pid=12\nwal=\nfds=7\nnoise\n"), {"pid": 12, "fds": 7})


class Statistics(unittest.TestCase):
    def test_slope_of_a_line_is_exact(self):
        points = [(i * 60.0, 1000.0 + 2048.0 * i / 60.0) for i in range(0, 181)]
        self.assertAlmostEqual(ss.slope_per_hour(points), 2048.0, places=6)

    def test_slope_is_none_without_variance(self):
        self.assertIsNone(ss.slope_per_hour([(1.0, 5.0)]))
        self.assertIsNone(ss.slope_per_hour([(1.0, 5.0), (1.0, 9.0)]))
        self.assertEqual(ss.slope_per_hour([(0.0, 7.0), (60.0, 7.0), (120.0, 7.0)]), 0.0)

    def test_largest_gap(self):
        self.assertEqual(ss.largest_gap([30, 0, 10, 20]), 10)
        self.assertEqual(ss.largest_gap([0, 10, 130]), 120)
        self.assertEqual(ss.largest_gap([5]), 0)

    def test_fdm_events_are_unique_and_skip_status_and_refusals(self):
        events = ss.fdm_events(fixture_calls() + fixture_calls())
        self.assertEqual([(e.operation, e.total_counter) for e in events], [("signWorkIn", 1), ("signSale", 2), ("signSale", 3), ("signReportTurnoverZ", 4), ("signReportUserZ", 5)])

    def test_counters_continuous_on_the_fixture(self):
        self.assertEqual(ss.counter_breaks(ss.fdm_events(fixture_calls())), [])

    def test_counter_breaks_name_the_gap(self):
        calls = [sign("signSale", "N", 1, 1), sign("signSale", "N", 3, 2), sign("signWorkIn", "S", 1, 4)]
        problems = ss.counter_breaks(ss.fdm_events(calls))
        self.assertIn("N 1 -> 3", problems)
        self.assertIn("total 2 -> 4", problems)


class Markers(unittest.TestCase):
    def test_markers_of_the_app_only(self):
        scanner = ss.MarkerScanner(PACKAGE, {4321})
        found = [m for m in (scanner.feed(line) for line in fixture("logcat_crash.txt").splitlines()) if m]
        self.assertEqual(len(found), 4)
        self.assertTrue(found[0].startswith("FATAL EXCEPTION"))
        self.assertIn("ANR in", found[1])
        self.assertIn("Fatal signal 11", found[2])
        self.assertIn("state=Fatal", found[3])

    def test_a_fatal_exception_of_another_process_is_ignored(self):
        scanner = ss.MarkerScanner(PACKAGE)
        lines = ["E AndroidRuntime: FATAL EXCEPTION: main", "E AndroidRuntime: Process: com.example.other, PID: 9"]
        self.assertEqual([scanner.feed(line) for line in lines], [None, None])


class Verdicts(unittest.TestCase):
    def healthy(self, minutes: int = 20, **kw) -> ss.Run:
        samples = [sample(i) for i in range(minutes + 1)]
        return make_run(samples, driver_rows(minutes, ok_sales=2, z_ok=2, days=2), journal(2, zs=2), {}, **kw)

    def test_a_healthy_finished_run_passes_every_check(self):
        checks = ss.evaluate(self.healthy(), min_hours=0.3)
        failed = [c for c in checks if not c.ok]
        self.assertEqual(failed, [], [c.detail for c in failed])
        self.assertEqual(len(checks), 11)

    def test_pid_change_fails(self):
        run = self.healthy()
        run.samples[7]["pid"] = 9999
        check = ss.check_pid_stable(run)
        self.assertFalse(check.ok)
        self.assertIn("9999", check.detail)

    def test_missing_process_fails_but_adb_errors_and_post_end_samples_do_not(self):
        run = self.healthy()
        run.samples[3].update(pid=None, adb_error=True)
        run.samples.append(sample(30, pid=None, post_end=True))
        self.assertTrue(ss.check_pid_stable(run).ok)
        run.samples[4]["pid"] = None
        self.assertFalse(ss.check_pid_stable(run).ok)

    def test_crash_marker_fails_no_crash(self):
        run = self.healthy()
        self.assertTrue(ss.check_no_crash(run).ok)
        run.samples[5]["markers"] = ["FATAL EXCEPTION: main"]
        self.assertFalse(ss.check_no_crash(run).ok)

    def test_sales_signed_needs_equality_when_finished(self):
        run = self.healthy()
        self.assertTrue(ss.check_sales_signed(run).ok)
        run.journal = journal(1, zs=2)
        self.assertFalse(ss.check_sales_signed(run).ok)

    def test_a_failed_sale_fails_sales_signed(self):
        run = self.healthy()
        run.driver.append({"type": "sale", "ok": False, "ts": T0 * 1000})
        self.assertFalse(ss.check_sales_signed(run).ok)

    def test_a_running_run_tolerates_the_sale_in_flight(self):
        run = self.healthy()
        run.driver = [r for r in run.driver if r["type"] != "end"]
        run.journal = journal(1, zs=2)
        run.meta = {"last_poll_dev_ms": (T0 + 6) * 1000}
        check = ss.check_sales_signed(run)
        self.assertTrue(check.ok, check.detail)

    def test_counter_gap_fails_continuity(self):
        run = self.healthy()
        self.assertTrue(ss.check_counters(run).ok)
        run.journal[1]["response"]["data"]["signSale"]["fdmRef"]["totalCounter"] = 7
        check = ss.check_counters(run)
        self.assertFalse(check.ok)
        self.assertIn("total", check.detail)

    def test_lost_journal_polls_fail_continuity(self):
        run = self.healthy()
        run.meta = {"overflows": 1}
        self.assertFalse(ss.check_counters(run).ok)

    def test_z_done_one_per_elapsed_day(self):
        run = self.healthy()
        self.assertTrue(ss.check_z_done(run).ok)
        run.driver = [r for r in run.driver if not (r["type"] == "z" and r["ts"] > (T0 + 100) * 1000)]
        self.assertFalse(ss.check_z_done(run).ok)

    def test_z_failure_fails(self):
        run = self.healthy()
        run.driver.append({"type": "z", "ok": False, "ts": T0 * 1000})
        self.assertFalse(ss.check_z_done(run).ok)

    def test_memory_is_not_evaluated_on_a_short_run(self):
        check = ss.check_memory(self.healthy())
        self.assertTrue(check.ok)
        self.assertIn("not evaluated", check.detail)

    def test_memory_slope_after_the_first_hour(self):
        samples = [sample(i, pss_kb=200_000 + 10_240 * i // 60, native_kb=50_000) for i in range(0, 241)]
        check = ss.check_memory(make_run(samples))
        self.assertFalse(check.ok, check.detail)
        self.assertIn("+10.00 MB/h", check.detail)
        flat = [sample(i, pss_kb=200_000 + (i % 3) * 100, native_kb=50_000) for i in range(0, 241)]
        self.assertTrue(ss.check_memory(make_run(flat)).ok)

    def test_threads_and_fds_growth(self):
        run = self.healthy()
        self.assertTrue(ss.check_threads(run).ok)
        run.samples[10]["threads"] = 61
        run.samples[11]["fds"] = 151
        self.assertFalse(ss.check_threads(run).ok)
        self.assertFalse(ss.check_fds(run).ok)
        run.samples[10]["threads"] = 60
        run.samples[11]["fds"] = 150
        self.assertTrue(ss.check_threads(run).ok)
        self.assertTrue(ss.check_fds(run).ok)

    def test_beat_gap_over_120_seconds_fails(self):
        run = self.healthy()
        self.assertTrue(ss.check_beats(run).ok)
        run.driver = [r for r in run.driver if not (r["type"] == "beat" and 300 < r["ts"] / 1000 - T0 < 440)]
        check = ss.check_beats(run)
        self.assertFalse(check.ok)
        self.assertIn("largest gap", check.detail)

    def test_a_silent_driver_fails_while_the_run_goes_on(self):
        run = self.healthy()
        run.driver = [r for r in run.driver if r["type"] != "end" and r["ts"] / 1000 - T0 < 300]
        self.assertFalse(ss.check_beats(run).ok)

    def test_storage_needs_one_gib_and_reports_growth(self):
        run = self.healthy()
        run.samples[-1].update(db_bytes=300_000 + 2 * 6471, backup_kb=100)
        check = ss.check_storage(run)
        self.assertTrue(check.ok)
        self.assertIn("6471 B/sale", check.detail)
        self.assertIn("51200 B/Z", check.detail)
        run.samples[4]["free_kb"] = 900_000
        self.assertFalse(ss.check_storage(run).ok)

    def test_min_hours(self):
        run = self.healthy()
        self.assertTrue(ss.check_min_hours(run, 0.3).ok)
        self.assertFalse(ss.check_min_hours(run, 4).ok)
        self.assertTrue(ss.check_min_hours(run, None).ok)

    def test_report_lines_and_total(self):
        lines = ss.report_lines(ss.evaluate(self.healthy(), 0.3))
        self.assertEqual(len(lines), 12)
        self.assertTrue(lines[0].startswith("SOAK | pid-stable | PASS | "))
        self.assertEqual(lines[-1], "SOAK | total | PASS | 0 failed (11 checks)")
        bad = ss.report_lines(ss.evaluate(self.healthy(), 4))
        self.assertEqual(bad[-1], "SOAK | total | FAIL | 1 failed (11 checks)")
        self.assertIn("SOAK | min-hours | FAIL | ", bad[-2])

    def test_checkpoint_row_carries_every_verdict(self):
        run = self.healthy()
        row = ss.checkpoint_row(run, ss.evaluate(run), 123.0)
        self.assertTrue(row["ok"])
        self.assertEqual([c["check"] for c in row["checks"]][:3], ["pid-stable", "no-crash", "sales-signed"])
        self.assertEqual(row["elapsed_s"], 20 * 60)


class FakeDevice:
    """Plays a short soak: the app lives, the driver writes its records, then ends."""

    END_AFTER_READS = 15

    def __init__(self, rows: list[dict]) -> None:
        body = [r for r in rows if r.get("type") != "end"]
        self.before = "".join(json.dumps(r) + "\n" for r in body).encode()
        self.after = "".join(json.dumps(r) + "\n" for r in rows).encode()
        self.package = PACKAGE
        self.clock = lambda: T0
        self.reads = 0

    def probe(self):
        return {"pid": 4321, "dev_t": int(self.clock()), "rss_kb": 300_000, "threads": 40, "free_kb": 6_000_000}

    def meminfo(self, pid):
        return {"pss_kb": 200_000, "native_kb": 50_000}

    def app_files(self, pid):
        return {"db_bytes": 300_000, "wal_bytes": 0, "backup_kb": 0, "backup_files": 0, "fds": 100}

    def driver_tail(self, offset):
        self.reads += 1
        return (self.after if self.reads > self.END_AFTER_READS else self.before)[offset:]

    def logcat_process(self):
        return None


class FakeMock:
    def __init__(self, calls: list[dict]) -> None:
        self.calls = calls

    def requests(self):
        return self.calls[-100:]


class SamplerLoop(unittest.TestCase):
    def make(self, tmp: str, rows, calls, min_hours=None):
        with open(os.path.join(tmp, "state.json"), "w", encoding="utf-8") as handle:
            json.dump({"package": PACKAGE, "hours": 0.1, "day_minutes": 10, "serial": "emulator-5554", "fdm_port": 1, "min_hours": min_hours}, handle)
        clock = {"t": T0}

        def now() -> float:
            return clock["t"]

        def sleep(seconds: float) -> None:
            clock["t"] += seconds

        device = FakeDevice(rows)
        device.clock = now
        sampler = ss.Sampler(tmp, device, FakeMock(calls), clock=now, sleep=sleep)
        return sampler, device

    def test_a_whole_run_against_fakes_writes_every_file_and_passes(self):
        rows = driver_rows(6, ok_sales=2, z_ok=1, days=1)
        with tempfile.TemporaryDirectory() as tmp:
            sampler, _ = self.make(tmp, rows, journal(2, zs=1), min_hours=4)
            self.assertEqual(sampler.run(), 1)
            for name in ("samples.jsonl", "checkpoints.jsonl", "fdm_journal.jsonl", "driver.jsonl", "report.txt", "journal_meta.json"):
                self.assertTrue(os.path.exists(os.path.join(tmp, name)), name)
            report = open(os.path.join(tmp, "report.txt"), encoding="utf-8").read().splitlines()
            self.assertEqual(report[-1], "SOAK | total | FAIL | 1 failed (11 checks)", report)
            self.assertIn("SOAK | min-hours | FAIL | ", "\n".join(report))
            samples = ss.read_jsonl(os.path.join(tmp, "samples.jsonl"))
            self.assertGreaterEqual(len(samples), 2)
            self.assertTrue(samples[-1]["post_end"])

    def test_journal_poll_dedupes_and_counts_a_lost_window(self):
        with tempfile.TemporaryDirectory() as tmp:
            first = journal(3)
            sampler, _ = self.make(tmp, [], first)
            sampler.poll_journal()
            sampler.poll_journal()
            self.assertEqual(len(ss.read_jsonl(os.path.join(tmp, "fdm_journal.jsonl"))), 3)
            sampler.mock = FakeMock([dict(sign("signSale", "N", 500 + i, 500 + i), at=f"b{i}") for i in range(100)])
            sampler.poll_journal()
            self.assertEqual(sampler.meta["overflows"], 1)
            self.assertEqual(len(ss.read_jsonl(os.path.join(tmp, "fdm_journal.jsonl"))), 103)

    def test_status_calls_are_counted_not_stored(self):
        with tempfile.TemporaryDirectory() as tmp:
            sampler, _ = self.make(tmp, [], fixture_calls())
            sampler.poll_journal()
            self.assertEqual(sampler.meta["status_calls"], 1)
            stored = ss.read_jsonl(os.path.join(tmp, "fdm_journal.jsonl"))
            self.assertNotIn("Status", [c["operation"] for c in stored])

    def test_the_run_ends_when_the_app_is_gone(self):
        with tempfile.TemporaryDirectory() as tmp:
            sampler, device = self.make(tmp, [], [])
            device.probe = lambda: {"pid": None, "dev_t": int(T0), "rss_kb": None, "threads": None, "free_kb": 6_000_000}
            self.assertEqual(sampler.run(), 1)
            samples = ss.read_jsonl(os.path.join(tmp, "samples.jsonl"))
            self.assertGreaterEqual(len([s for s in samples if not s["pid"]]), ss.MISSING_LIMIT)


class Cli(unittest.TestCase):
    def test_state_set_and_get(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "state.json")
            self.assertEqual(ss.main(["state", path, "set", "serial=emulator-5554", "hours=0.34", "pids=[1,2]"]), 0)
            state = ss.read_json(path)
            self.assertEqual(state, {"serial": "emulator-5554", "hours": 0.34, "pids": [1, 2]})

    def test_report_without_samples_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(ss.main(["report", "--dir", tmp]), 1)

    def test_report_from_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            run = self.healthy_files(tmp)
            self.assertEqual(ss.main(["report", "--dir", run, "--min-hours", "0.3"]), 0)
            self.assertEqual(ss.main(["report", "--dir", run, "--min-hours", "4"]), 1)

    def healthy_files(self, tmp: str) -> str:
        for name, rows in (
            ("samples.jsonl", [sample(i) for i in range(21)]),
            ("driver.jsonl", driver_rows(20, ok_sales=2, z_ok=2, days=2)),
            ("fdm_journal.jsonl", journal(2, zs=2)),
        ):
            for row in rows:
                ss.append_jsonl(os.path.join(tmp, name), row)
        ss.write_json(os.path.join(tmp, "state.json"), {"day_minutes": 10})
        return tmp


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    print(f"TESTSOAK | total | {'PASS' if result.wasSuccessful() else 'FAIL'} | {result.testsRun} tests")
    sys.exit(0 if result.wasSuccessful() else 1)
