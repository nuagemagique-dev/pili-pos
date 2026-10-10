#!/usr/bin/env python3
"""Builds the Android journal of one UC scenario (P7 section 4.4, side android).

Inputs, all JSON files pulled by scripts/android/uc_e2e.sh:
  --steps FILE     the steps file the instrumented test wrote:
                   {scenario, group, ids, posSwVersion, steps:[{cmd, ok, error, result}], verdict}
  --requests FILE  GET /mock/requests of pos-fdm-mock: [{at, operation, request, response}]
  --jobs FILE      GET /jobs of pos-printer-mock: [{id, at, bytes, printed, drawerKick, text}]
  --out FILE       where the journal goes
  --detail         also print, on stderr, the first step that did not meet its expectation

The journal:
  {format: 1, side: "android", group, scenario, ids, posSwVersion, steps, fdm, statusCalls, tickets, verdict}
  fdm          the non-Status calls of the mock, in order, as {operation, request, response}
  statusCalls  the Status calls, as {request, response}
  tickets      the text of each printer job, in print order (a job without text, a drawer kick, is left out)
  verdict      PASS when the steps file says PASS and no step has ok false, else FAIL

The journal holds no money logic: it copies what the device, the FDM mock and the printer mock reported.
Prints 'JOURNAL <verdict> fdm=<n> status=<n> tickets=<n>' on stdout. Exit 0 when a journal was written (also
for a FAIL verdict: the caller decides), 2 on a usage error or an input that is not what the runner writes.

Run with python3 -I. Standard library only.
"""
import argparse
import json
import sys

FORMAT = 1
SIDE = "android"
STATUS_OPERATION = "Status"
STEP_KEYS = ("cmd", "ok", "error", "result")


class JournalError(Exception):
    """An input file is not what the runner or the mocks write."""


def _require(condition, message):
    if not condition:
        raise JournalError(message)


def _load(path, what):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle)
    except OSError as err:
        raise JournalError("%s: cannot read %s (%s)" % (what, path, err)) from err
    except ValueError as err:
        raise JournalError("%s: %s is not JSON (%s)" % (what, path, err)) from err


def split_requests(requests):
    """Splits the mock's log into (fdm calls, status calls), each in the mock's order."""
    _require(isinstance(requests, list), "requests: a JSON array was expected")
    fdm, status = [], []
    for index, call in enumerate(requests):
        _require(isinstance(call, dict), "requests[%d]: an object was expected" % index)
        for key in ("operation", "request", "response"):
            _require(key in call, "requests[%d]: missing '%s'" % (index, key))
        _require(isinstance(call["operation"], str) and call["operation"], "requests[%d]: operation must be a text" % index)
        if call["operation"] == STATUS_OPERATION:
            status.append({"request": call["request"], "response": call["response"]})
        else:
            fdm.append({"operation": call["operation"], "request": call["request"], "response": call["response"]})
    return fdm, status


def tickets_of(jobs):
    """The text of each printer job in print order; jobs without text (drawer kicks) are left out."""
    _require(isinstance(jobs, list), "jobs: a JSON array was expected")
    for index, job in enumerate(jobs):
        _require(isinstance(job, dict), "jobs[%d]: an object was expected" % index)
        _require(isinstance(job.get("id"), int), "jobs[%d]: 'id' must be an integer" % index)
        _require(isinstance(job.get("text", ""), str), "jobs[%d]: 'text' must be a text" % index)
    ordered = sorted(jobs, key=lambda job: job["id"])
    return [job["text"] for job in ordered if job.get("text", "") != ""]


def clean_steps(steps):
    """The steps as the journal keeps them: cmd, ok, error, result (the runner's 'detail' is dropped)."""
    _require(isinstance(steps, list), "steps: 'steps' must be an array")
    cleaned = []
    for index, step in enumerate(steps):
        _require(isinstance(step, dict), "steps[%d]: an object was expected" % index)
        for key in STEP_KEYS:
            _require(key in step, "steps[%d]: missing '%s'" % (index, key))
        _require(isinstance(step["cmd"], str) and step["cmd"], "steps[%d]: 'cmd' must be a text" % index)
        _require(isinstance(step["ok"], bool), "steps[%d]: 'ok' must be true or false" % index)
        cleaned.append({key: step[key] for key in STEP_KEYS})
    return cleaned


def first_failure(steps_doc):
    """A one-line text about the first step that did not meet its expectation, or ''."""
    for index, step in enumerate(steps_doc.get("steps", [])):
        if isinstance(step, dict) and step.get("ok") is False:
            detail = step.get("detail") or step.get("error") or "no detail"
            return "step %d %s: %s" % (index, step.get("cmd"), detail)
    return ""


def build_journal(steps_doc, requests, jobs):
    """The journal of format 1 for side android."""
    _require(isinstance(steps_doc, dict), "steps file: an object was expected")
    for key in ("scenario", "group", "ids", "posSwVersion", "steps", "verdict"):
        _require(key in steps_doc, "steps file: missing '%s'" % key)
    _require(isinstance(steps_doc["ids"], list), "steps file: 'ids' must be an array")
    _require(
        isinstance(steps_doc["posSwVersion"], str) and steps_doc["posSwVersion"],
        "steps file: 'posSwVersion' must be a non-empty text",
    )
    _require(steps_doc["verdict"] in ("PASS", "FAIL"), "steps file: 'verdict' must be PASS or FAIL")
    steps = clean_steps(steps_doc["steps"])
    fdm, status = split_requests(requests)
    all_ok = all(step["ok"] for step in steps)
    verdict = "PASS" if steps_doc["verdict"] == "PASS" and all_ok else "FAIL"
    return {
        "format": FORMAT,
        "side": SIDE,
        "group": steps_doc["group"],
        "scenario": steps_doc["scenario"],
        "ids": steps_doc["ids"],
        "posSwVersion": steps_doc["posSwVersion"],
        "steps": steps,
        "fdm": fdm,
        "statusCalls": status,
        "tickets": tickets_of(jobs),
        "verdict": verdict,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description="Build the Android journal of one UC scenario.")
    parser.add_argument("--steps", required=True)
    parser.add_argument("--requests", required=True)
    parser.add_argument("--jobs", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--detail", action="store_true")
    args = parser.parse_args(argv)
    try:
        steps_doc = _load(args.steps, "steps")
        journal = build_journal(steps_doc, _load(args.requests, "requests"), _load(args.jobs, "jobs"))
    except JournalError as err:
        print("uc_journal: %s" % err, file=sys.stderr)
        return 2
    with open(args.out, "w", encoding="utf-8", newline="\n") as handle:
        json.dump(journal, handle, indent=2, ensure_ascii=False)
        handle.write("\n")
    if args.detail:
        failure = first_failure(steps_doc)
        if failure:
            print("uc_journal: %s" % failure, file=sys.stderr)
    print(
        "JOURNAL %s fdm=%d status=%d tickets=%d"
        % (journal["verdict"], len(journal["fdm"]), len(journal["statusCalls"]), len(journal["tickets"]))
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
