#!/usr/bin/env python3
"""uc_matrix_android.py - Android view of the SPF test traceability matrix (card P7-T9, P7D9).

Reuses scripts/uc_matrix.py (loaded with importlib, never edited) for the official ids, the
labels, the Rust test scan and the Linux exclusions of scripts/uc_scope.toml, then decides, for
every id, how the Android port covers it:

  Couvert (scénario Android)   a scenario under android/app/src/androidTest/assets/uc lists the
                               id in "ids" and docs/android/uc/results.json has verdict PASS for
                               that scenario
  En attente                   the id is required (REQUIRED, 47 ids of P7D9) and has no scenario
                               or no PASS result yet (accepted only with --allow-pending)
  Couvert (tests moteur ...)   not required, and Rust tests carry the id that the device run executes
                               (docs/android/DEVICE_TESTS.md: pos-engine and the other crates of the
                               cert graph, not the Tauri shell, the mocks, pos-peppol or #[ignore]),
                               at least one of them already existed at the last recorded device run
                               (the commit named in docs/android/uc/device_engine_tests.md, read
                               with git); the tests added since are listed apart in the row
  Couvert (tests moteur sur l'hôte ...)
                               the same, but none of its tests existed at that run (or no run is
                               recorded): they passed on the host only, the device run is to do
  Hors périmètre, Non applicable, Couvert par <id>
                               the Linux exclusions of scripts/uc_scope.toml, mirrored
  Exclu sur Android            a declared exclusion of scripts/android/uc_scope_android.toml, for
                               an id that Linux covers only with a Playwright or real-binary UI
                               test (one French reason per id)
  MANQUANT                     none of the above

Output: docs/android/UC_MATRIX_ANDROID.md (French, brand-neutral).

Usage:
  python3 scripts/android/uc_matrix_android.py --check                  # release mode
  python3 scripts/android/uc_matrix_android.py --check --allow-pending  # development mode
  python3 scripts/android/uc_matrix_android.py --write                  # rewrite the doc
  python3 scripts/android/uc_matrix_android.py --list-missing           # MANQUANT and pending ids
  --repo DIR                                                             # repository root

Lines: 'UCMATRIX | <check> | PASS|FAIL | <detail>' then 'UCMATRIX | total | PASS|FAIL | ...'.
--check exits 1 on a MANQUANT id, a pending id (unless --allow-pending), a stale or unknown
Android exclusion, an unreadable input, or a difference with the committed doc other than the
date line (the doc is not compared with --allow-pending).

python3 stdlib only.
"""

from __future__ import annotations

import argparse
import datetime
import importlib.util
import json
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_REPO = HERE.parent.parent

LINUX_SCRIPT = "scripts/uc_matrix.py"
SCOPE_ANDROID = "scripts/android/uc_scope_android.toml"
SCENARIOS_DIR = "android/app/src/androidTest/assets/uc"
RESULTS_JSON = "docs/android/uc/results.json"
DEVICE_RECORD = "docs/android/uc/device_engine_tests.md"
OUTPUT_MD = "docs/android/UC_MATRIX_ANDROID.md"

DATE_PREFIX = "Date de génération : "

# P7D9: the 47 ids that each need a passing Android scenario.
REQUIRED = (
    "M110 M111 M112B M121 M123 M130 M131 M133 M140 M141 M160 M161 M170 M171 M171B M172 "
    "Q100 Q110 M180 M181 M182 M183 M184 T1 T2 T3 T4 T5 T6 T7 T8 UC201 UC223 UC230 UC4109 "
    "UC4119 ROUND1 ROUND2 ROUND3 ROUND4 ROUND5 V_FDM_LOCKED V_BUFFER_FULL V_BUFFER_NEAR_FULL "
    "V_UNKNOWN_POS V_UNAUTHORIZED FAQ_Q205"
).split()

ST_SCENARIO = "scenario"
ST_PENDING = "pending"
ST_ENGINE = "engine"
ST_ENGINE_HOST = "engine_host"
ST_HORS = "hors_perimetre"
ST_NA = "non_applicable"
ST_COUVERT_PAR = "couvert_par"
ST_EXCLU = "exclu_android"
ST_MISSING = "missing"

# Order of the summary table.
STATUS_ORDER = [
    ST_SCENARIO,
    ST_ENGINE,
    ST_ENGINE_HOST,
    ST_PENDING,
    ST_EXCLU,
    ST_HORS,
    ST_NA,
    ST_COUVERT_PAR,
    ST_MISSING,
]

STATUS_LABELS = {
    ST_SCENARIO: "Couvert (scénario Android)",
    ST_ENGINE: "Couvert (tests moteur exécutés sur l'appareil)",
    ST_ENGINE_HOST: "Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire)",
    ST_PENDING: "En attente",
    ST_EXCLU: "Exclu sur Android",
    ST_HORS: "Hors périmètre",
    ST_NA: "Non applicable",
    ST_COUVERT_PAR: "Couvert par un autre identifiant",
    ST_MISSING: "MANQUANT",
}


class InputProblem(Exception):
    """An unreadable or invalid input file (reported by the 'inputs' check)."""


# ---------------------------------------------------------------------------
# Linux matrix (scripts/uc_matrix.py, loaded as is)
# ---------------------------------------------------------------------------


def load_ucm(repo: Path):
    """Fresh module object of the repository's scripts/uc_matrix.py (this repository's copy when
    `repo` has none, so a fake repository needs no copy of the 800-line script)."""
    path = repo / LINUX_SCRIPT
    if not path.is_file():
        path = DEFAULT_REPO / LINUX_SCRIPT
    spec = importlib.util.spec_from_file_location("uc_matrix_linux", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module  # dataclasses look the module up here
    spec.loader.exec_module(module)
    return module


# ---------------------------------------------------------------------------
# Scenarios, results, Android scope
# ---------------------------------------------------------------------------


@dataclass
class Scenario:
    group: str
    stem: str
    rel: str  # path under the scenarios directory, e.g. 10_basic/010_M110.json
    ids: list
    verdict: str = ""  # "" when results.json has no entry


def load_scenarios(repo: Path) -> list:
    base = repo / SCENARIOS_DIR
    out = []
    if not base.is_dir():
        return out
    for f in sorted(base.rglob("*.json")):
        rel = f.relative_to(base).as_posix()
        try:
            data = json.loads(f.read_text(encoding="utf-8"))
        except (OSError, ValueError) as exc:
            raise InputProblem(f"{SCENARIOS_DIR}/{rel}: unreadable scenario ({exc})") from exc
        if not isinstance(data, dict):
            raise InputProblem(f"{SCENARIOS_DIR}/{rel}: the scenario is not a JSON object")
        ids = data.get("ids", [data["id"]] if "id" in data else [])
        if not isinstance(ids, list) or not all(isinstance(i, str) for i in ids):
            raise InputProblem(f"{SCENARIOS_DIR}/{rel}: 'ids' is not a list of strings")
        out.append(Scenario(group=f.parent.name, stem=f.stem, rel=rel, ids=ids))
    return out


def load_results(repo: Path) -> dict:
    """{(group, scenario): verdict} from docs/android/uc/results.json (P7.md 4.5); {} when absent."""
    path = repo / RESULTS_JSON
    if not path.is_file():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        entries = data["scenarios"]
        return {(e["group"], e["scenario"]): str(e["verdict"]) for e in entries}
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise InputProblem(f"{RESULTS_JSON}: unreadable results ({type(exc).__name__}: {exc})") from exc


def load_scope_android(repo: Path, required: list) -> dict:
    path = repo / SCOPE_ANDROID
    if not path.is_file():
        return {}
    try:
        with path.open("rb") as fh:
            data = tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise InputProblem(f"{SCOPE_ANDROID}: unreadable ({exc})") from exc
    for id_, entry in data.items():
        if not isinstance(entry, dict):
            raise InputProblem(f"{SCOPE_ANDROID}: [{id_}] is not a table")
        if entry.get("kind", "exclu_android") != "exclu_android":
            raise InputProblem(f"{SCOPE_ANDROID}: [{id_}] has kind {entry.get('kind')!r} (expected exclu_android)")
        reason = entry.get("reason")
        if not isinstance(reason, str) or not reason.strip():
            raise InputProblem(f"{SCOPE_ANDROID}: [{id_}] is missing a reason")
        if id_ in required:
            raise InputProblem(f"{SCOPE_ANDROID}: [{id_}] is a required id and cannot be excluded")
    return data


# ---------------------------------------------------------------------------
# Matrix
# ---------------------------------------------------------------------------


@dataclass
class ARow:
    id: str
    label: str
    scenarios: list = field(default_factory=list)  # Scenario files listing the id
    rust_tests: list = field(default_factory=list)
    host_only: list = field(default_factory=list)  # rust_tests absent from the last device run
    status: str = ST_MISSING
    couvert_par: str = ""
    required: bool = False

    @property
    def status_label(self) -> str:
        if self.status == ST_COUVERT_PAR:
            return f"Couvert par {self.couvert_par}"
        return STATUS_LABELS[self.status]

    @property
    def result_label(self) -> str:
        if not self.scenarios:
            return "-"
        verdicts = sorted({s.verdict or "Aucun résultat" for s in self.scenarios})
        return ", ".join(verdicts)


@dataclass
class DeviceRun:
    """The last recorded run of the engine tests on the device."""

    commit: str
    date: str
    names: frozenset  # fn names of the device crates at `commit`


@dataclass
class Matrix:
    sections: list  # [(name, [ARow])]
    stale: list  # human-readable stale or unknown Android exclusions
    unknown_required: list
    scope_android: dict
    device_run: DeviceRun | None = None

    def rows(self):
        for _, rows in self.sections:
            yield from rows

    def ids_with(self, status: str) -> list:
        return [r.id for r in self.rows() if r.status == status]

    def counts(self) -> dict:
        counts = {s: 0 for s in STATUS_ORDER}
        for r in self.rows():
            counts[r.status] += 1
        return counts


# Crates whose tests the device run executes (docs/android/DEVICE_TESTS.md): the pos-engine crate
# and the other crates of the cert build graph. Not run: the Tauri shell (src-tauri), pos-peppol,
# the mock crates, the export crates and the #[ignore] tests.
DEVICE_CRATES = (
    "pos-engine",
    "pos-core",
    "pos-db",
    "pos-fiscal",
    "pos-print",
    "pos-mail",
    "pos-hubrise",
    "pos-catalog",
    "pos-hubrise-types",
)
IGNORE_ATTR_RE = re.compile(r"#\[\s*ignore\b")


def _is_ignored(repo: Path, rt, ucm) -> bool:
    """True when a `#[ignore]` attribute sits among the attribute lines above the fn `rt`."""
    try:
        lines = (repo / rt.file).read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return False
    for i, line in enumerate(lines):
        m = ucm.FN_RE.match(line)
        if not m or m.group(1) != rt.fn_name:
            continue
        j = i - 1
        while j >= 0 and j > i - 1 - ucm.BACKWARD_SCAN_LIMIT:
            stripped = lines[j].strip()
            if IGNORE_ATTR_RE.search(stripped):
                return True
            if stripped == "" or stripped.startswith("#[") or stripped.startswith("//"):
                j -= 1
                continue
            break
    return False


def device_rust_tests(repo: Path, ucm) -> list:
    """The Rust tests of ucm.scan_rust_tests that the device run executes: those under
    crates/<device crate>/ that are not #[ignore] (the Linux scan also reads src-tauri/src and
    every other crate)."""
    prefixes = tuple(f"crates/{c}/" for c in DEVICE_CRATES)
    out = []
    for rt in ucm.scan_rust_tests(repo):
        if rt.file.startswith(prefixes) and not _is_ignored(repo, rt, ucm):
            out.append(rt)
    return out


DEVICE_COMMIT_RE = re.compile(r"on commit `([0-9a-f]{7,40})`")
DEVICE_DATE_RE = re.compile(r"Run on (\d{4}-\d{2}-\d{2})")


def load_device_run(repo: Path) -> DeviceRun | None:
    """The last device run recorded in DEVICE_RECORD (None when the file is absent): its commit,
    its date and the names of the fns that the device crates held at that commit (git grep). A
    test that carries an id and whose name is among them ran on the device."""
    path = repo / DEVICE_RECORD
    if not path.is_file():
        return None
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise InputProblem(f"{DEVICE_RECORD}: {exc}") from exc
    commit = DEVICE_COMMIT_RE.search(text)
    date = DEVICE_DATE_RE.search(text)
    if not commit or not date:
        raise InputProblem(f"{DEVICE_RECORD}: no 'Run on <date>' or 'on commit `<hash>`'")
    proc = subprocess.run(
        ["git", "-C", str(repo), "grep", "-h", "-o", "-E", r"fn [A-Za-z0-9_]+", commit.group(1), "--"]
        + [f"crates/{c}/" for c in DEVICE_CRATES],
        capture_output=True,
        text=True,
    )
    if proc.returncode not in (0, 1):  # 1: no match
        raise InputProblem(f"{DEVICE_RECORD}: git grep at {commit.group(1)} failed: {proc.stderr.strip()}")
    names = frozenset(line.split()[1] for line in proc.stdout.splitlines() if line.startswith("fn "))
    return DeviceRun(commit.group(1), date.group(1), names)


def device_tests_for(id_: str, tests: list, ucm) -> list:
    """Names of the device tests that carry `id_` (same matching rule as scripts/uc_matrix.py)."""
    token = id_.lower()
    names = []
    for t in tests:
        if ucm.token_present(t.fn_name.lower(), token) or id_.upper() in {c.strip().upper() for c in t.covers}:
            names.append(t.fn_name)
    return names


def build_matrix(repo: Path, required=None, ucm=None) -> Matrix:
    required = list(REQUIRED if required is None else required)
    ucm = ucm or load_ucm(repo)
    linux = ucm.build_report(repo)
    scenarios = load_scenarios(repo)
    results = load_results(repo)
    scope_android = load_scope_android(repo, required)
    device_tests = device_rust_tests(repo, ucm)
    device_run = load_device_run(repo)
    ran_names = device_run.names if device_run else frozenset()

    for s in scenarios:
        s.verdict = results.get((s.group, s.stem), "")
    by_id: dict = {}
    for s in scenarios:
        for id_ in s.ids:
            by_id.setdefault(id_, []).append(s)

    known = {r.id for _, rows in linux.sections for r in rows}
    sections = []
    stale = []
    for name, rows in linux.sections:
        out = []
        for lr in rows:
            scs = by_id.get(lr.id, [])
            dev = device_tests_for(lr.id, device_tests, ucm)
            row = ARow(
                id=lr.id,
                label=lr.label,
                scenarios=scs,
                rust_tests=dev,
                required=lr.id in required,
            )
            # any(PASS): one PASS and one FAIL scenario for the same id counts as covered here;
            # the replay gate (uc-replay, --exit mode) fails on any FAIL verdict anyway.
            if any(s.verdict == "PASS" for s in scs):
                row.status = ST_SCENARIO
            elif row.required:
                row.status = ST_PENDING
            elif lr.status == "hors_perimetre":
                row.status = ST_HORS
            elif dev:
                row.host_only = [t for t in dev if t not in ran_names]
                row.status = ST_ENGINE if len(row.host_only) < len(dev) else ST_ENGINE_HOST
            elif lr.id in scope_android:
                row.status = ST_EXCLU
            elif lr.status == "non_applicable":
                row.status = ST_NA
            elif lr.status == "couvert_par":
                row.status = ST_COUVERT_PAR
                row.couvert_par = lr.couvert_par
            else:
                row.status = ST_MISSING
            if lr.id in scope_android:
                if scs:
                    stale.append(f"{lr.id} (has a scenario)")
                elif dev:
                    stale.append(f"{lr.id} (is covered by Rust tests)")
                elif lr.status in ("hors_perimetre", "non_applicable", "couvert_par"):
                    stale.append(f"{lr.id} (already excluded on Linux as {lr.status})")
            out.append(row)
        sections.append((name, out))
    for id_ in sorted(scope_android):
        if id_ not in known:
            stale.append(f"{id_} (unknown to the Linux matrix)")
    unknown_required = sorted(i for i in required if i not in known)
    return Matrix(sections, sorted(stale), unknown_required, scope_android, device_run)


# ---------------------------------------------------------------------------
# Markdown
# ---------------------------------------------------------------------------


def device_paragraph(run: DeviceRun | None) -> str:
    """The paragraph that says which device run the engine statuses rest on."""
    if run is None:
        return (
            "Passage sur l'appareil : aucun passage des tests du moteur sur l'appareil n'est "
            f"enregistré (`{DEVICE_RECORD}` absent) ; les identifiants couverts par des tests Rust ont "
            "le statut Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire)."
        )
    return (
        "Passage sur l'appareil : le dernier passage des tests du moteur sur l'émulateur date du "
        f"{run.date} (`{DEVICE_RECORD}`, commit `{run.commit}`). Un test qui n'existait pas encore à "
        "ce commit a passé sur la machine hôte seulement : son exécution sur l'appareil reste à faire "
        "(`scripts/android/device_engine_tests.sh`). Un identifiant dont aucun test n'existait à ce "
        "commit a le statut Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) ; pour "
        "un identifiant déjà couvert sur l'appareil, les tests ajoutés depuis sont listés à part "
        "(« sur l'hôte seulement »)."
    )


def render_markdown(matrix: Matrix, ucm, today: str, n_required: int) -> str:
    esc = ucm._esc
    cell = ucm._fmt_cell
    total = sum(matrix.counts().values())
    lines = [
        "# Matrice de traçabilité des tests SPF, version Android",
        "",
        "Ce document indique, pour chaque identifiant de test décrit par le SPF Finances "
        "(Basic Set, Reports, Advanced Set, checklist du manuel de certification, FAQ producteurs), "
        "comment le port Android de la caisse le couvre : par un scénario de bout en bout rejoué "
        "sur l'appareil et comparé au moteur de référence, par les tests du moteur (exécutés sur "
        "l'appareil, ou sur la machine hôte en attendant leur passage sur l'appareil), ou par une "
        "exclusion déclarée.",
        "",
        "Généré automatiquement par `scripts/android/uc_matrix_android.py` à partir de la matrice "
        "Linux (`scripts/uc_matrix.py`, `scripts/uc_scope.toml`), des scénarios "
        "(`android/app/src/androidTest/assets/uc`), des résultats "
        "(`docs/android/uc/results.json`) et des exclusions propres à Android "
        "(`scripts/android/uc_scope_android.toml`). Une réexécution sans changement produit un "
        "fichier identique, à la ligne de date près.",
        "",
        f"{DATE_PREFIX}{today}",
        "",
        f"Les {n_required} identifiants requis doivent chacun avoir un scénario Android au verdict PASS ; "
        "tant que ce n'est pas le cas, leur statut est En attente.",
        "",
        "Légende du statut : Couvert (scénario Android) (un scénario liste l'identifiant et son "
        "résultat est PASS), Couvert (tests moteur exécutés sur l'appareil) (des tests Rust portent "
        "l'identifiant et au moins l'un d'eux a passé lors du dernier passage sur l'appareil), "
        "Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) (des tests Rust portent "
        "l'identifiant, mais aucun n'existait lors du dernier passage sur l'appareil : ils ont passé "
        "sur la machine hôte seulement), En attente (identifiant requis "
        "sans scénario au verdict PASS), Exclu sur Android (comportement d'écran couvert sur Linux "
        "par un test d'interface seulement, prouvé sur Android autrement ; raison en fin de "
        "document), Hors périmètre, Non applicable et Couvert par X (exclusions de la matrice "
        "Linux reprises à l'identique), MANQUANT (aucune couverture ni exclusion).",
        "",
        device_paragraph(matrix.device_run),
        "",
        "## Synthèse",
        "",
        "| Statut | Nombre |",
        "|---|---|",
    ]
    counts = matrix.counts()
    for st in STATUS_ORDER:
        lines.append(f"| {STATUS_LABELS[st]} | {counts[st]} |")
    lines.append(f"| Total | {total} |")
    lines.append("")

    for name, rows in matrix.sections:
        lines.append(f"## {name}")
        lines.append("")
        lines.append("| Id | Libellé | Scénario Android | Résultat Android | Tests moteur | Statut |")
        lines.append("|---|---|---|---|---|---|")
        for r in rows:
            files = cell([s.rel for s in r.scenarios])
            tests = cell(r.rust_tests)
            if r.status == ST_ENGINE and r.host_only:
                ran = [t for t in r.rust_tests if t not in r.host_only]
                tests = f"{cell(ran)} ; sur l'hôte seulement : {cell(r.host_only)}"
            lines.append(
                f"| {esc(r.id)} | {esc(r.label)} | {files} | {esc(r.result_label)} | "
                f"{tests} | {r.status_label} |"
            )
        lines.append("")

    excluded = [r for r in matrix.rows() if r.status == ST_EXCLU]
    if excluded:
        lines.append("## Exclusions propres à Android")
        lines.append("")
        lines.append("| Id | Raison |")
        lines.append("|---|---|")
        for r in excluded:
            lines.append(f"| {esc(r.id)} | {esc(matrix.scope_android[r.id]['reason'])} |")
        lines.append("")
    return "\n".join(lines).rstrip("\n") + "\n"


def strip_date(text: str) -> str:
    return "\n".join(ln for ln in text.split("\n") if not ln.startswith(DATE_PREFIX))


# ---------------------------------------------------------------------------
# Checks and CLI
# ---------------------------------------------------------------------------


def run_checks(repo: Path, allow_pending: bool, compare_doc: bool, required=None, ucm=None):
    """([(check, ok, detail)], Matrix | None, rendered markdown | None)."""
    required = list(REQUIRED if required is None else required)
    results = []

    def add(check, ok, detail):
        results.append((check, ok, detail))

    try:
        ucm = ucm or load_ucm(repo)
        matrix = build_matrix(repo, required, ucm)
    except InputProblem as exc:
        add("inputs", False, str(exc))
        return results, None, None
    add("inputs", True, "scenarios, results and Android scope readable")

    add(
        "required",
        not matrix.unknown_required and len(set(required)) == len(required),
        f"{len(required)} required ids"
        + (f", unknown to the Linux matrix: {', '.join(matrix.unknown_required)}" if matrix.unknown_required else ""),
    )

    missing = matrix.ids_with(ST_MISSING)
    add("missing", not missing, "no MANQUANT id" if not missing else f"{len(missing)} MANQUANT: {', '.join(missing)}")

    pending = matrix.ids_with(ST_PENDING)
    if not pending:
        add("pending", True, "no pending id")
    elif allow_pending:
        add("pending", True, f"{len(pending)} pending accepted (--allow-pending): {', '.join(pending)}")
    else:
        add("pending", False, f"{len(pending)} pending required ids: {', '.join(pending)}")

    add(
        "exclusions",
        not matrix.stale,
        f"{len(matrix.scope_android)} Android exclusions, none stale"
        if not matrix.stale
        else f"stale Android exclusions: {', '.join(matrix.stale)}",
    )

    n_required = len(required)
    rendered = render_markdown(matrix, ucm, datetime.date.today().isoformat(), n_required)
    if not compare_doc:
        add("doc", True, "not compared (--allow-pending)")
    else:
        path = repo / OUTPUT_MD
        try:
            committed = path.read_text(encoding="utf-8")
        except OSError:
            add("doc", False, f"{OUTPUT_MD} is absent; run --write")
        else:
            same = strip_date(committed) == strip_date(rendered)
            add("doc", same, f"{OUTPUT_MD} is up to date" if same else f"{OUTPUT_MD} differs from the generated matrix; run --write")
    return results, matrix, rendered


def emit(results) -> bool:
    ok_all = True
    for check, ok, detail in results:
        ok_all = ok_all and ok
        print(f"UCMATRIX | {check} | {'PASS' if ok else 'FAIL'} | {detail}")
    return ok_all


def total_line(ok: bool, matrix, failed=()) -> None:
    if matrix is None:
        print(f"UCMATRIX | total | {'PASS' if ok else 'FAIL'} | inputs unreadable")
        return
    c = matrix.counts()
    detail = ", ".join(f"{c[s]} {STATUS_LABELS[s]}" for s in STATUS_ORDER if c[s])
    prefix = f"failed: {', '.join(failed)}; " if failed else ""
    print(f"UCMATRIX | total | {'PASS' if ok else 'FAIL'} | {prefix}{sum(c.values())} ids: {detail}")


def main(argv=None, required=None, ucm=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="regenerate in memory and verify")
    parser.add_argument("--allow-pending", action="store_true", help="with --check: accept pending required ids")
    parser.add_argument("--write", action="store_true", help="rewrite docs/android/UC_MATRIX_ANDROID.md")
    parser.add_argument("--list-missing", action="store_true", help="print the MANQUANT and pending ids")
    parser.add_argument("--repo", default=str(DEFAULT_REPO), help="repository root")
    args = parser.parse_args(argv)
    if not (args.check or args.write or args.list_missing):
        parser.error("one of --check, --write, --list-missing is required")
    if args.allow_pending and not args.check:
        parser.error("--allow-pending only applies to --check")

    repo = Path(args.repo).resolve()

    if args.list_missing:
        try:
            matrix = build_matrix(repo, required, ucm)
        except InputProblem as exc:
            print(f"UCMATRIX | inputs | FAIL | {exc}")
            return 1
        for id_ in matrix.ids_with(ST_MISSING):
            print(f"MANQUANT {id_}")
        for id_ in matrix.ids_with(ST_PENDING):
            print(f"EN ATTENTE {id_}")
        return 1 if matrix.ids_with(ST_MISSING) else 0

    if args.write:
        results, matrix, rendered = run_checks(repo, True, False, required, ucm)
        if rendered is not None:
            out = repo / OUTPUT_MD
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text(rendered, encoding="utf-8")
            print(f"wrote {OUTPUT_MD}")
        results = [r for r in results if r[0] != "doc"]
        ok = emit(results)
        total_line(ok, matrix, [r[0] for r in results if not r[1]])
        return 0 if ok else 1

    results, matrix, _ = run_checks(repo, args.allow_pending, not args.allow_pending, required, ucm)
    ok = emit(results)
    total_line(ok, matrix, [r[0] for r in results if not r[1]])
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
