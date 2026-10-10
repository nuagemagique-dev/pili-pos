#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P1b gate
#
# One command that decides whether a P1b branch is mergeable. Prints one
# line per check:
#     GATE | <check> | PASS|FAIL|SKIP | <detail>
# runs every check even after a failure, ends with exactly one
#     GATE | total | PASS|FAIL | <n> failed (<m> checks)
# counting every check (the P1a ones included) and exits 1 on any FAIL. The
# output of a failed check is printed on stderr.
#
# Usage: scripts/android/p1b_gate.sh [--fast] [--exit] [--ui] [--base REF]
#   --fast     skip the slow UI and tool checks (ui-e2e, ui-e2e-inventory and the
#              two self-tests) and pass --fast to the P1a gate
#   --exit     phase exit: also build and test the UI, and run the checks that the
#              moved rules are gone from ui/src and that the version is 1.0.0
#   --ui       treat the UI as changed (build and test it whatever the diff says)
#   --base REF branch the checks compare with (default android/int)
#
# Checks, in order
#   ui-build          `npm run build` in ui (vue-tsc, then vite) when ui differs from
#                     REF, ui/dist/index.html is missing, or --ui / --exit is given;
#                     else SKIP. It runs first: the cert build embeds ui/dist.
#   <P1a checks>      scripts/android/p1a_gate.sh --allow-ui [--fast] --base REF: its
#                     GATE lines are printed as they are, except its total line; its
#                     checks and failures count in this total.
#   hygiene-baseline-p1b  docs/android/baseline/fdm_journal_* and .../repro unchanged
#                     against REF, and playwright_list.txt unchanged when REF has it
#   ipc-signatures-additive  every `--signatures` line of src-tauri/src/ipc at REF is
#                     still printed for the working tree (new commands may be added)
#   Full mode only (not --fast):
#   ui-e2e            `npx playwright test --project=e2e` in ui with PW_PORT set to a
#                     free port; runs when the UI changed or --ui / --exit, else SKIP
#   ui-e2e-inventory  scripts/android/playwright_inventory.py --diff (same condition)
#   playwright-inventory-selftest, journal-compare-selftest
#   With --exit only:
#   ui-rules          the rules moved to the engine are gone from ui/src (idle timer,
#                     FDM polling, explicit work in/out, order leaving wired through
#                     the router, posSwVersion on the login screen); the detail names
#                     the first failing rule
#   version           1.0.0 in Cargo.toml [workspace.package], src-tauri/tauri.conf.json
#                     and ui/package.json
#
# Needs jq, node/npm, python3 and cargo. Honours CARGO_TARGET_DIR.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT" || exit 2

fast=0
exit_mode=0
force_ui=0
base="android/int"
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast) fast=1 ;;
    --exit) exit_mode=1 ;;
    --ui) force_ui=1 ;;
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

# ── is the UI part of this change? ──────────────────────────────
ui_changed=0
if [[ "$force_ui" -eq 1 || "$exit_mode" -eq 1 ]]; then
    ui_changed=1
elif [[ ! -f ui/dist/index.html ]]; then
    ui_changed=1
elif git rev-parse -q --verify "$base^{commit}" >/dev/null; then
    [[ -z "$(git diff --name-only "$base...HEAD" -- ui)" ]] || ui_changed=1
else
    ui_changed=1 # no usable base: do not assume the UI is unchanged
fi

# ── (i) ui-build ────────────────────────────────────────────────
build_ui() {
    [[ -d ui/node_modules ]] || { echo "ui/node_modules is missing: cd ui && npm ci"; return 1; }
    (cd ui && npm run build) || return 1
    [[ -f ui/dist/index.html ]] || { echo "ui/dist/index.html is missing after the build"; return 1; }
    echo "ui built (vue-tsc + vite)"
}
if [[ "$ui_changed" -eq 1 ]]; then
    gate ui-build build_ui
else
    skip ui-build "ui unchanged"
fi

# ── (ii) the P1a part ───────────────────────────────────────────
p1a_args=(--allow-ui --base "$base")
[[ "$fast" -eq 0 ]] || p1a_args=(--allow-ui --fast --base "$base")
p1a_out="$TMP/p1a.out"
bash scripts/android/p1a_gate.sh "${p1a_args[@]}" >"$p1a_out"
p1a_rc=$?
p1a_lines="$(grep -E '^GATE \| ' "$p1a_out" | grep -vE '^GATE \| total \| ' || true)"
if grep -qE '^GATE \| total \| ' "$p1a_out"; then
    [[ -z "$p1a_lines" ]] || printf '%s\n' "$p1a_lines"
    p1a_n="$(printf '%s\n' "$p1a_lines" | grep -c '^GATE | ' || true)"
    p1a_f="$(printf '%s\n' "$p1a_lines" | grep -c '^GATE | [^|]* | FAIL | ' || true)"
    n_checks=$((n_checks + p1a_n))
    n_failed=$((n_failed + p1a_f))
    if [[ "$p1a_rc" -ne 0 && "$p1a_f" -eq 0 ]]; then
        # a non-zero exit with no failing check line: never let that pass
        n_checks=$((n_checks + 1))
        n_failed=$((n_failed + 1))
        printf 'GATE | p1a-gate | FAIL | p1a_gate.sh exited %d without a failing check\n' "$p1a_rc"
    fi
else
    [[ -z "$p1a_lines" ]] || printf '%s\n' "$p1a_lines"
    n_checks=$((n_checks + 1))
    n_failed=$((n_failed + 1))
    printf 'GATE | p1a-gate | FAIL | p1a_gate.sh exited %d without a total line\n' "$p1a_rc"
fi

# ── (iii) baseline hygiene ──────────────────────────────────────
h_baseline() {
    git rev-parse -q --verify "$base^{commit}" >/dev/null || { echo "base ref not found: $base"; return 1; }
    local changed
    changed="$(git diff --name-only "$base...HEAD" -- docs/android/baseline | grep -E '^docs/android/baseline/(fdm_journal_[^/]*|repro/.*)$' || true)"
    [[ -z "$changed" ]] || { printf '%s\n' "$changed"; echo "baseline journal or repro file(s) changed"; return 1; }
    if git cat-file -e "$base:docs/android/baseline/playwright_list.txt" 2>/dev/null; then
        git diff --quiet "$base...HEAD" -- docs/android/baseline/playwright_list.txt ||
            { echo "docs/android/baseline/playwright_list.txt changed"; return 1; }
        echo "fdm_journal_*, repro and playwright_list.txt unchanged"
    else
        echo "fdm_journal_* and repro unchanged (playwright_list.txt not in $base yet)"
    fi
}
gate hygiene-baseline-p1b h_baseline

# ── (iv) IPC signatures stay additive ───────────────────────────
ipc_additive() {
    local ref_dir="$TMP/ipc_ref"
    git rev-parse -q --verify "$base^{commit}" >/dev/null || { echo "base ref not found: $base"; return 1; }
    mkdir -p "$ref_dir"
    git archive "$base" src-tauri/src/ipc | tar -x -C "$ref_dir" || { echo "no src-tauri/src/ipc in $base"; return 1; }
    [[ -d "$ref_dir/src-tauri/src/ipc" ]] || { echo "no src-tauri/src/ipc in $base"; return 1; }
    python3 scripts/android/check_ipc_wrappers.py --signatures "$ref_dir/src-tauri/src/ipc" >"$TMP/sig_ref.txt" ||
        { echo "check_ipc_wrappers.py failed on $base"; return 1; }
    python3 scripts/android/check_ipc_wrappers.py --signatures src-tauri/src/ipc >"$TMP/sig_now.txt" ||
        { echo "check_ipc_wrappers.py failed on the working tree"; return 1; }
    [[ -s "$TMP/sig_ref.txt" ]] || { echo "no wrapper signature found in $base"; return 1; }
    local kept added missing
    missing="$(grep -vxFf "$TMP/sig_now.txt" "$TMP/sig_ref.txt" || true)"
    [[ -z "$missing" ]] || { printf '%s\n' "$missing"; echo "$(printf '%s\n' "$missing" | wc -l | tr -d ' ') wrapper signature(s) changed or removed"; return 1; }
    kept="$(wc -l <"$TMP/sig_ref.txt" | tr -d ' ')"
    added="$(( $(wc -l <"$TMP/sig_now.txt" | tr -d ' ') - kept ))"
    echo "$kept kept, $added added"
}
gate ipc-signatures-additive ipc_additive

# ── (v)-(viii) full mode ────────────────────────────────────────
run_e2e() {
    local port out="$TMP/e2e.out" rc passed failed
    port="$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')" ||
        { echo "no free TCP port"; return 1; }
    (cd ui && PW_PORT="$port" npx playwright test --project=e2e) >"$out" 2>&1
    rc=$?
    [[ $rc -eq 0 ]] || tail -n 60 "$out"
    passed="$(grep -E '^[[:space:]]*[0-9]+ passed' "$out" | tail -n 1 | grep -oE '[0-9]+' | head -n 1 || true)"
    failed="$(grep -E '^[[:space:]]*[0-9]+ failed' "$out" | tail -n 1 | grep -oE '[0-9]+' | head -n 1 || true)"
    echo "${passed:-0} passed${failed:+, $failed failed} (PW_PORT $port)"
    return $rc
}

if [[ "$fast" -eq 0 ]]; then
    if [[ "$ui_changed" -eq 1 ]]; then
        gate ui-e2e run_e2e
        gate ui-e2e-inventory python3 scripts/android/playwright_inventory.py --diff
    else
        skip ui-e2e "ui unchanged"
        skip ui-e2e-inventory "ui unchanged"
    fi
    gate playwright-inventory-selftest python3 scripts/android/test_playwright_inventory.py
    gate journal-compare-selftest bash scripts/android/test_compare_fdm_journals.sh
fi

# ── (ix)-(x) exit checks ────────────────────────────────────────
ui_rules() {
    local f w hit
    if [[ -e ui/src/composables/useIdleTimer.ts ]]; then
        echo "rule 1: ui/src/composables/useIdleTimer.ts still exists"
        return 1
    fi
    hit="$(grep -rnF 'useIdleTimer' ui/src || true)"
    [[ -z "$hit" ]] || { printf '%s\n' "$hit" | head -n 5; echo "rule 1: useIdleTimer still referenced in ui/src"; return 1; }
    for w in pollDelay POLL_OK_MS POLL_MAX_MS startPolling stopPolling BLOCKING_CODES pickPrimaryMessage; do
        hit="$(grep -rnF "$w" ui/src || true)"
        [[ -z "$hit" ]] || { printf '%s\n' "$hit" | head -n 5; echo "rule 2: $w still in ui/src"; return 1; }
    done
    for w in 'ipc.workIn(' 'ipc.workOut(' 'ipc.signTableOrder(' 'ipc.checkFdmStatus(' 'ipc.listOpenShifts('; do
        hit="$(grep -rnF "$w" ui/src/stores ui/src/router ui/src/App.vue 2>/dev/null || true)"
        [[ -z "$hit" ]] || { printf '%s\n' "$hit" | head -n 5; echo "rule 3: $w still called in stores, router or App.vue"; return 1; }
    done
    grep -qF 'leaveOrder' ui/src/router/index.ts 2>/dev/null || { echo "rule 4: ui/src/router/index.ts does not contain leaveOrder"; return 1; }
    grep -qF 'userActivity' ui/src/App.vue 2>/dev/null || { echo "rule 5: ui/src/App.vue does not contain userActivity"; return 1; }
    grep -qF 'posSwVersion' ui/src/views/LoginView.vue 2>/dev/null || { echo "rule 6: ui/src/views/LoginView.vue does not contain posSwVersion"; return 1; }
    echo "all moved rules are gone from ui/src"
}

check_version() {
    local want="1.0.0" ws tauri pkg bad=""
    command -v jq >/dev/null || { echo "jq is not installed"; return 1; }
    ws="$(awk '/^\[/ { in_ws = ($0 == "[workspace.package]") } in_ws && /^version[[:space:]]*=/ { gsub(/.*=[[:space:]]*"/, ""); gsub(/".*/, ""); print; exit }' Cargo.toml)"
    tauri="$(jq -r '.version // empty' src-tauri/tauri.conf.json 2>/dev/null)"
    pkg="$(jq -r '.version // empty' ui/package.json 2>/dev/null)"
    [[ "$ws" == "$want" ]] || bad+="Cargo.toml [workspace.package] is '${ws:-none}'; "
    [[ "$tauri" == "$want" ]] || bad+="tauri.conf.json is '${tauri:-none}'; "
    [[ "$pkg" == "$want" ]] || bad+="ui/package.json is '${pkg:-none}'; "
    [[ -z "$bad" ]] || { echo "${bad}wanted $want"; return 1; }
    echo "version $want in Cargo.toml, tauri.conf.json and ui/package.json"
}

if [[ "$exit_mode" -eq 1 ]]; then
    gate ui-rules ui_rules
    gate version check_version
fi

if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks)\n' "$n_checks"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks)\n' "$n_failed" "$n_checks"
exit 1
