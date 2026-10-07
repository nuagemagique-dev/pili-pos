#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P1a gate
#
# One command that decides whether a P1a branch is mergeable. Prints one
# line per check:
#     GATE | <check> | PASS|FAIL|SKIP | <detail>
# runs every check even after a failure, ends with
#     GATE | total | PASS|FAIL | <n> failed
# and exits 1 on any FAIL. The output of a failed check is printed on stderr.
#
# Usage: scripts/android/p1a_gate.sh [--fast] [--base REF]
#   --fast       only the fast checks (fmt, clippy, tests, hygiene, engine)
#   --base REF   branch the hygiene checks compare with (default android/int)
#
# Fast checks
#   fmt, clippy (workspace, -D warnings), tests (workspace)
#   hygiene-*  against REF: no U+2014 in added lines, no build output or key
#              file / .env / local.properties, Conventional Commits subjects,
#              untouchable files unchanged, no source/checksum change in Cargo.lock
#   engine-*   ENGINE_SRC = crates/pos-engine/src when it exists, else
#              src-tauri/src/commands + state.rs + engine.rs (when present):
#              no Tauri in the engine (checked once crates/pos-engine or
#              src-tauri/src/ipc exists; before that src-tauri/src/commands
#              still holds the Tauri wrappers and the check would fail), no Tauri
#              package in its dependency tree, no unlisted global (static,
#              thread_local!, lazy_static!) in ENGINE_SRC or crates/pos-fiscal/src
#              (allow list: scripts/android/p1a_globals_allow.txt)
# Full adds (every check below the fast ones)
#   cert clippy + tests of every member with a cert feature, the test
#   inventory diff and its self-test, the Linux freeze check and its
#   self-test, the dossier checkers, and (when the files exist) the IPC
#   wrapper and dossier path tools.
#
# Needs ui/dist/index.html (the Tauri crate embeds it): cd ui && npm ci && npm run build
# Honours CARGO_TARGET_DIR; otherwise builds in <repo>/target.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT" || exit 2

fast=0
base="android/int"
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast) fast=1 ;;
    --base)
        base="${2:?--base needs a ref}"
        shift
        ;;
    -h | --help)
        sed -n '2,/^set -uo/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
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

# run_tests <cargo test args...>: the failure tail first, then one summary line.
run_tests() {
    local out="$TMP/tests.out" rc
    cargo test "$@" >"$out" 2>&1
    rc=$?
    [[ $rc -eq 0 ]] || tail -n 40 "$out"
    awk '/^test result:/ { p += $4; f += $6 } END { printf "%d passed, %d failed\n", p, f }' "$out"
    return $rc
}

# gate <check> <command...>: PASS when the command exits 0.
gate() {
    local name="$1" log
    shift
    n_checks=$((n_checks + 1))
    log="$TMP/check.$n_checks.log"
    if "$@" >"$log" 2>&1; then
        printf 'GATE | %s | PASS | %s\n' "$name" "$(grep -v '^[[:space:]]*$' "$log" | tail -n 1 | clean_detail)"
    else
        n_failed=$((n_failed + 1))
        printf 'GATE | %s | FAIL | %s\n' "$name" "$(grep -v '^[[:space:]]*$' "$log" | tail -n 1 | clean_detail)"
        tail -n 40 "$log" | sed 's/^/    | /' >&2
    fi
}

skip() { # <check> <detail>
    n_checks=$((n_checks + 1))
    printf 'GATE | %s | SKIP | %s\n' "$1" "$2"
}

# ── prerequisites ───────────────────────────────────────────────
need_ui() {
    [[ -f ui/dist/index.html ]] || {
        echo "ui/dist/index.html is missing (the Tauri crate embeds it). Build it: cd ui && npm ci && npm run build"
        return 1
    }
    echo "ui/dist present"
}
gate ui-dist need_ui

# cert members: workspace packages that declare a cert feature. Fails closed:
# a missing jq, a cargo metadata error or an empty list is a FAIL, never a
# silent zero-iteration loop (same refusal as test_inventory.sh).
cert_members() {
    command -v jq >/dev/null || { echo "jq is not installed (needed to list the cert members)"; return 1; }
    cargo metadata --format-version 1 --no-deps >"$TMP/metadata.json" 2>"$TMP/metadata.err" ||
        { tail -n 10 "$TMP/metadata.err"; echo "cargo metadata failed"; return 1; }
    jq -r '.packages[] | select(.features | has("cert")) | .name' "$TMP/metadata.json" | LC_ALL=C sort >"$TMP/cert_pkgs.txt" ||
        { echo "jq failed on the cargo metadata output"; return 1; }
    [[ -s "$TMP/cert_pkgs.txt" ]] || { echo "no workspace member with a cert feature"; return 1; }
    echo "cert members: $(tr '\n' ' ' <"$TMP/cert_pkgs.txt" | sed 's/ $//')"
}

# ── hygiene against the base ref ────────────────────────────────
EMDASH="$(printf '\342\200\224')"

h_base_exists() { git rev-parse -q --verify "$base^{commit}" >/dev/null || { echo "base ref not found: $base"; return 1; }; }

h_emdash() {
    local hits
    hits="$(git diff --text -U0 "$base...HEAD" | grep -E '^\+' | grep -v '^+++' | grep -n "$EMDASH" || true)"
    [[ -z "$hits" ]] || { printf '%s\n' "$hits" | head -n 20; echo "U+2014 in added lines"; return 1; }
    echo "no U+2014 in added lines"
}

h_artifacts() {
    local bad
    bad="$(git diff --name-only --diff-filter=ACMRT "$base...HEAD" | grep -E \
        -e '\.(so|dylib|apk|aab|aar|dex|a|rlib|jks|keystore|p12|pfx|pem|key)$' \
        -e '(^|/)\.env(\.[^/]*)?$' \
        -e '(^|/)local\.properties$' |
        grep -vE '(^|/)\.env\.(example|sample|template)$' || true)"
    [[ -z "$bad" ]] || { printf '%s\n' "$bad"; echo "forbidden file(s) changed"; return 1; }
    echo "no build output, key file, .env or local.properties changed"
}

h_subjects() {
    local bad total
    total="$(git log --format=%s "$base..HEAD" | wc -l | tr -d ' ')"
    bad="$(git log --format=%s "$base..HEAD" | grep -vE '^(feat|fix|docs|style|refactor|perf|test|chore)(\([a-z0-9-]+\))?: ' || true)"
    [[ -z "$bad" ]] || { printf '%s\n' "$bad"; echo "subject(s) not in Conventional Commits form"; return 1; }
    echo "$total subject(s) conform"
}

h_untouchable() {
    local paths=(ui rust-toolchain.toml deploy/build-release.sh scripts/build_and_hash.sh scripts/check_repro.sh docs/android/baseline/test_inventory.txt)
    if git diff --quiet "$base...HEAD" -- "${paths[@]}"; then
        echo "untouchable files unchanged"
    else
        git diff --stat "$base...HEAD" -- "${paths[@]}"
        echo "untouchable file(s) changed"
        return 1
    fi
}

h_lock() {
    local bad
    bad="$(git diff "$base...HEAD" -- Cargo.lock | grep -E '^[+-](source|checksum)' || true)"
    [[ -z "$bad" ]] || { printf '%s\n' "$bad" | head -n 20; echo "Cargo.lock source/checksum lines changed"; return 1; }
    echo "Cargo.lock source/checksum lines unchanged"
}

if git rev-parse -q --verify "$base^{commit}" >/dev/null; then
    gate hygiene-emdash h_emdash
    gate hygiene-artifacts h_artifacts
    gate hygiene-subjects h_subjects
    gate hygiene-untouchable h_untouchable
    gate hygiene-cargo-lock h_lock
else
    gate hygiene-base h_base_exists
fi

# ── engine hygiene ──────────────────────────────────────────────
# engine-tauri-free runs once the Tauri wrappers have left the engine sources:
# crates/pos-engine exists (after the move), or src-tauri/src/ipc exists (the
# wrapper split of P1a step 2, before the move). Before that the sources still
# hold Tauri code (99 matches on android/int) and the check would only fail.
ENGINE_SRC=()
ENGINE_MOVED=0
ENGINE_TAURI_CHECK=0
if [[ -d crates/pos-engine/src ]]; then
    ENGINE_SRC=(crates/pos-engine/src)
    ENGINE_MOVED=1
    ENGINE_TAURI_CHECK=1
else
    [[ -d src-tauri/src/ipc ]] && ENGINE_TAURI_CHECK=1
    [[ -d src-tauri/src/commands ]] && ENGINE_SRC+=(src-tauri/src/commands)
    [[ -f src-tauri/src/state.rs ]] && ENGINE_SRC+=(src-tauri/src/state.rs)
    [[ -f src-tauri/src/engine.rs ]] && ENGINE_SRC+=(src-tauri/src/engine.rs)
fi

e_tauri_free() {
    local hits
    hits="$(grep -rnE 'tauri::|use tauri|#\[tauri|target_os' "${ENGINE_SRC[@]}" || true)"
    [[ -z "$hits" ]] || { printf '%s\n' "$hits" | head -n 20; echo "$(printf '%s\n' "$hits" | wc -l | tr -d ' ') Tauri or target_os reference(s) in ${ENGINE_SRC[*]}"; return 1; }
    echo "no Tauri or target_os reference in ${ENGINE_SRC[*]}"
}

e_tree() {
    local pk
    pk="$(cargo tree -p pos-engine -e normal,build --offline --prefix none 2>&1)" || { printf '%s\n' "$pk" | tail -n 20; echo "cargo tree -p pos-engine failed"; return 1; }
    local bad
    bad="$(printf '%s\n' "$pk" | awk '{ print $1 }' | grep -E '^tauri' | LC_ALL=C sort -u || true)"
    [[ -z "$bad" ]] || { printf '%s\n' "$bad"; echo "Tauri package(s) in the pos-engine dependency tree"; return 1; }
    echo "no tauri* package in the pos-engine dependency tree"
}

e_globals() {
    local allow="$TMP/globals.allow" hits bad="" line text files=("${ENGINE_SRC[@]}")
    [[ -d crates/pos-fiscal/src ]] && files+=(crates/pos-fiscal/src)
    [[ -f scripts/android/p1a_globals_allow.txt ]] || { echo "scripts/android/p1a_globals_allow.txt is missing"; return 1; }
    grep -vE '^[[:space:]]*(#|$)' scripts/android/p1a_globals_allow.txt >"$allow" || true
    hits="$(grep -rnE -e '^[[:space:]]*(pub(\([a-z]+\))?[[:space:]]+)?static[[:space:]]+(mut[[:space:]]+)?[A-Z_][A-Z0-9_]*' -e 'thread_local!' -e 'lazy_static!' "${files[@]}" || true)"
    local seen=0
    while IFS= read -r line; do
        [[ -n "$line" ]] || continue
        seen=$((seen + 1))
        text="$(printf '%s' "$line" | cut -d: -f3-)"
        if [[ -s "$allow" ]] && printf '%s' "$text" | grep -qwFf "$allow"; then continue; fi
        bad+="$line"$'\n'
    done <<<"$hits"
    [[ -z "$bad" ]] || { printf '%s' "$bad"; echo "unlisted global(s) (add to p1a_globals_allow.txt only with a reason)"; return 1; }
    echo "$seen global(s), all listed in p1a_globals_allow.txt"
}

if [[ "$ENGINE_TAURI_CHECK" -eq 1 ]]; then
    gate engine-tauri-free e_tauri_free
else
    skip engine-tauri-free "no src-tauri/src/ipc yet: the wrappers have not left the engine sources"
fi
if [[ "$ENGINE_MOVED" -eq 1 ]]; then
    gate engine-tree e_tree
else
    skip engine-tree "no crates/pos-engine yet"
fi
gate engine-globals e_globals

# ── cargo checks ────────────────────────────────────────────────
gate fmt cargo fmt --all --check
gate clippy cargo clippy --workspace --all-targets -- -D warnings
gate tests run_tests --workspace --no-fail-fast

if [[ "$fast" -eq 0 ]]; then
    gate cert-members cert_members
    for pkg in $(cat "$TMP/cert_pkgs.txt" 2>/dev/null); do
        gate "clippy-cert-$pkg" cargo clippy -p "$pkg" --all-targets --no-default-features --features cert -- -D warnings
        gate "tests-cert-$pkg" run_tests -p "$pkg" --no-default-features --features cert --no-fail-fast
    done
    gate test-inventory bash scripts/android/test_inventory.sh --diff
    gate test-inventory-selftest bash scripts/android/test_test_inventory.sh
    gate linux-frozen-selftest bash scripts/android/test_check_linux_frozen.sh

    gate dossier-check python3 scripts/check_dossier.py
    gate dossier-check-selftest python3 scripts/test_check_dossier.py
    gate uc-matrix python3 scripts/uc_matrix.py
    gate uc-matrix-selftest python3 scripts/test_uc_matrix.py

    # Tools added by other P1a cards: run only when present.
    optional() { # <check> <script> [args...]
        local name="$1" file="$2"
        shift 2
        if [[ -f "$file" ]]; then gate "$name" python3 "$file" "$@"; else skip "$name" "absent: $file"; fi
    }
    optional ipc-wrappers scripts/android/check_ipc_wrappers.py
    optional ipc-wrappers-selftest scripts/android/test_check_ipc_wrappers.py
    optional dossier-paths scripts/android/rewrite_dossier_paths.py --check
    optional dossier-paths-selftest scripts/android/test_rewrite_dossier_paths.py

    if git rev-parse -q --verify 'refs/tags/linux-v1.1.0^{commit}' >/dev/null ||
        git rev-parse -q --verify 'refs/tags/linux-v1.1.0-rc1^{commit}' >/dev/null; then
        gate linux-frozen bash scripts/android/check_linux_frozen.sh
    else
        skip linux-frozen "no freeze tag yet"
    fi
fi

if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks)\n' "$n_checks"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks)\n' "$n_failed" "$n_checks"
exit 1
