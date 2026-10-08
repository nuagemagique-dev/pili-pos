#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P5 gate
#
# The gate of the UI lane (phase P5, PD216): everything p4_gate.sh --app checks, plus the
# UI rules (command parity, ViewModel and clock rules, money arithmetic, string twins).
# Same output contract as p2_gate.sh to p4_gate.sh. Prints one line per check:
#     GATE | <check> | PASS|FAIL|SKIP | <detail>
# runs every check even after a failure, ends with exactly one
#     GATE | total | PASS|FAIL | <n> failed (<m> checks)
# and exits 1 on any FAIL. The output of a failed check is printed on stderr.
#
# Usage: scripts/android/p5_gate.sh [--fast] [--exit] [--app-e2e] [--linux-repro]
#                                   [--base REF] [--allow REGEX] [--min-app-tests N]
#                                   [--parity-strict]
#   --fast, --exit, --app-e2e, --linux-repro, --base, --allow, --min-app-tests
#                 passed to p4_gate.sh --app (--fast skips the self-tests and the :app
#                 Gradle checks there and here)
#   --parity-strict   check_ui_parity.py runs with --strict (the P5b exit): a pending or
#                 stale command, and a parity table token without a call site, fail
#
# Checks, in order
#   Step 1        every check line of p4_gate.sh --app with the options above (its total
#                 line is not printed; its checks and failures are counted; a run that
#                 prints no total line is a FAIL of the check p4-gate)
#   Step 2, own checks
#   ui-parity     check_ui_parity.py [--strict]; detail: its total line
#   ui-contracts  the .kt files under android/app/src/main/kotlin/dev/nuagemagique/pilipos/
#                 ui/ and feature/, except ui/theme/, with comments removed: FAIL on
#                 hiltViewModel, EngineHolder, EngineFactory, EngineNatives,
#                 NavHostController, getIdentifier(, ZoneId.systemDefault,
#                 TimeZone.getDefault, Locale.getDefault, a Color(0x literal and a numeric
#                 .sp literal such as 14.sp (theme tokens only). Detail: file:line
#   ui-money      the same files with comments and string literals removed: FAIL on a line
#                 where an identifier containing cents or Cents is an operand of a binary
#                 * / % + - or of *= /= %= += -=, on sumOf or fold whose lambda or
#                 arguments use such an identifier, on .times( .div( .rem( .plus( .minus(
#                 applied to one (receiver or argument), and on roundToInt, roundToLong or
#                 Math.round in a line with one. Comparisons, unary minus, ++ and --, and
#                 the lambda arrow -> are not operators. A line of ui/format/Money.kt that
#                 ends with the comment '// money-format-only' is exempt (PD208); the
#                 marker anywhere else is a FAIL
#   ui-strings    every android/app/src/main/res/values/strings_ui_*.xml has twins in
#                 values-en/ and values-zh/ with exactly the same set of name attributes,
#                 and every name of strings_ui_<area>.xml starts with ui_<area>_; SKIP
#                 while no such file exists
#   Full mode only (not --fast):
#   selftest-p5-gate     bash scripts/android/test_p5_gate.sh
#   selftest-ui-parity   python3 scripts/android/test_check_ui_parity.py
#
# Environment (for the self-test): PILI_P5_P4_GATE (path of the p4 gate run in step 1,
# default p4_gate.sh next to this script), PILI_P5_ROOT (root of the source checks and of
# check_ui_parity.py --repo, default the repository of this script), PILI_P5_NESTED=1 (the
# self-test checks are SKIP).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

TOOLS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SELF="$TOOLS/$(basename "${BASH_SOURCE[0]}")"
P4_GATE="${PILI_P5_P4_GATE:-$TOOLS/p4_gate.sh}"
ROOT="${PILI_P5_ROOT:-$(cd "$TOOLS/../.." && pwd -P)}"

fast=0
parity_strict=0
p4_args=(--app)
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast)
        fast=1
        p4_args+=(--fast)
        ;;
    --exit | --app-e2e | --linux-repro) p4_args+=("$1") ;;
    --parity-strict) parity_strict=1 ;;
    --base | --allow | --min-app-tests)
        [[ $# -ge 2 ]] || { echo "$1 needs a value" >&2; exit 2; }
        p4_args+=("$1" "$2")
        shift
        ;;
    -h | --help)
        sed -n '2,/^set -uo/p' "$SELF" | sed '$d; s/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "unknown argument: $1" >&2
        exit 2
        ;;
    esac
    shift
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
n_checks=0
n_failed=0

clean_detail() {
    local d
    d="$(tr '\n|' ' /' | sed 's/  */ /g; s/^ //; s/ $//' | cut -c1-160)"
    printf '%s' "${d:-ok}"
}

last_line() { grep -v '^[[:space:]]*$' "$1" | tail -n 1 | clean_detail; }

# gate <check> <command...>: PASS when the command exits 0.
gate() {
    local name="$1" log
    shift
    n_checks=$((n_checks + 1))
    log="$TMP/check.$n_checks.log"
    if "$@" >"$log" 2>&1; then
        printf 'GATE | %s | PASS | %s\n' "$name" "$(last_line "$log")"
    else
        n_failed=$((n_failed + 1))
        printf 'GATE | %s | FAIL | %s\n' "$name" "$(last_line "$log")"
        tail -n 40 "$log" | sed 's/^/    | /' >&2
    fi
}

skip() { # <check> <detail>
    n_checks=$((n_checks + 1))
    printf 'GATE | %s | SKIP | %s\n' "$1" "$2"
}

# ── step 1: p4_gate.sh --app ────────────────────────────────────
# Its check lines are passed through, its total line is dropped.
run_p4_gate() {
    local out="$TMP/p4.out" rc line total=0
    if [[ ! -f "$P4_GATE" ]]; then
        n_checks=$((n_checks + 1))
        n_failed=$((n_failed + 1))
        printf 'GATE | p4-gate | FAIL | %s is absent\n' "$P4_GATE"
        return
    fi
    bash "$P4_GATE" ${p4_args[@]+"${p4_args[@]}"} >"$out"
    rc=$?
    while IFS= read -r line; do
        case "$line" in
        "GATE | total | "*) total=1 ;;
        "GATE | "*)
            printf '%s\n' "$line"
            n_checks=$((n_checks + 1))
            case "$line" in "GATE | "*" | FAIL | "*) n_failed=$((n_failed + 1)) ;; esac
            ;;
        esac
    done <"$out"
    if [[ "$total" -eq 0 ]]; then
        n_checks=$((n_checks + 1))
        n_failed=$((n_failed + 1))
        printf 'GATE | p4-gate | FAIL | p4_gate.sh printed no total line (exit %d)\n' "$rc"
    fi
}
run_p4_gate

# ── ui-parity ───────────────────────────────────────────────────
ui_parity() {
    local args=(--repo "$ROOT") rc
    [[ "$parity_strict" -eq 0 ]] || args+=(--strict)
    python3 -I "$TOOLS/check_ui_parity.py" "${args[@]}" >"$TMP/parity.out"
    rc=$?
    grep -E '\| (MISSING|FAIL) \|' "$TMP/parity.out" | head -n 30
    tail -n 1 "$TMP/parity.out"
    return $rc
}
gate ui-parity ui_parity

# ── ui-contracts, ui-money, ui-strings (one Python program) ─────
IFS= read -r -d '' UI_CHECKS <<'PY'
import os
import re
import sys
import xml.etree.ElementTree as ET

check, root, tools = sys.argv[1], sys.argv[2], sys.argv[3]
sys.path.insert(0, tools)
from check_ui_parity import strip_kotlin  # noqa: E402

PKG = "android/app/src/main/kotlin/dev/nuagemagique/pilipos"
RES = "android/app/src/main/res"
MONEY_FILE = PKG + "/ui/format/Money.kt"
MARKER = "// money-format-only"


def kotlin_files():
    """(repo-relative path, absolute path) of the files under ui/ and feature/, except ui/theme/."""
    for sub in ("ui", "feature"):
        base = os.path.join(root, PKG, sub)
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames.sort()
            rel_dir = os.path.relpath(dirpath, root).replace(os.sep, "/")
            if (rel_dir + "/").startswith(PKG + "/ui/theme/"):
                dirnames[:] = []
                continue
            for fn in sorted(filenames):
                if fn.endswith(".kt"):
                    yield rel_dir + "/" + fn, os.path.join(dirpath, fn)


def read(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read()


def report(hits, what):
    if hits:
        print("\n".join(hits[:40]))
        print("%d %s: %s" % (len(hits), what, "; ".join(hits[:3])))
        sys.exit(1)


# ── ui-contracts ────────────────────────────────────────────────
CONTRACTS = [
    ("hiltViewModel", re.compile(r"\bhiltViewModel\b")),
    ("EngineHolder", re.compile(r"\bEngineHolder\b")),
    ("EngineFactory", re.compile(r"\bEngineFactory\b")),
    ("EngineNatives", re.compile(r"\bEngineNatives\b")),
    ("NavHostController", re.compile(r"\bNavHostController\b")),
    ("getIdentifier(", re.compile(r"\bgetIdentifier\s*\(")),
    ("ZoneId.systemDefault", re.compile(r"\bZoneId\s*\.\s*systemDefault\b")),
    ("TimeZone.getDefault", re.compile(r"\bTimeZone\s*\.\s*getDefault\b")),
    ("Locale.getDefault", re.compile(r"\bLocale\s*\.\s*getDefault\b")),
    ("Color(0x literal", re.compile(r"\bColor\s*\(\s*0[xX]")),
    ("numeric .sp literal", re.compile(r"(?<![\w.])\d[\d_]*(?:\.\d+)?[fF]?\.sp\b")),
]


def contracts():
    hits = []
    for rel, path in kotlin_files():
        lines = strip_kotlin(read(path), drop_strings=False).split("\n")
        for no, ln in enumerate(lines, 1):
            for name, rx in CONTRACTS:
                if rx.search(ln):
                    hits.append("%s:%d %s" % (rel, no, name))
    report(hits, "ui contract violation(s)")
    print("no contract violation in the ui/ and feature/ sources")


# ── ui-money ────────────────────────────────────────────────────
WORD = re.compile(r"[A-Za-z_]\w*")
NON_OPERAND_WORDS = {"return", "throw", "in", "is", "else", "if", "when", "while", "do", "yield", "as", "to"}
APPLIED_AFTER = re.compile(r"\s*\??\.\s*(?:times|div|rem|plus|minus)\s*\(")
APPLIED_BEFORE = re.compile(r"\.\s*(?:times|div|rem|plus|minus)\s*\(\s*$")
ROUNDING = re.compile(r"\b(?:roundToInt|roundToLong)\b|\bMath\s*\.\s*round\b")
FOLD = re.compile(r"\b(?:sumOf|fold)\b")


def is_cents(word):
    return "cents" in word or "Cents" in word


def match_forward(s, i, op, cl):
    depth = 0
    for k in range(i, len(s)):
        if s[k] == op:
            depth += 1
        elif s[k] == cl:
            depth -= 1
            if depth == 0:
                return k
    return len(s) - 1


def match_backward(s, i, op, cl):
    depth = 0
    for k in range(i, -1, -1):
        if s[k] == cl:
            depth += 1
        elif s[k] == op:
            depth -= 1
            if depth == 0:
                return k
    return 0


def operand_end(ln, i):
    """True when ln[i] (a non-space character) can end an operand."""
    ch = ln[i]
    if ch == ")":
        # `if (c) -x`: the parenthesis closes a condition, the sign is unary
        o = match_backward(ln, i, "(", ")")
        m = re.search(r"([A-Za-z_]\w*)\s*$", ln[:o])
        return not (m and m.group(1) in ("if", "while", "for", "when", "catch"))
    if ch.isalnum() or ch in "_]\"'":
        m = re.search(r"([A-Za-z_]\w*)$", ln[: i + 1])
        return not (m and m.group(1) in NON_OPERAND_WORDS)
    return False


ARITH_MEMBERS = {"times", "div", "rem", "plus", "minus"}
MEMBER = re.compile(r"\??\.\s*([A-Za-z_]\w*)")


def call_suffixes(ln, e):
    """Skip (...), [...] and !! suffixes starting at e."""
    while e < len(ln):
        if ln[e] == "(":
            e = match_forward(ln, e, "(", ")") + 1
        elif ln[e] == "[":
            e = match_forward(ln, e, "[", "]") + 1
        elif ln.startswith("!!", e):
            e += 2
        else:
            break
    return e


def extend_forward(ln, e):
    """End of the operand that goes on at e: suffixes, then .name / ?.name members with their call suffixes.
    .times/.div/.rem/.plus/.minus are not consumed, they keep their own rule."""
    e = call_suffixes(ln, e)
    while True:
        mm = MEMBER.match(ln, e)
        if not mm or mm.group(1) in ARITH_MEMBERS:
            return e
        e = call_suffixes(ln, mm.end())


def chain_bounds(ln, m):
    """Start and end of the operand around the identifier match m (receiver chain and call suffixes)."""
    s = m.start()
    e = extend_forward(ln, m.end())
    while s > 0 and ln[s - 1] == ".":
        p = s - 1
        if p > 0 and ln[p - 1] == "?":
            p -= 1
        if p == 0:
            break
        q = p - 1
        if ln[q] in ")]":
            q = match_backward(ln, q, "(" if ln[q] == ")" else "[", ln[q])
            if q == 0:
                s = 0
                break
            q -= 1
        if q >= 0 and (ln[q].isalnum() or ln[q] == "_"):
            while q > 0 and (ln[q - 1].isalnum() or ln[q - 1] == "_"):
                q -= 1
            s = q
        else:
            break
    return s, e


def left_operator(ln, s):
    """(operator, index) of the binary operator just before position s, or None."""
    i = s - 1
    while i >= 0 and ln[i] in " \t":
        i -= 1
    if i < 0:
        return None
    ch = ln[i]
    if ch == "=":
        if i >= 1 and ln[i - 1] in "*/%+-":
            return ln[i - 1] + "=", i - 1
        return None
    if ch in "*/%+-":
        if ch in "+-" and i >= 1 and ln[i - 1] == ch:
            return None  # ++ or -- applied before the operand
        j = i - 1
        while j >= 0 and ln[j] in " \t":
            j -= 1
        if j >= 0 and operand_end(ln, j):
            return ch, i
    return None


def right_operator(ln, e):
    """(operator, index after it) of the binary operator just after position e, or None."""
    j = e
    while j < len(ln) and ln[j] in " \t":
        j += 1
    two = ln[j : j + 2]
    if two in ("*=", "/=", "%=", "+=", "-="):
        return two, j + 2
    if j < len(ln) and ln[j] in "*/%":
        return ln[j], j + 1
    if j < len(ln) and ln[j] in "+-" and two not in ("++", "--", "->"):
        return ln[j], j + 1
    return None


def string_next_to(ln, side, pos):
    """True when the operand on the far side of an operator is a string literal (drop_strings keeps the quotes)."""
    if side == "right":
        j = pos
        while j < len(ln) and ln[j] in " \t":
            j += 1
        return j < len(ln) and ln[j] == '"'
    j = pos - 1
    while j >= 0 and ln[j] in " \t":
        j -= 1
    return j >= 0 and ln[j] == '"'


def operator_around(ln, s, e, called):
    """Binary operator that has the operand ln[s:e] on one side, or None. A called identifier
    (formatCents(x) + "...") is exempt only when the operator is + and the other operand is a string literal."""
    left = left_operator(ln, s)
    if left and not (called and left[0] == "+" and string_next_to(ln, "left", left[1])):
        return left[0]
    right = right_operator(ln, e)
    if right and not (called and right[0] == "+" and string_next_to(ln, "right", right[1])):
        return right[0]
    return None


GROUP_KEYWORDS = ("if", "while", "for", "when", "catch")


def is_bare_group(ln, k):
    """True when the `(` at k opens a plain parenthesised expression: not a call (after an identifier,
    `)`, `]` or `>`) and not a condition (after if/while/for/when/catch)."""
    j = k - 1
    while j >= 0 and ln[j] in " \t":
        j -= 1
    if j < 0:
        return True
    if ln[j] in ")]>":
        return False
    if ln[j].isalnum() or ln[j] == "_":
        m = re.search(r"([A-Za-z_]\w*)$", ln[: j + 1])
        word = m.group(1) if m else ""
        return word in NON_OPERAND_WORDS and word not in GROUP_KEYWORDS
    return True


def enclosing_groups(ln, pos):
    """(start, end) of every bare parenthesised group around pos, innermost first."""
    out = []
    depth = 0
    for k in range(pos - 1, -1, -1):
        if ln[k] == ")":
            depth += 1
        elif ln[k] == "(":
            if depth:
                depth -= 1
            elif is_bare_group(ln, k):
                out.append((k, match_forward(ln, k, "(", ")")))
    return out


def line_findings(ln):
    found = []
    for m in WORD.finditer(ln):
        if not is_cents(m.group(0)):
            continue
        s, e = chain_bounds(ln, m)
        op = operator_around(ln, s, e, ln.startswith("(", m.end()))
        if not op:
            for gs, ge in enclosing_groups(ln, m.start()):
                op = operator_around(ln, gs, extend_forward(ln, ge + 1), False)
                if op:
                    break
        if op:
            found.append("operator %s on %s" % (op, m.group(0)))
        elif APPLIED_AFTER.match(ln, e) or APPLIED_BEFORE.search(ln[:s]):
            found.append("times/div/rem/plus/minus on %s" % m.group(0))
        elif ROUNDING.search(ln):
            found.append("rounding of %s" % m.group(0))
    return found


def fold_findings(text):
    """(line, text) of sumOf and fold calls whose arguments or lambda use a cents identifier."""
    out = []
    for m in FOLD.finditer(text):
        i = m.end()
        while i < len(text) and text[i] in " \t":
            i += 1
        if i < len(text) and text[i] == "<":
            i = text.find(">", i)
            if i < 0:
                continue
            i += 1
            while i < len(text) and text[i] in " \t":
                i += 1
        spans = []
        if i < len(text) and text[i] == "(":
            close = match_forward(text, i, "(", ")")
            spans.append((i, close + 1))
            i = close + 1
            while i < len(text) and text[i] in " \t\r\n":
                i += 1
        if i < len(text) and text[i] == "{":
            spans.append((i, match_forward(text, i, "{", "}") + 1))
        for a, b in spans:
            hit = next((w.group(0) for w in WORD.finditer(text, a, b) if is_cents(w.group(0))), None)
            if hit:
                out.append((text.count("\n", 0, m.start()) + 1, "%s uses %s" % (m.group(0), hit)))
                break
    return out


def money():
    hits = []
    for rel, path in kotlin_files():
        raw = read(path)
        raw_lines = raw.split("\n")
        exempt = set()
        for no, ln in enumerate(raw_lines, 1):
            if "money-format-only" in ln:
                if rel == MONEY_FILE and ln.rstrip().endswith(MARKER):
                    exempt.add(no)
                else:
                    where = "only the end of a line of ui/format/Money.kt" if rel == MONEY_FILE else "outside ui/format/Money.kt"
                    hits.append("%s:%d money-format-only marker %s" % (rel, no, where))
        text = strip_kotlin(raw, drop_strings=True)
        for no, ln in enumerate(text.split("\n"), 1):
            if no in exempt:
                continue
            for f in line_findings(ln):
                hits.append("%s:%d %s" % (rel, no, f))
        for no, f in fold_findings(text):
            if no not in exempt:
                hits.append("%s:%d %s" % (rel, no, f))
    hits = sorted(set(hits), key=lambda h: (h.split(":")[0], int(h.split(":")[1].split(" ")[0]), h))
    report(hits, "money arithmetic violation(s)")
    print("no cents arithmetic in the ui/ and feature/ sources")


# ── ui-strings ──────────────────────────────────────────────────
def string_names(path):
    tree = ET.parse(path).getroot()
    return {el.get("name") for el in tree if el.get("name")}


def strings():
    res = os.path.join(root, RES, "values")
    files = sorted(f for f in os.listdir(res) if re.fullmatch(r"strings_ui_.+\.xml", f)) if os.path.isdir(res) else []
    if not files:
        print("SKIP")
        sys.exit(3)
    problems = []
    for fn in files:
        area = fn[len("strings_ui_") : -len(".xml")]
        try:
            base = string_names(os.path.join(res, fn))
        except (ET.ParseError, OSError) as e:
            problems.append("values/%s: cannot parse (%s)" % (fn, e))
            continue
        bad = sorted(n for n in base if not n.startswith("ui_%s_" % area))
        if bad:
            problems.append("values/%s: %d name(s) without the prefix ui_%s_: %s" % (fn, len(bad), area, ", ".join(bad[:5])))
        for loc in ("values-en", "values-zh"):
            twin = os.path.join(root, RES, loc, fn)
            if not os.path.isfile(twin):
                problems.append("%s/%s is missing" % (loc, fn))
                continue
            try:
                names = string_names(twin)
            except (ET.ParseError, OSError) as e:
                problems.append("%s/%s: cannot parse (%s)" % (loc, fn, e))
                continue
            lacks, extra = sorted(base - names), sorted(names - base)
            if lacks:
                problems.append("%s/%s lacks %d name(s): %s" % (loc, fn, len(lacks), ", ".join(lacks[:5])))
            if extra:
                problems.append("%s/%s has %d name(s) not in values/: %s" % (loc, fn, len(extra), ", ".join(extra[:5])))
    report(problems, "string problem(s)")
    print("%d strings_ui file(s) with en and zh twins and the ui_<area>_ prefix" % len(files))


{"contracts": contracts, "money": money, "strings": strings}[check]()
PY

ui_source_check() { python3 -I -c "$UI_CHECKS" "$1" "$ROOT" "$TOOLS"; }

gate ui-contracts ui_source_check contracts
gate ui-money ui_source_check money

# ui-strings exits 3 when there is no strings_ui file: SKIP.
n_checks=$((n_checks + 1))
strings_log="$TMP/check.strings.log"
ui_source_check strings >"$strings_log" 2>&1
strings_rc=$?
if [[ $strings_rc -eq 3 ]]; then
    printf 'GATE | ui-strings | SKIP | no strings_ui_*.xml yet\n'
elif [[ $strings_rc -eq 0 ]]; then
    printf 'GATE | ui-strings | PASS | %s\n' "$(last_line "$strings_log")"
else
    n_failed=$((n_failed + 1))
    printf 'GATE | ui-strings | FAIL | %s\n' "$(last_line "$strings_log")"
    tail -n 40 "$strings_log" | sed 's/^/    | /' >&2
fi

# ── full mode: the self-tests ───────────────────────────────────
if [[ "$fast" -eq 0 ]]; then
    if [[ -n "${PILI_P5_NESTED:-}" ]]; then
        skip selftest-p5-gate "nested in a self-test"
        skip selftest-ui-parity "nested in a self-test"
    else
        export PILI_P5_NESTED=1
        gate selftest-p5-gate bash "$TOOLS/test_p5_gate.sh"
        gate selftest-ui-parity python3 "$TOOLS/test_check_ui_parity.py"
        unset PILI_P5_NESTED
    fi
fi

if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks)\n' "$n_checks"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks)\n' "$n_failed" "$n_checks"
exit 1
