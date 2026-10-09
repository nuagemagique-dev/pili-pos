#!/usr/bin/env python3
"""Self-test of uc_journal.py. Run: python3 -I scripts/android/test_uc_journal.py"""
import copy
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import uc_journal  # noqa: E402

DATA = os.path.join(HERE, "testdata", "uc_journal")
SCRIPT = os.path.join(HERE, "uc_journal.py")


def load(name):
    with open(os.path.join(DATA, name), encoding="utf-8") as handle:
        return json.load(handle)


class BuildJournalTest(unittest.TestCase):
    def setUp(self):
        self.steps = load("steps_pass.json")
        self.requests = load("requests.json")
        self.jobs = load("jobs.json")

    def test_a_passing_scenario_matches_the_expected_journal(self):
        journal = uc_journal.build_journal(self.steps, self.requests, self.jobs)
        self.assertEqual(journal, load("expected_pass.json"))

    def test_the_header_of_the_journal_is_format_1_side_android(self):
        journal = uc_journal.build_journal(self.steps, self.requests, self.jobs)
        self.assertEqual(journal["format"], 1)
        self.assertEqual(journal["side"], "android")
        self.assertEqual((journal["group"], journal["scenario"]), ("00_smoke", "001_SMOKE"))
        self.assertTrue(journal["posSwVersion"].startswith("pili-pos-android/"))

    def test_status_calls_are_split_from_the_signing_calls_in_order(self):
        journal = uc_journal.build_journal(self.steps, self.requests, self.jobs)
        self.assertEqual([call["operation"] for call in journal["fdm"]], ["signSale", "signReportTurnoverX"])
        self.assertEqual(len(journal["statusCalls"]), 2)
        for call in journal["statusCalls"]:
            self.assertEqual(sorted(call), ["request", "response"])

    def test_fdm_entries_keep_request_and_response_unchanged(self):
        journal = uc_journal.build_journal(self.steps, self.requests, self.jobs)
        self.assertEqual(journal["fdm"][0]["request"], self.requests[1]["request"])
        self.assertEqual(journal["fdm"][0]["response"], self.requests[1]["response"])
        self.assertNotIn("at", journal["fdm"][0])

    def test_tickets_follow_the_job_ids_and_leave_out_the_drawer_kick(self):
        journal = uc_journal.build_journal(self.steps, self.requests, self.jobs)
        self.assertEqual(journal["tickets"], ["TICKET 1\nPerrier x2", "X REPORT\nTotal 0,00"])

    def test_a_failing_scenario_is_a_fail_verdict_without_the_runner_detail(self):
        journal = uc_journal.build_journal(load("steps_fail.json"), [], [])
        self.assertEqual(journal["verdict"], "FAIL")
        self.assertEqual(journal["steps"][1]["error"], "FDM_LOCKED")
        self.assertNotIn("detail", journal["steps"][1])
        self.assertEqual(journal["fdm"], [])
        self.assertEqual(journal["tickets"], [])

    def test_a_pass_verdict_with_a_step_that_is_not_ok_becomes_a_fail(self):
        steps = copy.deepcopy(self.steps)
        steps["steps"][2]["ok"] = False
        self.assertEqual(uc_journal.build_journal(steps, [], [])["verdict"], "FAIL")

    def test_first_failure_names_the_step_and_its_detail(self):
        text = uc_journal.first_failure(load("steps_fail.json"))
        self.assertEqual(text, "step 1 checkout: the call failed: FDM_LOCKED")
        self.assertEqual(uc_journal.first_failure(self.steps), "")

    def test_a_steps_file_without_pos_sw_version_is_refused(self):
        steps = copy.deepcopy(self.steps)
        del steps["posSwVersion"]
        with self.assertRaises(uc_journal.JournalError):
            uc_journal.build_journal(steps, [], [])

    def test_an_empty_pos_sw_version_is_refused(self):
        steps = copy.deepcopy(self.steps)
        steps["posSwVersion"] = ""
        with self.assertRaises(uc_journal.JournalError):
            uc_journal.build_journal(steps, [], [])

    def test_a_step_without_ok_is_refused(self):
        steps = copy.deepcopy(self.steps)
        del steps["steps"][0]["ok"]
        with self.assertRaises(uc_journal.JournalError):
            uc_journal.build_journal(steps, [], [])

    def test_requests_that_are_not_a_list_are_refused(self):
        with self.assertRaises(uc_journal.JournalError):
            uc_journal.build_journal(self.steps, {"operation": "signSale"}, [])

    def test_a_logged_call_without_operation_is_refused(self):
        with self.assertRaises(uc_journal.JournalError):
            uc_journal.split_requests([{"request": {}, "response": {}}])

    def test_a_job_without_integer_id_is_refused(self):
        with self.assertRaises(uc_journal.JournalError):
            uc_journal.tickets_of([{"id": "1", "text": "x"}])

    def test_the_input_documents_are_not_modified(self):
        before = (copy.deepcopy(self.steps), copy.deepcopy(self.requests), copy.deepcopy(self.jobs))
        uc_journal.build_journal(self.steps, self.requests, self.jobs)
        self.assertEqual((self.steps, self.requests, self.jobs), before)


class CommandLineTest(unittest.TestCase):
    def run_cli(self, steps, requests="requests.json", jobs="jobs.json", extra=()):
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "journal.json")
            args = [
                sys.executable,
                "-I",
                SCRIPT,
                "--steps",
                os.path.join(DATA, steps),
                "--requests",
                os.path.join(DATA, requests),
                "--jobs",
                os.path.join(DATA, jobs),
                "--out",
                out,
                *extra,
            ]
            proc = subprocess.run(args, capture_output=True, text=True, check=False)
            written = None
            if os.path.exists(out):
                with open(out, encoding="utf-8") as handle:
                    raw = handle.read()
                written = (raw, json.loads(raw))
            return proc, written

    def test_the_command_line_writes_the_journal_and_a_summary_line(self):
        proc, written = self.run_cli("steps_pass.json")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(proc.stdout.strip(), "JOURNAL PASS fdm=2 status=2 tickets=2")
        self.assertEqual(written[1], load("expected_pass.json"))
        self.assertTrue(written[0].endswith("}\n"))

    def test_a_fail_verdict_is_still_exit_0_and_detail_goes_to_stderr(self):
        proc, written = self.run_cli("steps_fail.json", extra=("--detail",))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertTrue(proc.stdout.startswith("JOURNAL FAIL"))
        self.assertIn("step 1 checkout: the call failed: FDM_LOCKED", proc.stderr)
        self.assertEqual(written[1]["verdict"], "FAIL")

    def test_a_missing_input_is_exit_2_and_writes_nothing(self):
        proc, written = self.run_cli("steps_pass.json", requests="does_not_exist.json")
        self.assertEqual(proc.returncode, 2)
        self.assertIn("cannot read", proc.stderr)
        self.assertIsNone(written)

    def test_main_returns_2_for_input_that_is_not_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            bad = os.path.join(tmp, "bad.json")
            with open(bad, "w", encoding="utf-8") as handle:
                handle.write("{not json")
            argv = [
                "--steps",
                bad,
                "--requests",
                os.path.join(DATA, "requests.json"),
                "--jobs",
                os.path.join(DATA, "jobs.json"),
                "--out",
                os.path.join(tmp, "out.json"),
            ]
            err = io.StringIO()
            with redirect_stdout(io.StringIO()), redirect_stderr(err):
                code = uc_journal.main(argv)
            self.assertEqual(code, 2)
            self.assertIn("is not JSON", err.getvalue())


if __name__ == "__main__":
    unittest.main(verbosity=1)
