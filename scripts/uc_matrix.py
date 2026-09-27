#!/usr/bin/env python3
"""uc_matrix.py - SPF Finances test traceability matrix generator/checker.

For every official test identifier described by the SPF documents under
`docs/` (Basic Set, Reports, Advanced Set, certification manual checklist,
FAQ producteurs), this script looks for a Rust test or a Playwright /
real-binary UI test whose name carries the identifier, or for a declared
exclusion in `scripts/uc_scope.toml`. It then writes (or, without --write,
only checks) `docs/SUBMISSION_ARTIFACTS/MATRICE_TRACABILITE_FR.md`.

This is the generated replacement for the hand-written COUVERTURE_*.md
files: it can never go stale because it is derived from the doc folders
and the test sources on every run.

python3 stdlib only (including tomllib, stdlib since 3.11).

Usage:
    scripts/uc_matrix.py                 # check only, exit 1 if MANQUANT
    scripts/uc_matrix.py --write         # also (re)write the matrix doc
    scripts/uc_matrix.py --list-missing  # print MANQUANT ids and exit
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

DEFAULT_ROOT = Path(__file__).resolve().parent.parent

BASIC_SET_SUBDIR = "docs/20260821_UC_BasicSet_POS_FDM_1.8"
REPORTS_SUBDIR = "docs/20260821_UC_Reports_1.3"
ADVANCED_SET_SUBDIR = "docs/20260821_UC_AdvancedSet_POS_FDM_1.6"
SCOPE_TOML = "scripts/uc_scope.toml"
OUTPUT_MD = "docs/SUBMISSION_ARTIFACTS/MATRICE_TRACABILITE_FR.md"

VALID_KINDS = ("hors_perimetre", "non_applicable", "couvert_par")

STATUS_LABELS = {
    "covered": "Couvert",
    "hors_perimetre": "Hors périmètre",
    "non_applicable": "Non applicable",
    "missing": "MANQUANT",
}


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------


@dataclass
class Item:
    id: str
    label: str


@dataclass
class Row:
    id: str
    label: str
    rust_tests: list = field(default_factory=list)
    ui_tests: list = field(default_factory=list)
    status: str = "missing"
    couvert_par: str = ""

    @property
    def status_label(self) -> str:
        if self.status == "couvert_par":
            return f"Couvert par {self.couvert_par}"
        return STATUS_LABELS[self.status]


@dataclass
class Report:
    sections: list  # list of (section_name, [Row, ...])
    missing_ids: list

    def counts(self) -> dict:
        counts = {"covered": 0, "hors_perimetre": 0, "non_applicable": 0, "couvert_par": 0, "missing": 0}
        for _, rows in self.sections:
            for row in rows:
                counts[row.status] += 1
        return counts


# ---------------------------------------------------------------------------
# ID collection - Basic Set / Reports / Advanced Set are scanned from disk;
# the manual checklist and the FAQ are a fixed vocabulary (no folder to scan).
# ---------------------------------------------------------------------------

BASIC_REPORTS_ID_RE = re.compile(r"^(M\d+[A-Z]?|Q\d+)(?:_(.*))?$")
ADVANCED_ID_RE = re.compile(r"^UC\d+")
T_FOLDER_RE = re.compile(r"^T(\d+)$")


def _top_level_dirs(base: Path):
    if not base.is_dir():
        return []
    return sorted((p for p in base.iterdir() if p.is_dir()), key=lambda p: p.name)


def humanize(remainder: str) -> str:
    """Mechanical, best-effort readable label derived from a folder name
    remainder (fallback for any id not in the hand-written FR label maps
    below, e.g. a folder added by a future UC set revision)."""
    if not remainder:
        return ""
    s = remainder.replace("_", " ").replace("%", " % ")
    s = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", " ", s)
    s = re.sub(r"(?<=[A-Z])(?=[A-Z][a-z])", " ", s)
    return " ".join(s.split())


def split_advanced_id(name: str):
    """Split an Advanced Set folder name into (short_id, description).

    The official folder names are `UC<digits>` optionally followed by a
    single uppercase-letter suffix (`UC221A`) or `bis` (`UC4103bis`), which
    only counts as part of the id when it is immediately followed by `_`
    or the end of the name - otherwise it is just the first letter of the
    description glued directly onto the digits (`UC4121CustomerInvoice`).
    """
    m = re.match(r"^(UC\d+)(.*)$", name)
    if not m:
        return None
    base, rest = m.group(1), m.group(2)
    if rest.startswith("bis") and (len(rest) == 3 or rest[3] == "_"):
        short_id = base + "bis"
        rest = rest[3:]
    elif rest[:1].isalpha() and rest[:1].isupper() and (len(rest) == 1 or rest[1] == "_"):
        short_id = base + rest[0]
        rest = rest[1:]
    else:
        short_id = base
    description = rest.lstrip("_")
    return short_id, description


def collect_basic_set_items(root: Path) -> list:
    base = root / BASIC_SET_SUBDIR
    items = []
    for d in _top_level_dirs(base):
        m = BASIC_REPORTS_ID_RE.match(d.name)
        if not m:
            continue
        id_ = m.group(1)
        label = BASIC_SET_LABELS.get(id_) or humanize(m.group(2) or "") or id_
        items.append(Item(id_, label))
    return items


def collect_reports_items(root: Path) -> list:
    base = root / REPORTS_SUBDIR
    items = []
    for d in _top_level_dirs(base):
        if d.name == "InputReport":
            for t in _top_level_dirs(d):
                tm = T_FOLDER_RE.match(t.name)
                if not tm:
                    continue
                items.append(Item(t.name, REPORTS_LABELS.get(t.name, t.name)))
            continue
        m = BASIC_REPORTS_ID_RE.match(d.name)
        if not m:
            continue
        id_ = m.group(1)
        label = REPORTS_LABELS.get(id_) or humanize(m.group(2) or "") or id_
        items.append(Item(id_, label))
    return items


def collect_advanced_set_items(root: Path) -> list:
    base = root / ADVANCED_SET_SUBDIR
    items = []
    for d in _top_level_dirs(base):
        if not ADVANCED_ID_RE.match(d.name):
            continue
        split = split_advanced_id(d.name)
        if not split:
            continue
        id_, description = split
        label = ADVANCED_SET_LABELS.get(id_) or humanize(description) or id_
        items.append(Item(id_, label))
    return items


# ---------------------------------------------------------------------------
# Fixed vocabularies: certification manual checklist + FAQ.
# ---------------------------------------------------------------------------

ROUND_ITEMS = [
    Item("ROUND1", "Arrondi cash, vente directe simple 11,52 EUR (situation 1, arrondi -0,02 EUR)"),
    Item("ROUND2", "Arrondi cash, vente directe simple 3,34 EUR (situation 2, arrondi +0,01 EUR)"),
    Item("ROUND3", "Arrondi cash, vente combinée 14,86 EUR (situation 3, arrondi -0,01 EUR)"),
    Item("ROUND4", "Arrondi avec chèques-repas non arrondis + solde cash (situation 4, arrondi +0,01 EUR)"),
    Item("ROUND5", "Arrondi multi-paiements: chèques-repas + carte de débit + cash (situation 5, arrondi -0,01 EUR)"),
]

# Section V of the certification manual (code -> short French message, taken
# verbatim from the manual's tb_messages table).
V_CODES = [
    ("CLIENT_CERT_NEAR_EXPIRATION", "Certificat client proche de l'expiration"),
    ("CLIENT_CERT_EXPIRED", "Certificat client expiré"),
    ("RTC_SYNC_FAILED", "Échec de synchronisation de l'horloge temps réel"),
    ("UPDATE_URLS_FAILED", "Échec de mise à jour des URLs"),
    ("UPDATE_TRUST_CERT_FAILED", "Échec de mise à jour des certificats de confiance"),
    ("UPDATE_TASK_LIST_FAILED", "Échec de téléchargement de la liste de tâches"),
    ("TASK_FEEDBACK_FAILED", "Échec de signalement du résultat de la tâche"),
    ("NOP_FAILED", "Échec de l'envoi du message NOP"),
    ("BUFFER_NEAR_FULL", "Utilisation du buffer du FDM au-delà de 70%"),
    ("SERVER_CERT_RESOLVE_FAILED", "Échec de résolution du certificat serveur"),
    ("TRANSACTION_UPLOAD_FAILED", "Échec répété (plus de 10 fois) d'envoi d'une transaction"),
    ("INITIALIZATION_FAILED", "Échec d'initialisation du FDM"),
    ("RTC_NOT_INITIALIZED", "Horloge temps réel du FDM non synchronisée"),
    ("CORRUPT_RECORD_ENCOUNTERED", "Transaction corrompue rencontrée en mémoire du FDM"),
    ("UPDATE_PARAMS_FAILED", "Échec de mise à jour des paramètres"),
    ("UPDATE_CLIENT_CERT_FAILED", "Échec de mise à jour du certificat client"),
    ("UPDATE_VAT_RATES_FAILED", "Échec de mise à jour des taux de TVA"),
    ("UPDATE_POS_ALLOWLIST_FAILED", "Échec de mise à jour de la liste d'autorisation"),
    ("DUPLICATE_REQUEST", "Requête en doublon reçue par le FDM"),
    ("BUFFER_FULL", "Buffer du FDM plein"),
    ("FDM_LOCKED", "FDM verrouillé"),
    ("TOO_MANY_MEMORY_ERRORS", "Trop de transactions corrompues en mémoire"),
    ("UNAUTHORIZED", "Requête non autorisée reçue par le FDM"),
    ("INVALID_REQUEST", "Requête invalide, non traitable par le FDM"),
    ("INTERNAL_ERROR", "Problème technique interne du FDM"),
    ("UNDEFINED_ERROR", "Erreur indéfinie rencontrée par le FDM"),
    ("UNDEFINED_OTHER", "Autre erreur, avertissement ou information non listée"),
    ("FDM_NOT_OPERATIONAL", "FDM non opérationnel"),
    ("UNKNOWN_POS", "POS non repris dans la liste d'autorisation du FDM"),
]
V_ITEMS = [Item(f"V_{code}", label) for code, label in V_CODES]

VI_ITEMS = [
    Item("VI_N", "Contenu du ticket de vente (event N, ticket de caisse TVA)"),
    Item("VI_REFUND", "Contenu du ticket de remboursement (event N REFUND)"),
    Item("VI_PROFORMA", "Contenu du ticket pro forma (aperçu d'addition)"),
    Item("VI_COPY", "Contenu du ticket copie (réimpression)"),
    Item("VI_TRAINING", "Contenu du ticket en mode formation (training)"),
    Item("VI_BAR", "Contenu des autres tickets sans désignation spécifique (cuisine, bar)"),
    Item("VI_Z", "Contenu du ticket de rapport Z"),
]

MANUAL_ITEMS = ROUND_ITEMS + V_ITEMS + VI_ITEMS

FAQ_ITEMS = [
    Item("FAQ_Q202", "signPreBill: usage et lien avec un signOrder préalable"),
    Item("FAQ_Q205", "Une transaction en cours doit-elle donner lieu à un event P ou N"),
    Item("FAQ_Q206", "Vente soldée à 0,00 EUR (remise de 100%): enregistrement"),
    Item("FAQ_Q207", "Règles d'usage de signPreBill et signOrder en gestion de table"),
    Item("FAQ_Q209", "Ticket de caisse TVA obligatoire même pour une vente à 0,00 EUR"),
    Item("FAQ_Q211", "Unicité de la référence de costCenter"),
    Item("FAQ_Q212", "Évolution de la référence de costCenter lors d'un transfert de table"),
    Item("FAQ_Q301", "Distinction entre motifs REFUND et CORRECTION (negQuantityReason)"),
    Item("FAQ_Q303", "Base de comptage des negQuantityReason dans le rapport X/Z"),
    Item("FAQ_Q401", "Money In/Out obligatoire dès que la caisse encaisse du cash, même sans tiroir"),
    Item("FAQ_Q402", "Enregistrement du rendu de monnaie et du cashback dans les objets payment"),
    Item("FAQ_Q403", "Encodage des pourboires (TIP) et des arrondis (ROUNDING) vers le FDM"),
    Item("FAQ_Q405", "signDrawerOpen: usage et absence de lien obligatoire avec signMoneyInOut"),
    Item("FAQ_Q501", "Périodicité du rapport Z de chiffre d'affaires (au moins une fois par jour)"),
    Item("FAQ_Q502", "Les events R de rapport sont exclus des timestamps de la période couverte"),
    Item("FAQ_Q503", "signCopy obligatoire pour toute réimpression, y compris une facture A4"),
    Item("FAQ_Q504", "Le rapport de chiffre d'affaires n'inclut pas les events de type Copy"),
    Item("FAQ_Q505", "Identification du deviceId (adresse MAC ou identifiant matériel unique)"),
    Item("FAQ_Q506", "La query Status du FDM est facultative"),
    Item("FAQ_Q507", "La mention TICKET DE CAISSE T.V.A. est réservée au ticket de caisse fiscal"),
    Item("FAQ_Q509", "Champ totalAmount du rapport Z (objet users): somme des transactionTotal"),
]


# ---------------------------------------------------------------------------
# Hand-written French labels for the current (2026-08-21) Basic Set, Reports
# and Advanced Set folder snapshots. Any id not listed here (e.g. a folder
# added by a future revision of the UC sets) falls back to `humanize()`.
# ---------------------------------------------------------------------------

BASIC_SET_LABELS = {
    "M110": "Vente directe (ticket de caisse TVA)",
    "M111": "Remboursement total d'une vente",
    "M112A": "Remboursement partiel combiné à une nouvelle vente",
    "M112B": "Remboursement partiel en ticket séparé",
    "M121": "Commande en gestion de table",
    "M122": "Changement de centre de coût (transfert de table)",
    "M123": "Addition provisoire (pre-bill)",
    "M130": "Mouvement de caisse (argent entrant/sortant)",
    "M131": "Ouverture du tiroir-caisse",
    "M132": "Correction de paiement",
    "M133": "Déclaration de fond de caisse (comptage)",
    "M140": "Entrée en service de l'utilisateur",
    "M141": "Sortie de service de l'utilisateur",
    "M150": "Facturation",
    "M160": "Copie d'un ticket de vente",
    "M161": "Copie d'une commande",
    "M170": "Entrée en service en mode formation",
    "M171": "Vente en mode formation",
    "M171B": "Vente en mode formation (style de requête @skip)",
    "M172": "Copie en mode formation",
    "Q100": "Requête de statut du FDM",
    "Q110": "Requête de statut, messages multilingues",
}

REPORTS_LABELS = {
    "M180": "Rapport X du chiffre d'affaires",
    "M181": "Rapport Z du chiffre d'affaires",
    "M182": "Rapport X des utilisateurs",
    "M183": "Rapport Z des utilisateurs",
    "M184": "Rapport Z combiné (chiffre d'affaires + utilisateurs)",
    "T1": "Rejeu d'entrée T1 (signOrder)",
    "T2": "Rejeu d'entrée T2 (signSale)",
    "T3": "Rejeu d'entrée T3 (signSale, remboursement)",
    "T4": "Rejeu d'entrée T4 (signSale)",
    "T5": "Rejeu d'entrée T5 (signOrder)",
    "T6": "Rejeu d'entrée T6 (signOrder)",
    "T7": "Rejeu d'entrée T7 (signPreBill)",
    "T8": "Rejeu d'entrée T8 (signSale)",
}

ADVANCED_SET_LABELS = {
    "UC200": "Vente directe d'un produit composite",
    "UC201": "Vente avec arrondi du paiement",
    "UC220": "Modification de prix sur une ligne de vente",
    "UC221A": "Modification de prix sur une ligne (cas A)",
    "UC221B": "Modification de prix sur une ligne (cas B)",
    "UC222": "Modification de prix sur un produit composite",
    "UC223": "Modification de prix au niveau de l'event de vente",
    "UC224": "Modifications de prix multiples sur une vente",
    "UC225A": "Modification de prix en vente indirecte (cas A)",
    "UC225B": "Modification de prix en vente indirecte (cas B)",
    "UC226": "Modification de prix",
    "UC229B": "Article gratuit en vente directe",
    "UC230": "Correction d'une ligne de vente avant paiement",
    "UC240": "Scission de table vers une nouvelle table",
    "UC241": "Scission de table vers une table existante",
    "UC242": "Scission de table vers deux nouvelles tables",
    "UC243": "Scission de table avec remise",
    "UC250": "Fusion de tables",
    "UC251": "Fusion de tables (variante)",
    "UC252": "Fusion de tables avec remise",
    "UC260": "Vente d'un bon à usage multiple (MPV)",
    "UC261": "Utilisation d'un bon à usage multiple (MPV)",
    "UC262": "Utilisation d'un bon à usage multiple (MPV, variante)",
    "UC270": "Vente d'un bon à usage unique (SPV)",
    "UC271": "Utilisation d'un bon à usage unique (SPV)",
    "UC280": "Produit composite",
    "UC290": "Paiement mixte en devise étrangère",
    "UC291": "Paiement en devise étrangère",
    "UC292": "Pourboire",
    "UC294": "Paiement en devise étrangère (variante)",
    "UC295": "Paiement en devise étrangère (variante)",
    "UC296": "Paiement en devise étrangère avec pourboire",
    "UC4100": "Compte client",
    "UC4101": "Transfert de table vers une chambre (changement de centre de coût)",
    "UC4102": "Facturation combinée à une modification de prix",
    "UC4103": "Scission d'addition avec changement de centre de coût",
    "UC4103bis": "Scission d'addition (variante bis)",
    "UC4104": "Modification de prix sur un menu tout compris",
    "UC4105": "Conversion d'un bon MPV en espèces",
    "UC4106": "Modification de prix sur un menu tout compris",
    "UC4107": "Libellés de TVA A/B/C",
    "UC4108": "Libellé de TVA X (acompte)",
    "UC4109": "Vente sans paiement",
    "UC4110": "Paiement anticipé (acompte)",
    "UC4111": "Paiement anticipé (pourcentage du prix)",
    "UC4112": "Produit vendu au poids",
    "UC4113": "Acompte sur réservation",
    "UC4114": "Caution pour événement (festival)",
    "UC4116": "Correction combinée à une modification de prix",
    "UC4117": "Code GTIN sur une ligne de produit",
    "UC4118": "Correction de paiement (avancé)",
    "UC4119": "Paiement scindé (split payment)",
    "UC4119bis": "Scission d'addition (bis)",
    "UC4120": "Supplément avec modification de prix",
    "UC4120bis": "Supplément sur une ligne de transaction (bis)",
    "UC4121": "Facture client",
    "UC4122": "Note de crédit",
    "UC4123": "Changement de centre de coût par chaise",
}


# ---------------------------------------------------------------------------
# Rust test scanning.
# ---------------------------------------------------------------------------

TEST_ATTR_RE = re.compile(r"#\[\s*(?:test|tokio::test)\s*\]")
ATTR_LINE_RE = re.compile(r"^#\[")
FN_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)
COVERS_RE = re.compile(r"//\s*covers:\s*(.+)", re.IGNORECASE)
BACKWARD_SCAN_LIMIT = 8


def _rust_test_dirs(root: Path) -> list:
    dirs = []
    crates_dir = root / "crates"
    if crates_dir.is_dir():
        for crate_dir in sorted(crates_dir.iterdir()):
            if not crate_dir.is_dir():
                continue
            for sub in ("tests", "src"):
                d = crate_dir / sub
                if d.is_dir():
                    dirs.append(d)
    for extra in ("src-tauri/tests", "src-tauri/src"):
        d = root / extra
        if d.is_dir():
            dirs.append(d)
    return dirs


@dataclass
class RustTest:
    fn_name: str
    file: str
    covers: list = field(default_factory=list)


def scan_rust_tests(root: Path) -> list:
    """Return every `#[test]`/`#[tokio::test]` fn found under the crates and
    src-tauri test/src trees, together with any `// covers: ID, ID` marker
    found directly above it."""
    tests = []
    for d in _rust_test_dirs(root):
        for rs_file in sorted(d.rglob("*.rs")):
            try:
                lines = rs_file.read_text(encoding="utf-8", errors="replace").splitlines()
            except OSError:
                continue
            rel = str(rs_file.relative_to(root))
            for i, line in enumerate(lines):
                m = FN_RE.match(line)
                if not m:
                    continue
                fn_name = m.group(1)
                is_test = False
                covers: list = []
                j = i - 1
                floor = max(-1, i - 1 - BACKWARD_SCAN_LIMIT)
                while j > floor:
                    stripped = lines[j].strip()
                    if TEST_ATTR_RE.search(stripped):
                        is_test = True
                    cm = COVERS_RE.search(stripped)
                    if cm:
                        covers.extend(x.strip() for x in cm.group(1).split(",") if x.strip())
                    if stripped == "" or ATTR_LINE_RE.match(stripped) or stripped.startswith("//"):
                        j -= 1
                        continue
                    break
                if is_test:
                    tests.append(RustTest(fn_name, rel, covers))
    return tests


# ---------------------------------------------------------------------------
# Playwright / real-binary UI test scanning.
# ---------------------------------------------------------------------------

CALL_RE = re.compile(
    r"\b((?:test\.describe|describe|test)(?:\.(?:only|skip))?)\s*\(\s*"
    r"(?:`([^`]*)`|'([^']*)'|\"([^\"]*)\")"
)
TAG_RE = re.compile(r"\[([A-Za-z0-9_]+)\]")


@dataclass
class UiTest:
    title: str
    file: str
    ids: list = field(default_factory=list)


def _brace_span(text: str, from_pos: int):
    brace_open = text.find("{", from_pos)
    if brace_open == -1:
        return None
    depth = 0
    in_str = None
    i = brace_open
    n = len(text)
    while i < n:
        c = text[i]
        if in_str:
            if c == "\\":
                i += 2
                continue
            if c == in_str:
                in_str = None
        elif c in ("'", '"', "`"):
            in_str = c
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return brace_open, i
        i += 1
    return None


def _extract_ui_tests_from_text(text: str) -> list:
    calls = []
    for m in CALL_RE.finditer(text):
        kind = m.group(1)
        title = m.group(2) if m.group(2) is not None else (m.group(3) if m.group(3) is not None else m.group(4))
        calls.append((m.start(), kind, title or ""))

    describe_spans = []
    test_entries = []
    for pos, kind, title in calls:
        tags = TAG_RE.findall(title)
        if "describe" in kind:
            span = _brace_span(text, pos)
            if span:
                describe_spans.append((span[0], span[1], tags))
        else:
            test_entries.append((pos, title, tags))

    results = []
    for pos, title, own_tags in test_entries:
        ids = set(own_tags)
        for start, end, tags in describe_spans:
            if start <= pos <= end:
                ids.update(tags)
        results.append((title, sorted(ids)))
    return results


def scan_ui_tests(root: Path) -> list:
    tests = []
    globs = [
        (root / "ui" / "e2e", "*.spec.ts"),
        (root / "ui" / "e2e" / "real-binary", "*.mjs"),
    ]
    for base, pattern in globs:
        if not base.is_dir():
            continue
        for f in sorted(base.glob(pattern)):
            try:
                text = f.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            rel = str(f.relative_to(root))
            for title, ids in _extract_ui_tests_from_text(text):
                if ids:
                    tests.append(UiTest(title, rel, ids))
    return tests


# ---------------------------------------------------------------------------
# Token-boundary matching.
# ---------------------------------------------------------------------------


def token_present(name: str, token: str) -> bool:
    """True when `token` occurs in `name` at a snake_case token boundary:
    the character before it (if any) and the character after it (if any)
    must be `_`, i.e. `token` is a whole underscore-delimited word, not part
    of a longer one (`m110` must not match `m1101`; `t3` must not match
    `t30` nor the "t3" inside `test3`)."""
    start = 0
    while True:
        idx = name.find(token, start)
        if idx == -1:
            return False
        before_ok = idx == 0 or name[idx - 1] == "_"
        after = idx + len(token)
        after_ok = after == len(name) or name[after] == "_"
        if before_ok and after_ok:
            return True
        start = idx + 1


# ---------------------------------------------------------------------------
# Scope (declared exclusions).
# ---------------------------------------------------------------------------


def load_scope(path: Path) -> dict:
    if not path.is_file():
        return {}
    with path.open("rb") as f:
        data = tomllib.load(f)
    for id_, entry in data.items():
        kind = entry.get("kind")
        if kind not in VALID_KINDS:
            raise ValueError(f"{path}: [{id_}] has invalid kind {kind!r} (expected one of {VALID_KINDS})")
        if not entry.get("reason"):
            raise ValueError(f"{path}: [{id_}] is missing a reason")
        if kind == "couvert_par" and not entry.get("by"):
            raise ValueError(f"{path}: [{id_}] has kind couvert_par but no by=")
    return data


# ---------------------------------------------------------------------------
# Matching ids to tests + scope, building the report.
# ---------------------------------------------------------------------------


def _match_row(item: Item, rust_tests: list, ui_tests: list, scope: dict) -> Row:
    row = Row(id=item.id, label=item.label)
    token = item.id.lower()
    for t in rust_tests:
        if token_present(t.fn_name.lower(), token) or item.id.upper() in {c.strip().upper() for c in t.covers}:
            row.rust_tests.append(t.fn_name)
    for t in ui_tests:
        if item.id.upper() in {i.upper() for i in t.ids}:
            row.ui_tests.append(t.title)

    entry = scope.get(item.id)
    # A feature outside the certified build stays out of scope even
    # when the development build has tests for it (e.g. M150 invoices).
    if entry and entry["kind"] == "hors_perimetre":
        row.status = "hors_perimetre"
        return row

    if row.rust_tests or row.ui_tests:
        row.status = "covered"
        return row

    if entry:
        kind = entry["kind"]
        row.status = kind
        if kind == "couvert_par":
            row.couvert_par = entry["by"]
        return row

    row.status = "missing"
    return row


def build_report(root: Path) -> Report:
    rust_tests = scan_rust_tests(root)
    ui_tests = scan_ui_tests(root)
    scope = load_scope(root / SCOPE_TOML)

    sources = [
        ("Basic Set", collect_basic_set_items(root)),
        ("Reports", collect_reports_items(root)),
        ("Advanced Set", collect_advanced_set_items(root)),
        ("Manuel", MANUAL_ITEMS),
        ("FAQ", FAQ_ITEMS),
    ]

    sections = []
    missing_ids = []
    for name, items in sources:
        rows = []
        for item in sorted(items, key=lambda it: it.id):
            row = _match_row(item, rust_tests, ui_tests, scope)
            rows.append(row)
            if row.status == "missing":
                missing_ids.append(row.id)
        sections.append((name, rows))

    return Report(sections=sections, missing_ids=sorted(missing_ids))


# ---------------------------------------------------------------------------
# Markdown rendering.
# ---------------------------------------------------------------------------


def _esc(s: str) -> str:
    return s.replace("|", "\\|").replace("\n", " ").strip()


def _fmt_cell(names: list, limit: int = 4) -> str:
    uniq = sorted(set(names))
    if not uniq:
        return "-"
    shown = uniq[:limit]
    cell = ", ".join(f"`{_esc(n)}`" for n in shown)
    if len(uniq) > limit:
        cell += f" +{len(uniq) - limit}"
    return cell


def get_git_date(root: Path) -> str | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(root), "log", "-1", "--format=%cd", "--date=format:%Y-%m-%d"],
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0:
        return None
    date = result.stdout.strip()
    return date or None


def render_markdown(report: Report, root: Path) -> str:
    date = get_git_date(root) or "date du dernier commit indisponible"
    lines = []
    lines.append("# Matrice de traçabilité des tests SPF")
    lines.append("")
    lines.append(
        "Ce document liste chaque identifiant de test décrit par le SPF Finances "
        "(Basic Set, Reports, Advanced Set, checklist du manuel de certification, "
        "FAQ producteurs) et indique s'il est couvert par un test automatisé "
        "(Rust ou interface utilisateur), déclaré hors périmètre, non applicable, "
        "ou couvert par les tests d'un autre identifiant."
    )
    lines.append("")
    lines.append(
        "Généré automatiquement par `scripts/uc_matrix.py` à partir des dossiers "
        "sous `docs/`, des sources de test (`crates/*/tests`, `crates/*/src`, "
        "`src-tauri/tests`, `src-tauri/src`, `ui/e2e/`) et des exclusions "
        "déclarées dans `scripts/uc_scope.toml`. Une réexécution sans "
        "changement de code ou de documentation produit un fichier identique."
    )
    lines.append("")
    lines.append(f"Date de génération (date du dernier commit git) : {date}")
    lines.append("")
    lines.append(
        "Légende du statut : Couvert (au moins un test automatisé trouvé), "
        "Hors périmètre (fonctionnalité absente de moyun-pos v1.0.0, exclusion "
        "documentée), Non applicable (ce cas précis ne peut pas se produire "
        "avec le comportement actuel), Couvert par X (les tests de "
        "l'identifiant X couvrent aussi ce cas), MANQUANT (aucun test "
        "automatisé trouvé et aucune exclusion déclarée)."
    )
    lines.append("")

    for name, rows in report.sections:
        lines.append(f"## {name}")
        lines.append("")
        lines.append("| ID | Intitulé | Tests Rust | Tests UI | Statut |")
        lines.append("|---|---|---|---|---|")
        for row in rows:
            lines.append(
                f"| {_esc(row.id)} | {_esc(row.label)} | {_fmt_cell(row.rust_tests)} | "
                f"{_fmt_cell(row.ui_tests)} | {row.status_label} |"
            )
        lines.append("")

    return "\n".join(lines).rstrip("\n") + "\n"


def write_markdown(report: Report, root: Path) -> Path:
    out_path = root / OUTPUT_MD
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(render_markdown(report, root), encoding="utf-8")
    return out_path


# ---------------------------------------------------------------------------
# CLI.
# ---------------------------------------------------------------------------


def exit_code_for(report: Report) -> int:
    return 1 if report.missing_ids else 0


def print_summary(report: Report) -> None:
    counts = report.counts()
    total = sum(counts.values())
    excluded = counts["hors_perimetre"] + counts["non_applicable"] + counts["couvert_par"]
    print(f"Total ids: {total}")
    for name, rows in report.sections:
        print(f"  {name}: {len(rows)}")
    print(f"Couvert: {counts['covered']}")
    print(f"Exclus (hors périmètre + non applicable + couvert par): {excluded}")
    print(f"  hors_perimetre: {counts['hors_perimetre']}")
    print(f"  non_applicable: {counts['non_applicable']}")
    print(f"  couvert_par: {counts['couvert_par']}")
    print(f"MANQUANT: {counts['missing']}")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="Write the matrix markdown file")
    parser.add_argument("--list-missing", action="store_true", help="Print MANQUANT ids grouped by source")
    parser.add_argument("--root", default=None, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)

    root = Path(args.root).resolve() if args.root else DEFAULT_ROOT
    report = build_report(root)

    if args.write:
        out_path = write_markdown(report, root)
        print(f"Wrote {out_path.relative_to(root)}")

    print_summary(report)

    if args.list_missing or report.missing_ids:
        print("IDs MANQUANT:")
        for name, rows in report.sections:
            missing_here = [r.id for r in rows if r.status == "missing"]
            if missing_here:
                print(f"  {name}: {', '.join(missing_here)}")

    return exit_code_for(report)


if __name__ == "__main__":
    sys.exit(main())
