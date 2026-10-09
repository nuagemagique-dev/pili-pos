#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - X1 gate (export lane)
#
# One command that decides whether a card branch of phase X1 (the nightly export, lane
# android/lane-export) is mergeable. The user reopened the Linux build inputs for this
# lane (2026-10-08, decisions X1D2 and X1D3), so the freeze checks of the foundation
# gates fail by construction. This gate runs those gates, waives exactly those freeze
# checks and adds scope checks instead. No foundation gate script is edited.
# Same output contract as p3_gate.sh and p4_gate.sh. Prints one line per check:
#     GATE | <check> | PASS|FAIL|SKIP|WAIVED | <detail>
# runs every check even after a failure, ends with exactly one
#     GATE | total | PASS|FAIL | <n> failed (<m> checks, <w> waived)
# and exits 1 on any FAIL (2 on a usage error). The output of a failed check is printed
# on stderr; the stderr of the two sub-gates passes through.
#
# Usage: scripts/android/x1_gate.sh [--fast] [--exit] [--android] [--app] [--app-e2e]
#                                   [--linux-repro] [--base REF] [--allow REGEX]
#   --fast        card gate: p1b_gate.sh --fast (no UI end-to-end, no self-tests), and no
#                 x1-selftest
#   --exit        phase exit: --exit to both sub-gates (p4 also gets --android)
#   --android     passed to p4_gate.sh as --android (the Android Gradle checks of p3)
#   --app         run the :app Gradle checks of p4_gate.sh (without it p4 gets --fast)
#   --app-e2e     passed to p4_gate.sh as --app-e2e (emulator end-to-end run)
#   --linux-repro build the Linux binary twice in Docker (x1-linux-repro); the SHA-1 is
#                 for the user's re-freeze (X1D3), it is not compared with LINUX_FREEZE.md
#   --base REF    branch the checks compare with (default android/lane-export)
#   --allow REGEX every file changed in REF...HEAD, modified or untracked must match
#                 REGEX (bash [[ =~ ]]; the caller anchors it); without it x1-paths is SKIP
#
# Checks, in order
#   root:<name>     every check line of scripts/android/p1b_gate.sh --base REF, plus --fast
#                   when --fast and --exit when --exit (root Rust workspace, UI, dossier);
#                   its total line is dropped, its checks and failures are counted
#   android:<name>  every check line of scripts/android/p4_gate.sh --base REF, plus --fast
#                   unless --app or --exit, --android when --android or --exit, --exit when
#                   --exit, --app-e2e when --app-e2e. --allow, --app, --root and
#                   --linux-repro are never passed to it
#   Waivers         a FAIL of exactly one of the six names in the array X1_WAIVABLE below
#                   is printed as WAIVED, with the detail '<original detail> (X1 lane:
#                   Linux freeze reopened, X1D2)', and is not counted as a failure:
#                       root:linux-frozen         root:hygiene-cargo-lock
#                       android:linux-frozen      android:freeze-tag
#                       android:hygiene-root-lock android:hygiene-paths
#                   Any other FAIL counts, whatever the prefix. A sub-gate without a total
#                   line, or whose total contradicts its lines or exit code, also fails.
#   x1-paths        with --allow: every changed file (REF...HEAD, staged or unstaged,
#                   untracked and not ignored) matches REGEX; else SKIP
#   x1-scope        since the fork point X1_FORK (ba57adf, env PILI_X1_FORK) every file of
#                   that kind in a guarded area (Cargo.toml, Cargo.lock, rust-toolchain.toml,
#                   src-tauri, ui, crates, deploy, scripts/build_and_hash.sh,
#                   scripts/check_repro.sh, android/rust, android/pos-core, android/smoke,
#                   android/docker, docs/SUBMISSION_ARTIFACTS, docs/android/baseline) must
#                   match the export allowlist X1_SCOPE (array in this script, joined into
#                   ^(...)$). Files outside the guarded areas are left to x1-paths.
#                   X1_SCOPE alternatives:
#                       Cargo[.]toml
#                       Cargo[.]lock
#                       crates/pos-export/.*
#                       crates/pos-export-mock/.*
#                       crates/pos-core/src/config[.]rs
#                       crates/pos-fiscal/tests/(sign_request|fdm_client_errors|full_service|spf_basic_set)[.]rs
#                       crates/pos-engine/Cargo[.]toml
#                       crates/pos-engine/src/(lib|engine|state|background|install)[.]rs
#                       crates/pos-engine/src/commands/(mod|export|signing)[.]rs
#                       crates/pos-engine/tests/export_[a-z0-9_]+[.]rs
#                       crates/pos-engine-testkit/Cargo[.]toml
#                       crates/pos-engine-testkit/src/(lib|harness|export)[.]rs
#                       src-tauri/src/ipc/(mod|export)[.]rs
#                       src-tauri/src/main[.]rs
#                       ui/src/api/ipc[.]ts
#                       ui/src/types/index[.]ts
#                       ui/src/views/SettingsView[.]vue
#                       ui/src/components/ExportStatusPanel[.]vue
#                       ui/src/i18n-export[.]ts
#                       ui/e2e/export-status[.]spec[.]ts
#                       ui/e2e/fixtures/mock-ipc[.]ts
#                       android/rust/Cargo[.]lock
#                       android/rust/pos-ffi/Cargo[.]toml
#                       android/rust/pos-ffi/src/(lib|core)[.]rs
#                       android/rust/pos-ffi/src/(api|records)/(mod|export)[.]rs
#                       android/rust/pos-ffi/tests/ffi/(main|export)[.]rs
#                       android/pos-core/api/pos-core[.]api
#                       docs/SUBMISSION_ARTIFACTS/[A-Z_]+_FR[.]md
#                       docs/SUBMISSION_ARTIFACTS/pdf_out/[A-Z_]+_FR[.]pdf
#                       docs/SUBMISSION_ARTIFACTS/MANIFEST[.]txt
#   x1-lock-additive  Cargo.lock and android/rust/Cargo.lock only gain packages: every
#                   [[package]] entry (name, version, source, checksum) of the fork is
#                   still in the working tree with the same source and checksum (a version
#                   bump shows as a missing entry)
#   x1-parity       python3 scripts/android/check_command_parity.py --strict
#   x1-selftest     full mode only (not --fast): bash scripts/android/test_x1_gate.sh;
#                   SKIP while PILI_X1_NESTED=1
#   x1-linux-repro  --linux-repro only. FAIL when the working tree is not clean or Docker
#                   is not running. Clones the repository (git clone --no-local of the git
#                   common dir) to $HOME/.cache/pili-freeze/x1-clone, checks out HEAD's
#                   commit, runs scripts/check_repro.sh there (log: $HOME/.cache/pili-freeze/
#                   x1-linux-repro.log); PASS when it exits 0, the detail is its SHA-1. The
#                   clone's build outputs are removed afterwards (the log stays).
#
# Needs git, python3 (stdlib only) and what the two sub-gates need.
# Environment, for the self-test (test_x1_gate.sh) only: PILI_X1_P1B and PILI_X1_P4
# (scripts run with bash instead of the two sub-gates, with the same arguments),
# PILI_X1_ROOT (repository root to work in), PILI_X1_FORK (fork point), PILI_X1_PARITY
# (shell command run instead of check_command_parity.py --strict), PILI_X1_NESTED=1 (the
# self-test check is SKIP).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

TOOLS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SELF="$TOOLS/$(basename "${BASH_SOURCE[0]}")"

# Checks of the foundation gates that any change of a Linux build input fails by
# construction (X1D2). Exact prefixed names; keep in step with the header.
X1_WAIVABLE=(
    root:linux-frozen
    root:hygiene-cargo-lock
    android:linux-frozen
    android:freeze-tag
    android:hygiene-root-lock
    android:hygiene-paths
)
WAIVER_NOTE="(X1 lane: Linux freeze reopened, X1D2)"

# Fork point of the lane (android/lane-export).
X1_FORK="${PILI_X1_FORK:-ba57adf06191a1f386863f98f24930b6f5aa543c}"

# Areas that are frozen outside the export lane.
X1_GUARDED='^(Cargo[.]toml|Cargo[.]lock|rust-toolchain[.]toml|src-tauri/.*|ui/.*|crates/.*|deploy/.*|scripts/(build_and_hash|check_repro)[.]sh|android/(rust|pos-core|smoke|docker)/.*|docs/SUBMISSION_ARTIFACTS/.*|docs/android/baseline/.*)$'

# Export allowlist: the only files of the guarded areas that the lane may change.
# One alternative per line; joined into ^(...)$.
X1_SCOPE=(
    'Cargo[.]toml'
    'Cargo[.]lock'
    'crates/pos-export/.*'
    'crates/pos-export-mock/.*'
    'crates/pos-core/src/config[.]rs'
    'crates/pos-fiscal/tests/(sign_request|fdm_client_errors|full_service|spf_basic_set)[.]rs'
    'crates/pos-engine/Cargo[.]toml'
    'crates/pos-engine/src/(lib|engine|state|background|install)[.]rs'
    'crates/pos-engine/src/commands/(mod|export|signing)[.]rs'
    'crates/pos-engine/tests/export_[a-z0-9_]+[.]rs'
    'crates/pos-engine-testkit/Cargo[.]toml'
    'crates/pos-engine-testkit/src/(lib|harness|export)[.]rs'
    'src-tauri/src/ipc/(mod|export)[.]rs'
    'src-tauri/src/main[.]rs'
    'ui/src/api/ipc[.]ts'
    'ui/src/types/index[.]ts'
    'ui/src/views/SettingsView[.]vue'
    'ui/src/components/ExportStatusPanel[.]vue'
    'ui/src/i18n-export[.]ts'
    'ui/e2e/export-status[.]spec[.]ts'
    'ui/e2e/fixtures/mock-ipc[.]ts'
    'android/rust/Cargo[.]lock'
    'android/rust/pos-ffi/Cargo[.]toml'
    'android/rust/pos-ffi/src/(lib|core)[.]rs'
    'android/rust/pos-ffi/src/(api|records)/(mod|export)[.]rs'
    'android/rust/pos-ffi/tests/ffi/(main|export)[.]rs'
    'android/pos-core/api/pos-core[.]api'
    'docs/SUBMISSION_ARTIFACTS/[A-Z_]+_FR[.]md'
    'docs/SUBMISSION_ARTIFACTS/pdf_out/[A-Z_]+_FR[.]pdf'
    'docs/SUBMISSION_ARTIFACTS/MANIFEST[.]txt'
)
X1_SCOPE_RE="^($(
    IFS='|'
    printf '%s' "${X1_SCOPE[*]}"
))\$"

ROOT="${PILI_X1_ROOT:-$(cd "$TOOLS/../.." && pwd -P)}"
cd "$ROOT" || exit 2

fast=0
exit_mode=0
do_android=0
do_app=0
do_app_e2e=0
do_linux_repro=0
base="android/lane-export"
allow_re=""
allow_set=0
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast) fast=1 ;;
    --exit) exit_mode=1 ;;
    --android) do_android=1 ;;
    --app) do_app=1 ;;
    --app-e2e) do_app_e2e=1 ;;
    --linux-repro) do_linux_repro=1 ;;
    --base)
        [[ $# -ge 2 ]] || { echo "--base needs a ref" >&2; exit 2; }
        base="$2"
        shift
        ;;
    --allow)
        [[ $# -ge 2 ]] || { echo "--allow needs a regex" >&2; exit 2; }
        allow_re="$2"
        allow_set=1
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
n_waived=0

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

is_waivable() { # <prefixed name>
    local w
    for w in "${X1_WAIVABLE[@]}"; do
        [[ "$w" == "$1" ]] && return 0
    done
    return 1
}

# run_subgate <prefix> <script> [args...]: runs the script with bash, re-prints its check
# lines as 'GATE | <prefix>:<name> | ...', drops its total line, counts its checks, and
# turns a FAIL of a waivable name into WAIVED. Its stderr passes through.
run_subgate() {
    local prefix="$1" script="$2" out rc line rest name status detail
    local total_status="" lines_failed=0
    shift 2
    out="$TMP/sub.$prefix.out"
    bash "$script" "$@" >"$out"
    rc=$?
    while IFS= read -r line || [[ -n "$line" ]]; do
        case "$line" in
        "GATE | total | "*)
            rest="${line#"GATE | total | "}"
            total_status="${rest%% | *}"
            ;;
        "GATE | "*)
            rest="${line#"GATE | "}"
            name="${rest%% | *}"
            rest="${rest#"$name"}"
            rest="${rest# | }"
            status="${rest%% | *}"
            detail=""
            if [[ "$rest" == *" | "* ]]; then
                detail="${rest#"$status | "}"
            fi
            n_checks=$((n_checks + 1))
            if [[ "$status" == "FAIL" ]]; then
                lines_failed=$((lines_failed + 1))
                if is_waivable "$prefix:$name"; then
                    n_waived=$((n_waived + 1))
                    printf 'GATE | %s:%s | WAIVED | %s %s\n' "$prefix" "$name" "$detail" "$WAIVER_NOTE"
                else
                    n_failed=$((n_failed + 1))
                    printf 'GATE | %s:%s | FAIL | %s\n' "$prefix" "$name" "$detail"
                fi
            else
                printf 'GATE | %s:%s | %s | %s\n' "$prefix" "$name" "$status" "$detail"
            fi
            ;;
        esac
    done <"$out"
    if [[ -z "$total_status" ]]; then
        n_checks=$((n_checks + 1))
        n_failed=$((n_failed + 1))
        printf 'GATE | %s:total | FAIL | the sub-gate printed no total line (exit %d)\n' "$prefix" "$rc"
    elif [[ "$total_status" == "FAIL" && "$lines_failed" -eq 0 ]]; then
        n_checks=$((n_checks + 1))
        n_failed=$((n_failed + 1))
        printf 'GATE | %s:total | FAIL | the sub-gate total is FAIL but no check line failed (exit %d)\n' "$prefix" "$rc"
    elif [[ "$total_status" == "PASS" && "$rc" -ne 0 ]]; then
        n_checks=$((n_checks + 1))
        n_failed=$((n_failed + 1))
        printf 'GATE | %s:total | FAIL | the sub-gate total is PASS but it exited %d\n' "$prefix" "$rc"
    fi
}

# ── a, b: the two sub-gates ─────────────────────────────────────
p1b_args=(--base "$base")
[[ "$fast" -eq 0 ]] || p1b_args+=(--fast)
[[ "$exit_mode" -eq 0 ]] || p1b_args+=(--exit)
run_subgate root "${PILI_X1_P1B:-$TOOLS/p1b_gate.sh}" "${p1b_args[@]}"

p4_args=(--base "$base")
if [[ "$do_app" -eq 0 && "$exit_mode" -eq 0 ]]; then p4_args+=(--fast); fi
if [[ "$do_android" -eq 1 || "$exit_mode" -eq 1 ]]; then p4_args+=(--android); fi
[[ "$exit_mode" -eq 0 ]] || p4_args+=(--exit)
[[ "$do_app_e2e" -eq 0 ]] || p4_args+=(--app-e2e)
run_subgate android "${PILI_X1_P4:-$TOOLS/p4_gate.sh}" "${p4_args[@]}"

# ── changed files ───────────────────────────────────────────────
have_ref() { git rev-parse -q --verify "$1^{commit}" >/dev/null 2>&1; }

# changed_since <ref>: REF...HEAD, modified (staged or not) and untracked-not-ignored.
changed_since() {
    git -c core.quotepath=off diff --name-only --no-renames "$1...HEAD"
    git -c core.quotepath=off diff --name-only --no-renames HEAD
    git -c core.quotepath=off ls-files --others --exclude-standard
}

# ── d: x1-paths ─────────────────────────────────────────────────
x1_paths() {
    have_ref "$base" || { echo "base ref not found: $base"; return 1; }
    local files f n=0 bad=0
    files="$(changed_since "$base" | LC_ALL=C sort -u)"
    while IFS= read -r f; do
        [[ -n "$f" ]] || continue
        n=$((n + 1))
        if ! [[ "$f" =~ $allow_re ]]; then
            bad=$((bad + 1))
            [[ "$bad" -gt 30 ]] || echo "not allowed: $f"
        fi
    done <<<"$files"
    if [[ "$bad" -gt 0 ]]; then
        echo "$bad of $n changed file(s) do not match the allow regex"
        return 1
    fi
    echo "$n changed file(s), all match the allow regex"
}
if [[ "$allow_set" -eq 1 ]]; then
    gate x1-paths x1_paths
else
    skip x1-paths "no --allow given"
fi

# ── e: x1-scope ─────────────────────────────────────────────────
x1_scope() {
    have_ref "$X1_FORK" || { echo "fork point not found: $X1_FORK"; return 1; }
    local files f guarded=0 bad=0
    files="$(changed_since "$X1_FORK" | LC_ALL=C sort -u)"
    while IFS= read -r f; do
        [[ -n "$f" ]] || continue
        if [[ "$f" =~ $X1_GUARDED ]]; then
            guarded=$((guarded + 1))
            if ! [[ "$f" =~ $X1_SCOPE_RE ]]; then
                bad=$((bad + 1))
                [[ "$bad" -gt 30 ]] || echo "out of the export scope: $f"
            fi
        fi
    done <<<"$files"
    if [[ "$bad" -gt 0 ]]; then
        echo "$bad of $guarded guarded file(s) changed since ${X1_FORK:0:7} are outside the export scope"
        return 1
    fi
    echo "$guarded guarded file(s) changed since ${X1_FORK:0:7}, all in the export scope"
}
gate x1-scope x1_scope

# ── f: x1-lock-additive ─────────────────────────────────────────
x1_lock_additive() {
    have_ref "$X1_FORK" || { echo "fork point not found: $X1_FORK"; return 1; }
    python3 -I - "$X1_FORK" Cargo.lock android/rust/Cargo.lock <<'PY'
import re
import subprocess
import sys

fork = sys.argv[1]
files = sys.argv[2:]
FIELD = re.compile(r'^(name|version|source|checksum) = "(.*)"$')


def parse(text):
    entries = []
    cur = None
    for line in text.splitlines():
        s = line.strip()
        if s == "[[package]]":
            cur = {}
            entries.append(cur)
        elif s.startswith("["):
            cur = None
        elif cur is not None:
            m = FIELD.match(s)
            if m:
                cur[m.group(1)] = m.group(2)
    return [
        (e.get("name", ""), e.get("version", ""), e.get("source", ""), e.get("checksum", ""))
        for e in entries
    ]


problems = []
parts = []
for f in files:
    shown = subprocess.run(
        ["git", "show", "%s:%s" % (fork, f)], capture_output=True, text=True
    )
    old = parse(shown.stdout) if shown.returncode == 0 else []
    try:
        with open(f, encoding="utf-8") as fh:
            new = parse(fh.read())
    except OSError:
        new = []
    by_nv = {}
    for e in new:
        by_nv.setdefault((e[0], e[1]), []).append(e)
    kept = 0
    for name, version, source, checksum in old:
        cands = by_nv.get((name, version))
        if not cands:
            problems.append("%s: missing %s %s" % (f, name, version))
        elif not any(c[2] == source and c[3] == checksum for c in cands):
            problems.append("%s: source or checksum changed for %s %s" % (f, name, version))
        else:
            kept += 1
    old_keys = set((e[0], e[1], e[2]) for e in old)
    added = sum(1 for e in new if (e[0], e[1], e[2]) not in old_keys)
    parts.append("%s: %d kept, %d added" % (f, kept, added))

if problems:
    for p in problems[:30]:
        print(p)
    print("%d lock entr%s of the fork lost or changed (%s)" % (len(problems), "y" if len(problems) == 1 else "ies", "; ".join(parts)))
    sys.exit(1)
print("; ".join(parts))
PY
}
gate x1-lock-additive x1_lock_additive

# ── g: x1-parity ────────────────────────────────────────────────
if [[ -n "${PILI_X1_PARITY:-}" ]]; then
    gate x1-parity bash -c "$PILI_X1_PARITY"
else
    gate x1-parity python3 scripts/android/check_command_parity.py --strict
fi

# ── h: x1-selftest (full mode) ──────────────────────────────────
if [[ "$fast" -eq 0 ]]; then
    if [[ -n "${PILI_X1_NESTED:-}" ]]; then
        skip x1-selftest "nested in a self-test"
    else
        export PILI_X1_NESTED=1
        gate x1-selftest bash "$TOOLS/test_x1_gate.sh"
        unset PILI_X1_NESTED
    fi
fi

# ── i: x1-linux-repro ───────────────────────────────────────────
x1_linux_repro() {
    local cache="$HOME/.cache/pili-freeze" clone log common head got rc
    clone="$cache/x1-clone"
    log="$cache/x1-linux-repro.log"
    if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
        echo "the working tree is not clean: commit first"
        return 1
    fi
    command -v docker >/dev/null 2>&1 || { echo "docker is not installed"; return 1; }
    docker info >/dev/null 2>&1 || { echo "docker is not running"; return 1; }
    mkdir -p "$cache" || return 1
    head="$(git rev-parse HEAD)" || return 1
    common="$(git rev-parse --path-format=absolute --git-common-dir)" || return 1
    rm -rf "$clone"
    git clone -q --no-local "$common" "$clone" || { echo "git clone failed"; return 1; }
    if ! git -C "$clone" checkout -q --detach "$head"; then
        echo "cannot check out $head in the clone"
        return 1
    fi
    (cd "$clone" && bash scripts/check_repro.sh) >"$log" 2>&1
    rc=$?
    rm -rf "$clone/deploy/dist" "$clone/target" "$clone/ui/dist" "$clone/ui/node_modules"
    rm -f "$clone"/build_info_*.txt
    got="$(sed -n 's/.*Reproducible: SHA-1 \([0-9a-f]\{40\}\).*/\1/p' "$log" | tail -n 1)"
    if [[ $rc -ne 0 ]]; then
        tail -n 20 "$log"
        echo "check_repro.sh exited $rc (log: $log)"
        return 1
    fi
    if [[ -z "$got" ]]; then
        echo "check_repro.sh reported no SHA-1 (log: $log)"
        return 1
    fi
    echo "reproducible, SHA-1 $got"
}
if [[ "$do_linux_repro" -eq 1 ]]; then
    gate x1-linux-repro x1_linux_repro
fi

# ── j: total ────────────────────────────────────────────────────
if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks, %d waived)\n' "$n_checks" "$n_waived"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks, %d waived)\n' "$n_failed" "$n_checks" "$n_waived"
exit 1
