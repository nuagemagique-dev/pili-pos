#!/usr/bin/env python3
"""Embed the moyun-pos evidence into RESULTATS_TESTS_UC_FR.md.

For every UC section of the results document that the fixture replays
cover, insert (or replace) a block with:
- the request moyun-pos itself built and archived for the UC's event
  (evidence/uc/<UC>_event<N>_<label>_request.json);
- the outcome of the field-by-field comparison with the fixture
  (evidence/uc/<UC>_check.json), with the intended deviations.

Run after scripts/capture_uc_evidence.sh (it calls this script). The
blocks sit between markers, so a rerun replaces them in place.

Usage: scripts/embed_uc_evidence.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOC = ROOT / "docs/SUBMISSION_ARTIFACTS/RESULTATS_TESTS_UC_FR.md"
EVIDENCE = ROOT / "docs/SUBMISSION_ARTIFACTS/evidence/uc"

START = "<!-- moyun-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->"
END = "<!-- moyun-pos-evidence:end -->"

# UC -> (test that replays it, check files)
BASIC = {
    "M110": ("spf_e2e_fixtures.rs::m110_sale", ["M110"]),
    "M111": ("spf_e2e_fixtures.rs::m111_full_refund", ["M111"]),
    "M112B": (
        "spf_e2e_fixtures.rs::m112b_partial_refund_as_a_separate_ticket",
        ["M112B_1", "M112B_2"],
    ),
    "M121": ("spf_e2e_fixtures.rs::m121_table_order", ["M121"]),
    "M123": ("spf_e2e_fixtures.rs::m123_pre_bill", ["M123"]),
    "M130": ("spf_e2e_fixtures.rs::m130_money_in", ["M130"]),
    "M131": ("spf_e2e_fixtures.rs::m131_drawer_open", ["M131"]),
    "M140": ("spf_e2e_fixtures.rs::m140_work_in", ["M140"]),
    "M141": ("spf_e2e_fixtures.rs::m141_work_out", ["M141"]),
    "M160": ("spf_e2e_fixtures.rs::m160_copy_of_a_sale", ["M160"]),
    "M161": ("spf_e2e_fixtures.rs::m161_copy_of_an_order", ["M161"]),
    "M170": ("spf_e2e_fixtures.rs::m170_work_in_in_training", ["M170"]),
    "M171": ("spf_e2e_fixtures.rs::m171_sale_in_training", ["M171"]),
    "M172": ("spf_e2e_fixtures.rs::m172_copy_in_training", ["M172"]),
}

REPORTS = {
    "M180": "spf_e2e_report_fixtures.rs::m180_turnover_x_of_the_scenario_day",
    "M182": "spf_e2e_report_fixtures.rs::m182_user_x_of_the_scenario_day",
    "M184": "spf_e2e_report_fixtures.rs::m184_combined_z_of_the_scenario_day",
}
# M181 and M183 are signed by the combined Z of M184.
VIA_M184 = {"M181": "signReportTurnoverZ", "M183": "signReportUserZ"}

REASONS_FR = {
    "dine-in sales are settled from a table and carry costCenter TABLE": (
        "une vente en salle est encaissée depuis sa table et porte le "
        "centre de coût TABLE"
    ),
    "moyun-pos references the refunded ticket on every refund "
    "(FLUX_REMBOURSEMENT_FR.md); the fixture's partial refund carries no reference": (
        "moyun-pos référence le ticket remboursé sur chaque remboursement "
        "(*FLUX_REMBOURSEMENT_FR.md*) ; le remboursement partiel de la "
        "fixture n'en porte pas"
    ),
    "orders are not printed in v1.0.0 (no kitchen ticket), so the medium is NONE": (
        "les commandes ne sont pas imprimées en v1.0.0 (pas de ticket "
        "cuisine) : le support est NONE"
    ),
    "copies are printed in training mode too, so the medium is PAPER": (
        "les copies sont imprimées aussi en mode formation : le support "
        "est PAPER"
    ),
}

CHECKED_FR = {
    "business fields equal; identification fields by format; names present": (
        "champs métier identiques à la fixture ; champs d'identification "
        "(posId, deviceId, numéro de ticket, dates, période de booking) "
        "contrôlés sur leur format ; libellés présents"
    ),
}

REPORT_DEVIATIONS = [
    (
        "`transactions[P]`",
        "3 tickets, 198 €",
        "4 tickets, 250 €",
        "chaque event P de la période est compté (§2.2.2 `ticketCount`) : "
        "T1, T5, T6 et T7 ; point signalé au SPF",
    ),
    (
        "`payments[].id` / `name`",
        "identifiants et libellés du scénario",
        "identifiants et libellés de moyun-pos",
        "propres à chaque POS",
    ),
    (
        "`payments[].type` (carte)",
        "`CARD_CREDIT`",
        "`CARD_DEBIT`",
        "l'opérateur ne distingue pas carte de débit et de crédit",
    ),
    (
        "`departments[].name`",
        "libellés du scénario",
        "libellés français du catalogue",
        "propres au catalogue",
    ),
    (
        "`posDevices` / `fdmDevices`",
        "appareils du scénario",
        "ce terminal et le FDM de test",
        "propres à l'installation",
    ),
]


def events_of(uc: str) -> list[Path]:
    pat = re.compile(rf"^{re.escape(uc)}_event(\d+)_([A-Z])_request\.json$")
    found = []
    for p in EVIDENCE.iterdir():
        m = pat.match(p.name)
        if m:
            found.append((int(m.group(1)), p))
    return [p for _, p in sorted(found)]


def fmt(v) -> str:
    if v is None:
        return "—"
    if isinstance(v, (dict, list)):
        return "`" + json.dumps(v, ensure_ascii=False, separators=(", ", ": ")) + "`"
    return f"`{v}`"


def json_block(path: Path) -> str:
    data = json.loads(path.read_text(encoding="utf-8"))
    return "```json\n" + json.dumps(data, indent=2, ensure_ascii=False) + "\n```"


def file_list(paths: list[Path]) -> str:
    return ", ".join(f"`evidence/uc/{p.name}`" for p in paths)


def basic_block(uc: str) -> str | None:
    test, checks = BASIC[uc]
    events = events_of(uc)
    if not events:
        return None
    main = events[-1]
    out = [
        START,
        f"**Preuve moyun-pos (build `cert`)** — rejeu de la fixture par "
        f"`src-tauri/tests/{test}`. Requête **construite et archivée par "
        f"moyun-pos** pour l'event de l'UC (`evidence/uc/{main.name}`) :",
        "",
        json_block(main),
        "",
    ]
    if len(events) > 1:
        out += [
            f"Events préalables du scénario, archivés de même : "
            f"{file_list(events[:-1])}. Réponses FDM archivées : fichiers "
            f"`*_response.json` correspondants.",
            "",
        ]
    for c in checks:
        path = EVIDENCE / f"{c}_check.json"
        if not path.exists():
            sys.exit(f"missing {path}: run scripts/capture_uc_evidence.sh first")
        check = json.loads(path.read_text(encoding="utf-8"))
        checked = CHECKED_FR.get(check["checked"])
        if checked is None:
            sys.exit(f"{path.name}: no French wording for {check['checked']!r}")
        out.append(f"**Comparaison avec la fixture** (`evidence/uc/{path.name}`) : {checked}.")
        out.append("")
        devs = check.get("deviations", [])
        if devs:
            out += [
                "Écarts assumés :",
                "",
                "| Champ | Fixture | moyun-pos | Motif |",
                "|---|---|---|---|",
            ]
            for d in devs:
                reason = REASONS_FR.get(d["reason"])
                if reason is None:
                    sys.exit(f"{path.name}: no French wording for {d['reason']!r}")
                out.append(
                    f"| `{d['path']}` | {fmt(d.get('fixture'))} | {fmt(d.get('moyunPos'))} | {reason} |"
                )
            out.append("")
        else:
            out += ["Aucun écart.", ""]
    out.append(END)
    return "\n".join(out)


def report_block(uc: str) -> str | None:
    if uc in VIA_M184:
        events = [p for p in events_of("M184") if p.name.endswith("_R_request.json")]
        if not events:
            return None
        return "\n".join(
            [
                START,
                f"**Preuve moyun-pos (build `cert`)** — cet UC est signé par "
                f"le Z combiné de M184 (mutation `{VIA_M184[uc]}`), rejoué par "
                f"`src-tauri/tests/{REPORTS['M184']}` : voir {file_list(events)} "
                f"et la section M184.",
                END,
            ]
        )
    events = events_of(uc)
    reports = [p for p in events if p.name.endswith("_R_request.json")]
    if not reports:
        return None
    day = [p for p in events if p not in reports]
    out = [
        START,
        f"**Preuve moyun-pos (build `cert`)** — la journée du scénario "
        f"(InputReport T1–T8) est jouée par les vraies commandes, puis le "
        f"rapport est construit par moyun-pos à partir de son archive "
        f"fiscale et comparé à la fixture par `src-tauri/tests/{REPORTS[uc]}`.",
        "",
    ]
    for r in reports:
        out += [f"Requête du rapport (`evidence/uc/{r.name}`) :", "", json_block(r), ""]
    out += [
        f"Events de la journée, archivés de même : {file_list(day)}.",
        "",
        "Comparés exactement : nombre d'events et montants par label, "
        "départements, TVA (taux, base, TVA, total), paiements, quantités "
        "négatives, ouvertures de tiroir, totaux, paiements et présences "
        "par utilisateur. Écarts assumés :",
        "",
        "| Champ | Fixture | moyun-pos | Motif |",
        "|---|---|---|---|",
    ]
    for row in REPORT_DEVIATIONS:
        out.append("| " + " | ".join(row) + " |")
    out += ["", END]
    return "\n".join(out)


def sections(text: str) -> list[tuple[str, int, int]]:
    """(uc, start, end) of every '## <UC>_…' section."""
    heads = [(m.group(1), m.start()) for m in re.finditer(r"^## (M\d+[A-Z]?)_", text, re.M)]
    bounds = [m.start() for m in re.finditer(r"^#{1,2} ", text, re.M)]
    out = []
    for uc, start in heads:
        end = next((b for b in bounds if b > start), len(text))
        out.append((uc, start, end))
    return out


def main() -> None:
    text = DOC.read_text(encoding="utf-8")
    # Drop previous blocks.
    text = re.sub(
        re.escape(START) + r".*?" + re.escape(END) + r"\n*", "", text, flags=re.S
    )
    embedded = []
    for uc, start, end in reversed(sections(text)):
        if uc in BASIC:
            block = basic_block(uc)
        elif uc in REPORTS or uc in VIA_M184:
            block = report_block(uc)
        else:
            block = None
        if block is None:
            continue
        section = text[start:end].rstrip("\n")
        tail = ""
        if section.endswith("\n---"):
            section, tail = section[: -len("\n---")].rstrip("\n"), "---\n\n"
        text = text[:start] + section + "\n\n" + block + "\n\n" + tail + text[end:]
        embedded.append(uc)
    DOC.write_text(text, encoding="utf-8")
    print(f"embedded evidence for {len(embedded)} UC: {', '.join(reversed(embedded))}")


if __name__ == "__main__":
    main()
