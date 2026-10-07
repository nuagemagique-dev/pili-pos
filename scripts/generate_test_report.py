#!/usr/bin/env python3
"""Regenerate the test totals of RAPPORT_TESTS_BASELINE_FR.md.

Runs `cargo test --workspace --no-fail-fast` (default features) and the
cert build of the app (`--no-default-features --features cert`), then
replaces the block between the TEST_TOTALS markers with one row per
test binary. The "Couche" column is the first line of the binary's
`//!` module doc, so a new test file documents itself.

Usage: scripts/generate_test_report.py   (from anywhere in the repo)
"""
import os
import pathlib
import re
import subprocess
import sys
from datetime import datetime
from zoneinfo import ZoneInfo

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOC = ROOT / "docs/SUBMISSION_ARTIFACTS/RAPPORT_TESTS_BASELINE_FR.md"
BEGIN = "<!-- TEST_TOTALS:BEGIN (scripts/generate_test_report.py) -->"
END = "<!-- TEST_TOTALS:END -->"

RUNNING = re.compile(r"^\s*Running (?:unittests )?(\S+) \(target/[^/]+/deps/([A-Za-z0-9_]+)-[0-9a-f]+\)")
RESULT = re.compile(r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored")


def run(args: list[str]) -> list[tuple[str, str, int, int, int]]:
    """(source, binary, passed, failed, ignored) per test binary."""
    env = dict(os.environ, CARGO_INCREMENTAL="0")
    out = subprocess.run(
        ["cargo", "test", *args, "--no-fail-fast"],
        cwd=ROOT, env=env, text=True,
        # One stream: "Running" goes to stderr, the results to stdout.
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    rows, current = [], None
    for line in out.stdout.splitlines():
        if m := RUNNING.match(line):
            current = m.groups()
        elif (m := RESULT.match(line)) and current:
            rows.append((*current, *map(int, m.groups())))
            current = None
    if not rows:
        sys.exit(f"cargo test {' '.join(args)} produced no results:\n{out.stdout[-2000:]}")
    return rows


def crate_dir(binary: str) -> pathlib.Path:
    """Directory of the crate a unit-test binary belongs to."""
    crate = ROOT / "crates" / binary.replace("_", "-")
    return crate if crate.is_dir() else ROOT / "src-tauri"


def layer(source: str, binary: str) -> str:
    """First sentence of the module doc of the test's source file."""
    if source.startswith("src/"):
        path = crate_dir(binary) / source
    else:
        # `cargo test` prints paths relative to the crate being tested.
        matches = [p for p in ROOT.glob(f"**/{source}")
                   if not {"target", ".claude", "node_modules"} & set(p.parts)]
        path = matches[0] if len(matches) == 1 else None
    if path is None or not path.is_file():
        return "-"
    paragraph = []
    for line in path.read_text().splitlines():
        if not line.startswith("//!"):
            if paragraph or line.strip():
                break
            continue
        text = line[3:].strip()
        if not text:
            if paragraph:
                break
            continue
        paragraph.append(text)
    if paragraph:
        text = " ".join(paragraph)
        # The first sentence is enough for a table cell.
        text = re.split(r"(?<=\.)\s", text, maxsplit=1)[0].rstrip(":")
        if len(text) > 160:
            text = text[:157].rsplit(" ", 1)[0] + "…"
        return text.replace("|", "\\|")
    return "-"


def name(source: str, binary: str) -> str:
    kind = "unit" if source.endswith(("lib.rs", "main.rs")) else "intégration"
    return f"`{binary}` ({kind})"


# Crates left out of the cert binary (build_and_hash.sh checks their
# symbols are absent): counted in one row, not itemised.
EXCLUDED = ("pos_hubrise", "pos_peppol")


def table(rows) -> list[str]:
    lines = [
        "| Binaire de test | Couche | Passants | Échecs | Ignorés |",
        "|---|---|---:|---:|---:|",
    ]
    excluded = [0, 0, 0]
    for source, binary, p, f, i in rows:
        if binary.startswith(EXCLUDED):
            excluded = [a + b for a, b in zip(excluded, (p, f, i))]
            continue
        if p + f + i == 0:
            continue
        lines.append(f"| {name(source, binary)} | {layer(source, binary)} | {p} | {f} | {i} |")
    if any(excluded):
        p, f, i = excluded
        lines.append(f"| Crates exclues de la build cert (unit) | intégrations hors v1.0.0 | {p} | {f} | {i} |")
    p, f, i = (sum(r[k] for r in rows) for k in (2, 3, 4))
    lines.append(f"| **Total** | | **{p}** | **{f}** | **{i}** |")
    return lines


def main() -> int:
    workspace = run(["--workspace"])
    cert = run(["-p", "pili-pos", "-p", "pos-engine", "--no-default-features", "--features", "cert"])
    now = datetime.now(ZoneInfo("Europe/Brussels")).strftime("%Y-%m-%d %H:%M")
    rev = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                         capture_output=True, text=True).stdout.strip()

    block = [
        BEGIN,
        "",
        f"Généré le {now} (Europe/Brussels) sur la révision `{rev}`.",
        "",
        "### Workspace, fonctionnalités par défaut (`cargo test --workspace`)",
        "",
        *table(workspace),
        "",
        "### Application, build cert (`cargo test -p pili-pos -p pos-engine --no-default-features --features cert`)",
        "",
        *table(cert),
        "",
        END,
    ]
    text = DOC.read_text()
    if BEGIN not in text or END not in text:
        sys.exit(f"markers missing in {DOC}")
    head, rest = text.split(BEGIN, 1)
    _, tail = rest.split(END, 1)
    DOC.write_text(head + "\n".join(block) + tail)

    failed = sum(r[3] for r in workspace + cert)
    print(f"✓ {DOC.relative_to(ROOT)}: {sum(r[2] for r in workspace)} workspace, "
          f"{sum(r[2] for r in cert)} cert, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
