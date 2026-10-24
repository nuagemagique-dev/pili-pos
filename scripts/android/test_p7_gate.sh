#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/p7_gate.sh
#
# Usage: bash scripts/android/test_p7_gate.sh
#
# Runs p7_gate.sh against temporary git repositories (PILI_P7_ROOT). The tools the gate
# calls are stubs from scripts/android/testdata/p7_gate/ copied into the temporary
# repository (lane_paths.py, check_android_frozen.sh, the parity and inventory scripts, a
# gradlew, the self-test and emulator scripts), and a stub cargo found through
# PILI_P7_STUB_DIR. Their behaviour is chosen by STUB_* variables and their calls are
# recorded in $STUB_LOG. No Gradle, no cargo, no network, no emulator.
# Cases: usage errors, then each check in PASS and FAIL (hygiene-subjects, hygiene-dash,
# hygiene-secrets, scope with --allow and the shared owner, lanes, android-frozen with and
# without the tag, frozen-paths, the parity checks, the uccheck family, uc-replay,
# uc-matrix, gradle-plan, the self-test checks, root-inventory, the three emulator and
# report checks), the SKIP and --exit behaviour, --fast and the total line.
# Prints one line per case, then
#     TESTP7GATE | total | PASS|FAIL | <n> tests
# and exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
GATE="$ROOT/scripts/android/p7_gate.sh"
FIX="$ROOT/scripts/android/testdata/p7_gate"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
export HOME="$TMP/home"
mkdir -p "$HOME"
unset PILI_P7_NESTED PILI_P7_ROOT PILI_P7_STUB_DIR PILI_P7_SOAK_MIN_HOURS
for v in STUB_LANES_FAIL STUB_REGEX_EMPTY STUB_FROZEN_FAIL STUB_KTESTS_FAIL STUB_CMDPARITY_FAIL \
    STUB_UIPARITY_FAIL STUB_CARGO_FAIL STUB_UCCHECK STUB_GRADLE_FAIL STUB_TOTAL STUB_WS STUB_INV_FAIL; do
    unset "$v"
done
export STUB_LOG="$TMP/stub.log"
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TESTP7GATE | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TESTP7GATE | %s | FAIL | %s\n' "$1" "$2"; }
# check NAME CONDITION-RESULT(0 = true) DETAIL
check() { if [[ "$2" -eq 0 ]]; then ok "$1"; else bad "$1" "$3"; fi; }

# tgit REPO git-args...: git inside a temporary repository only.
tgit() {
    local r="$1"
    shift
    if [[ -z "$r" || "$r" != "$TMP"/* || ! -d "$r/.git" ]]; then
        echo "tgit: refusing to run git in '$r'" >&2
        exit 1
    fi
    git -C "$r" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"
}

# write FILE: stdin to FILE (directories are created)
write() {
    mkdir -p "$(dirname "$1")"
    cat >"$1"
}

EMDASH=$'\xe2\x80\x94'
ENDASH=$'\xe2\x80\x93'

STUBS="$TMP/stubs"
mkdir -p "$STUBS"
cp "$FIX/cargo" "$STUBS/cargo"
chmod +x "$STUBS/cargo"

# A pristine repository: branch 'base' with the stub tools and a few frozen files, checked out as 'work'.
PRISTINE="$TMP/pristine"
build_pristine() {
    local r="$PRISTINE" f
    mkdir -p "$r"
    git -C "$r" init -q -b base
    mkdir -p "$r/scripts/android" "$r/android" "$r/docs/android/uc" "$r/crates/pos-engine/src" "$r/ui" \
        "$r/src-tauri" "$r/docs/android/baseline"
    for f in lane_paths.py check_android_frozen.sh kotlin_test_inventory.py check_command_parity.py check_ui_parity.py test_inventory.sh; do
        cp "$FIX/$f" "$r/scripts/android/$f"
    done
    cp "$FIX/selftest_stub.sh" "$r/scripts/android/test_p7_gate.sh"
    cp "$FIX/selftest_stub.sh" "$r/scripts/android/test_p7_emulator_lock.sh"
    cp "$FIX/selftest_stub.sh" "$r/scripts/android/test_uc_live_module2.sh"
    cp "$FIX/gradlew" "$r/android/gradlew"
    chmod +x "$r"/scripts/android/* "$r/android/gradlew"
    printf '[workspace]\nmembers = []\n' >"$r/Cargo.toml"
    printf '# lock\n' >"$r/Cargo.lock"
    printf 'pub fn engine() {}\n' >"$r/crates/pos-engine/src/lib.rs"
    printf '<html></html>\n' >"$r/ui/index.html"
    printf 'fn main() {}\n' >"$r/src-tauri/main.rs"
    printf 'baseline\n' >"$r/docs/android/baseline/test_inventory.txt"
    printf '# decisions\n' >"$r/docs/android/DECISIONS.md"
    printf 'notes\n' >"$r/docs/android/uc/notes.md"
    printf 'other\n' >"$r/docs/other.md"
    git -C "$r" add -A
    git -C "$r" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -q -m "chore: base"
    git -C "$r" checkout -q -b work
}
build_pristine

# new_repo NAME: a fresh copy in REPO; clears every STUB_* switch and the options.
new_repo() {
    REPO="$TMP/repo.$1"
    rm -rf "$REPO"
    cp -R "$PRISTINE" "$REPO"
    : >"$STUB_LOG"
    for v in STUB_LANES_FAIL STUB_REGEX_EMPTY STUB_FROZEN_FAIL STUB_KTESTS_FAIL STUB_CMDPARITY_FAIL \
        STUB_UIPARITY_FAIL STUB_CARGO_FAIL STUB_UCCHECK STUB_GRADLE_FAIL STUB_TOTAL STUB_WS STUB_INV_FAIL; do
        unset "$v"
    done
    unset PILI_P7_SOAK_MIN_HOURS
}

# commit_all SUBJECT
commit_all() {
    tgit "$REPO" add -A
    tgit "$REPO" commit -q -m "$1"
}

# run_gate ARGS...: the gate on REPO; sets OUT, ERR, RC. PILI_P7_NESTED=1 unless NESTED=0.
run_gate() {
    OUT="$TMP/out.txt"
    ERR="$TMP/err.txt"
    local nested="${NESTED:-1}"
    if [[ "$nested" == "1" ]]; then
        PILI_P7_NESTED=1 PILI_P7_ROOT="$REPO" PILI_P7_STUB_DIR="$STUBS" bash "$GATE" "$@" >"$OUT" 2>"$ERR"
    else
        PILI_P7_ROOT="$REPO" PILI_P7_STUB_DIR="$STUBS" bash "$GATE" "$@" >"$OUT" 2>"$ERR"
    fi
    RC=$?
}
BASE=(--base base)

# status CHECK: PASS, FAIL, SKIP, or 'none' when the gate printed no such check
status() {
    local l
    l="$(grep "^GATE | $1 | " "$OUT" | head -n 1)"
    if [[ -z "$l" ]]; then
        echo none
        return
    fi
    echo "$l" | awk -F' [|] ' '{print $3}'
}
detail() { grep "^GATE | $1 | " "$OUT" | head -n 1 | awk -F' [|] ' '{print $4}'; }
total_line() { grep '^GATE | total | ' "$OUT"; }
# expect NAME CHECK STATUS: the check has that status
expect() { # <case> <check> <status>
    local got
    got="$(status "$2")"
    check "$1" $([[ "$got" == "$3" ]] && echo 0 || echo 1) "check $2 is $got (wanted $3); rc=$RC; out: $(grep "^GATE | $2 " "$OUT" | head -n 1); err: $(head -c 300 "$ERR")"
}

# ── 1. a clean tree passes, the total line counts the checks ──
new_repo clean
run_gate --fast "${BASE[@]}"
n="$(grep -c '^GATE | ' "$OUT")"
check clean-fast-passes-exit-0 $([[ $RC -eq 0 && "$(total_line)" == "GATE | total | PASS | 0 failed ($((n - 1)) checks)" ]] && echo 0 || echo 1) "rc=$RC total=$(total_line) n=$n"
order="$(grep '^GATE | ' "$OUT" | awk -F' [|] ' '{printf "%s ", $2}')"
check check-order $([[ "$order" == "hygiene-subjects hygiene-dash hygiene-secrets scope lanes android-frozen frozen-paths kotlin-tests command-parity ui-parity uccheck-fmt uccheck-clippy uccheck-test uccheck-lock uc-scenarios uc-replay uc-matrix total " ]] && echo 0 || echo 1) "order: $order"

# ── 2. usage errors exit 2 ──
new_repo usage
run_gate --bogus
rc1=$RC
run_gate --base
rc2=$RC
run_gate --allow
rc3=$RC
run_gate --allow '('
rc4=$RC
run_gate --base ''
rc5=$RC
check usage-errors-exit-2 $([[ $rc1 -eq 2 && $rc2 -eq 2 && $rc3 -eq 2 && $rc4 -eq 2 && $rc5 -eq 2 ]] && echo 0 || echo 1) "rc: $rc1 $rc2 $rc3 $rc4 $rc5"
run_gate --help
check help-prints-header $([[ $RC -eq 0 ]] && grep -q 'P7 gate' "$OUT" && ! grep -q '^set -uo' "$OUT" && echo 0 || echo 1) "rc=$RC out: $(head -n 3 "$OUT")"

# ── 3. hygiene-subjects ──
new_repo subj
printf 'x\n' >"$REPO/docs/android/uc/a.md"
commit_all "update the uc notes"
run_gate --fast "${BASE[@]}"
expect subject-not-conventional-fails hygiene-subjects FAIL
check failed-gate-exits-1-and-runs-all $([[ $RC -eq 1 && "$(total_line)" == "GATE | total | FAIL | 1 failed (17 checks)" ]] && echo 0 || echo 1) "rc=$RC total=$(total_line)"
check failed-output-on-stderr $(grep -q 'update the uc notes' "$ERR" && echo 0 || echo 1) "stderr: $(head -c 200 "$ERR")"
new_repo subj2
printf 'x\n' >"$REPO/docs/android/uc/a.md"
commit_all "feat(android)!: add a thing"
printf 'y\n' >"$REPO/docs/android/uc/b.md"
commit_all "docs: plain scope"
run_gate --fast "${BASE[@]}"
expect subjects-conventional-pass hygiene-subjects PASS
run_gate --fast --base no-such-ref
expect missing-base-ref-fails hygiene-subjects FAIL

# ── 4. hygiene-dash ──
new_repo dash1
printf 'a %s b\n' "$EMDASH" >"$REPO/docs/android/uc/a.md"
commit_all "docs: em"
run_gate --fast "${BASE[@]}"
expect added-em-dash-fails hygiene-dash FAIL
new_repo dash2
printf 'a %s b\n' "$ENDASH" >"$REPO/docs/android/uc/a.md"
commit_all "docs: en"
run_gate --fast "${BASE[@]}"
expect added-en-dash-fails hygiene-dash FAIL
new_repo dash3
printf 'untracked %s\n' "$EMDASH" >"$REPO/docs/android/uc/new.md"
run_gate --fast "${BASE[@]}"
expect untracked-dash-fails hygiene-dash FAIL
new_repo dash4
printf 'unstaged %s\n' "$ENDASH" >>"$REPO/docs/android/uc/notes.md"
run_gate --fast "${BASE[@]}"
expect working-tree-dash-fails hygiene-dash FAIL
new_repo dash5
printf 'old %s line\n' "$EMDASH" >"$REPO/docs/android/uc/old.md"
commit_all "docs: old line with a dash"
tgit "$REPO" branch -f base HEAD
printf 'clean line\n' >"$REPO/docs/android/uc/old.md"
commit_all "docs: remove the dash"
run_gate --fast "${BASE[@]}"
expect removed-dash-passes hygiene-dash PASS

# ── 5. hygiene-secrets ──
new_repo sec1
write "$REPO/docs/android/uc/release.jks" <<<"binary"
commit_all "chore: add keystore"
run_gate --fast "${BASE[@]}"
expect added-jks-fails hygiene-secrets FAIL
new_repo sec2
printf 'x\n' >"$REPO/docs/android/uc/.env.live"
run_gate --fast "${BASE[@]}"
expect untracked-env-file-fails hygiene-secrets FAIL
new_repo sec3
printf 'export %s=abc\n' "PILI_LIVE_FDM_TOKEN" >"$REPO/docs/android/uc/live.sh"
commit_all "chore: live"
run_gate --fast "${BASE[@]}"
expect added-live-token-fails hygiene-secrets FAIL
new_repo sec4
printf 'token="${%s:-}"\nexport %s=\n' "PILI_LIVE_FDM_TOKEN" "PILI_LIVE_FDM_TOKEN" >"$REPO/docs/android/uc/live.sh"
commit_all "chore: live reader"
run_gate --fast "${BASE[@]}"
expect live-token-name-without-value-passes hygiene-secrets PASS
new_repo sec5
printf -- '-----BEGIN %s-----\nabc\n' "EC PRIVATE KEY" >"$REPO/docs/android/uc/k.txt"
commit_all "chore: key"
run_gate --fast "${BASE[@]}"
expect added-private-key-block-fails hygiene-secrets FAIL
new_repo sec6
write "$REPO/docs/android/uc/dev-minisign.key" <<<"x"
commit_all "chore: minisign"
run_gate --fast "${BASE[@]}"
expect added-minisign-key-fails hygiene-secrets FAIL

# ── 6. scope ──
new_repo scope1
printf 'x\n' >"$REPO/docs/other.md"
commit_all "docs: out of scope"
run_gate --fast "${BASE[@]}"
expect out-of-scope-path-fails scope FAIL
check scope-names-the-path $(grep -q "docs/other.md$" "$ERR" && echo 0 || echo 1) "stderr: $(head -n 12 "$ERR")"
run_gate --fast "${BASE[@]}" --allow '^docs/other[.]md$'
expect allow-admits-the-path scope PASS
run_gate --fast "${BASE[@]}" --allow '^nothing$' --allow '^docs/other[.]md$'
expect allow-is-repeatable scope PASS
new_repo scope2
printf 'x\n' >>"$REPO/docs/android/DECISIONS.md"
printf 'x\n' >"$REPO/docs/android/uc/a.md"
printf '#!/bin/sh\n' >"$REPO/scripts/android/p7_soak.sh"
commit_all "docs: shared and lane paths"
run_gate --fast "${BASE[@]}"
expect shared-and-lane-paths-admitted scope PASS
new_repo scope3
printf 'x\n' >"$REPO/docs/untracked.md"
run_gate --fast "${BASE[@]}"
expect untracked-out-of-scope-fails scope FAIL
new_repo scope4
tgit "$REPO" mv docs/other.md docs/android/uc/moved.md
run_gate --fast "${BASE[@]}"
expect rename-from-out-of-scope-fails scope FAIL
new_repo scope5
export STUB_REGEX_EMPTY=1
run_gate --fast "${BASE[@]}"
expect empty-lane-regex-fails scope FAIL

# ── 7. lanes ──
new_repo lanes
export STUB_LANES_FAIL=1
run_gate --fast "${BASE[@]}"
expect lanes-failure-fails lanes FAIL

# ── 8. android-frozen ──
new_repo frozen1
run_gate --fast "${BASE[@]}"
expect frozen-skip-without-tag android-frozen SKIP
check frozen-skip-detail $([[ "$(detail android-frozen)" == "no tag android-v1.0.0-rc3 yet" ]] && echo 0 || echo 1) "detail: $(detail android-frozen)"
check frozen-not-called-without-tag $(! grep -q '^frozen' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
: >"$STUB_LOG"
tgit "$REPO" tag linux-v1.0.0-rc2 base
run_gate --fast "${BASE[@]}"
expect frozen-linux-tag-is-no-freeze android-frozen SKIP
tgit "$REPO" tag android-v1.0.0-rc3 base
run_gate --fast "${BASE[@]}"
check frozen-tag-used $(grep -q '^frozen --tag android-v1.0.0-rc3$' "$STUB_LOG" && ! grep -q -- '--ref' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
expect frozen-pass-with-tag android-frozen PASS
export STUB_FROZEN_FAIL=1
run_gate --fast "${BASE[@]}"
expect frozen-failure-fails android-frozen FAIL

# ── 9. frozen-paths ──
new_repo fp1
printf 'pub fn changed() {}\n' >"$REPO/crates/pos-engine/src/lib.rs"
commit_all "fix: touch the engine"
run_gate --fast "${BASE[@]}"
expect changed-frozen-path-fails frozen-paths FAIL
new_repo fp2
printf 'pub fn extra() {}\n' >"$REPO/crates/pos-engine/src/extra.rs"
run_gate --fast "${BASE[@]}"
expect untracked-file-under-crates-fails frozen-paths FAIL
new_repo fp3
printf '// edited\n' >>"$REPO/ui/index.html"
run_gate --fast "${BASE[@]}"
expect uncommitted-frozen-edit-fails frozen-paths FAIL
new_repo fp4
printf 'x\n' >"$REPO/docs/android/baseline/new.txt"
run_gate --fast "${BASE[@]}"
expect untracked-baseline-file-fails frozen-paths FAIL
new_repo fp5
printf 'x\n' >"$REPO/android/rust-notes.txt"
printf 'x\n' >"$REPO/docs/android/uc/a.md"
run_gate --fast "${BASE[@]}"
expect non-frozen-paths-pass frozen-paths PASS

# ── 10. kotlin-tests and the parity checks ──
new_repo parity
export STUB_KTESTS_FAIL=1
run_gate --fast "${BASE[@]}"
expect kotlin-tests-failure-fails kotlin-tests FAIL
unset STUB_KTESTS_FAIL
export STUB_CMDPARITY_FAIL=1
run_gate --fast "${BASE[@]}"
expect command-parity-failure-fails command-parity FAIL
unset STUB_CMDPARITY_FAIL
export STUB_UIPARITY_FAIL=1
run_gate --fast "${BASE[@]}"
expect ui-parity-failure-fails ui-parity FAIL
unset STUB_UIPARITY_FAIL

# ── 11. tool checks: SKIP when absent, FAIL with --exit ──
new_repo tools
run_gate --fast "${BASE[@]}"
sk=""
for c in uccheck-fmt uccheck-clippy uccheck-test uccheck-lock uc-scenarios uc-replay uc-matrix; do sk+="$(status $c) "; done
check absent-tools-skip $([[ "$sk" == "SKIP SKIP SKIP SKIP SKIP SKIP SKIP " && $RC -eq 0 ]] && echo 0 || echo 1) "statuses: $sk rc=$RC"
run_gate --exit --fast "${BASE[@]}"
sk=""
for c in uccheck-fmt uccheck-clippy uccheck-test uccheck-lock uc-scenarios uc-replay uc-matrix; do sk+="$(status $c) "; done
check exit-mode-absent-tools-fail $([[ "$sk" == "FAIL FAIL FAIL FAIL FAIL FAIL FAIL " && $RC -eq 1 ]] && echo 0 || echo 1) "statuses: $sk rc=$RC"
check exit-mode-total-counts $([[ "$(total_line)" == "GATE | total | FAIL | 7 failed (17 checks)" ]] && echo 0 || echo 1) "total: $(total_line)"

# ── 12. the uccheck family with the tool present ──
new_repo uc1
write "$REPO/android/uccheck/Cargo.toml" <<<"[workspace]"
run_gate --fast "${BASE[@]}"
expect uccheck-fmt-passes uccheck-fmt PASS
expect uccheck-clippy-passes uccheck-clippy PASS
expect uccheck-test-passes uccheck-test PASS
check uccheck-cargo-arguments $(grep -q '^cargo clippy --manifest-path android/uccheck/Cargo.toml --all-targets --locked --offline --target-dir target/uccheck -- -D warnings$' "$STUB_LOG" &&
    grep -q '^cargo test --manifest-path android/uccheck/Cargo.toml --locked --offline --target-dir target/uccheck$' "$STUB_LOG" &&
    grep -q '^cargo fmt --manifest-path android/uccheck/Cargo.toml --all --check$' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
expect uc-replay-skips-without-journals uc-replay SKIP
export STUB_CARGO_FAIL=1
run_gate --fast "${BASE[@]}"
expect cargo-failure-fails-clippy uccheck-clippy FAIL
unset STUB_CARGO_FAIL

# the stub directory comes before ~/.cargo/bin: a real cargo there must not shadow the stub
mkdir -p "$HOME/.cargo/bin"
printf '#!/bin/sh\necho "REAL cargo $*" >>"$STUB_LOG"\nexit 0\n' >"$HOME/.cargo/bin/cargo"
chmod +x "$HOME/.cargo/bin/cargo"
: >"$STUB_LOG"
run_gate --fast "${BASE[@]}"
check stub-cargo-before-home-cargo $(grep -q '^cargo fmt ' "$STUB_LOG" && ! grep -q '^REAL cargo' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
rm -rf "$HOME/.cargo"

# ── 13. uc-replay ──
new_repo replay1
write "$REPO/android/uccheck/Cargo.toml" <<<"[workspace]"
write "$REPO/docs/android/uc/journals/android/10_basic/010_M110.json" <<<"{}"
: >"$STUB_LOG"
run_gate --fast "${BASE[@]}"
expect uc-replay-passes uc-replay PASS
check uc-replay-runs-record-then-compare $(awk '/ record /{r=NR} / compare /{c=NR} END{exit !(r && c && r < c)}' "$STUB_LOG" &&
    grep -q 'record --scenarios android/app/src/androidTest/assets/uc --out target/p7-gate$' "$STUB_LOG" &&
    grep -q 'compare --linux target/p7-gate/linux --android docs/android/uc/journals/android --fixtures$' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
export STUB_UCCHECK=FAIL
run_gate --fast "${BASE[@]}"
expect uc-replay-compare-failure-fails uc-replay FAIL

# ── 14. uc-matrix and the lock/scenario scripts ──
new_repo matrix
cat >"$REPO/scripts/android/uc_matrix_android.py" <<'EOS'
import sys
open(__import__("os").environ["STUB_LOG"], "a").write("matrix " + " ".join(sys.argv[1:]) + "\n")
EOS
cat >"$REPO/scripts/android/uc_scenarios_check.py" <<'EOS'
import os
import sys
open(os.environ["STUB_LOG"], "a").write("scenarios\n")
sys.exit(1 if os.environ.get("STUB_SCEN_FAIL") == "1" else 0)
EOS
printf '#!/usr/bin/env bash\necho lock-parity-ok\n' >"$REPO/scripts/android/uc_lock_parity.sh"
run_gate --fast "${BASE[@]}"
expect uc-matrix-runs uc-matrix PASS
expect uc-scenarios-runs uc-scenarios PASS
expect uccheck-lock-runs uccheck-lock PASS
check matrix-allows-pending-outside-exit $(grep -qx 'matrix --check --allow-pending' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
: >"$STUB_LOG"
run_gate --exit --fast "${BASE[@]}"
check matrix-strict-in-exit-mode $(grep -qx 'matrix --check' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
STUB_SCEN_FAIL=1 run_gate --fast "${BASE[@]}"
expect uc-scenarios-failure-fails uc-scenarios FAIL

# ── 15. --fast, gradle-plan and the self-tests ──
new_repo full
run_gate "${BASE[@]}"
check full-runs-gradle-and-selftests $([[ "$(status gradle-plan)" == PASS && "$(status selftest-p7-gate)" == SKIP && "$(status selftest-lock)" == SKIP && "$(status selftest-live)" == SKIP ]] && echo 0 || echo 1) "gradle=$(status gradle-plan) st=$(status selftest-p7-gate) lock=$(status selftest-lock) live=$(status selftest-live)"
run_gate --fast "${BASE[@]}"
check fast-skips-gradle-and-selftests $([[ "$(status gradle-plan)" == none && "$(status selftest-p7-gate)" == none && "$(status selftest-lock)" == none && "$(status selftest-live)" == none ]] && echo 0 || echo 1) "gradle=$(status gradle-plan) live=$(status selftest-live)"
: >"$STUB_LOG"
NESTED=0 run_gate "${BASE[@]}"
check selftests-run-when-not-nested $([[ "$(status selftest-p7-gate)" == PASS && "$(status selftest-lock)" == PASS && "$(status selftest-live)" == PASS ]] && grep -q '^selftest test_p7_gate.sh$' "$STUB_LOG" && grep -q '^selftest test_p7_emulator_lock.sh$' "$STUB_LOG" && grep -q '^selftest test_uc_live_module2.sh$' "$STUB_LOG" && echo 0 || echo 1) "st=$(status selftest-p7-gate) lock=$(status selftest-lock) live=$(status selftest-live) log: $(cat "$STUB_LOG")"
check gradle-plan-arguments $(grep -q '^gradlew .*--offline lint ktlintCheck detekt test koverVerifyDebug assembleRelease :app:assembleDebugAndroidTest$' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
export STUB_GRADLE_FAIL=1
run_gate "${BASE[@]}"
expect gradle-failure-fails gradle-plan FAIL
unset STUB_GRADLE_FAIL
rm -f "$REPO/android/gradlew"
run_gate "${BASE[@]}"
expect gradle-absent-skips gradle-plan SKIP
run_gate --exit "${BASE[@]}"
expect gradle-absent-fails-in-exit-mode gradle-plan FAIL

# ── 16. root-inventory ──
new_repo root
run_gate --fast "${BASE[@]}"
expect root-inventory-only-with-root root-inventory none
run_gate --fast --root "${BASE[@]}"
expect root-inventory-passes root-inventory PASS
export STUB_WS=1300
run_gate --fast --root "${BASE[@]}"
expect root-inventory-below-floor-fails root-inventory FAIL
export STUB_WS=1400 STUB_INV_FAIL=1
run_gate --fast --root "${BASE[@]}"
expect root-inventory-lost-test-fails root-inventory FAIL

# ── 17. emulator and report checks ──
new_repo emu
for f in uc_e2e.sh p7_powercut.sh p7_soak.sh; do cp "$FIX/emulator_script_stub.sh" "$REPO/scripts/android/$f"; done
run_gate --fast "${BASE[@]}"
check emulator-checks-only-when-asked $([[ "$(status uc-e2e)" == none && "$(status powercut)" == none && "$(status soak-report)" == none ]] && echo 0 || echo 1) "uc-e2e=$(status uc-e2e)"
: >"$STUB_LOG"
run_gate --fast --uc-e2e --powercut --soak-report "${BASE[@]}"
check emulator-checks-pass $([[ "$(status uc-e2e)" == PASS && "$(status powercut)" == PASS && "$(status soak-report)" == PASS && $RC -eq 0 ]] && echo 0 || echo 1) "rc=$RC $(grep -E 'uc-e2e|powercut|soak' "$OUT" | tr '\n' ' ')"
check soak-report-arguments $(grep -qx 'script p7_soak.sh report --latest --min-hours 4' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
: >"$STUB_LOG"
PILI_P7_SOAK_MIN_HOURS=24 run_gate --fast --soak-report "${BASE[@]}"
check soak-min-hours-override $(grep -qx 'script p7_soak.sh report --latest --min-hours 24' "$STUB_LOG" && echo 0 || echo 1) "log: $(cat "$STUB_LOG")"
export STUB_TOTAL=FAIL
run_gate --fast --uc-e2e "${BASE[@]}"
expect uc-e2e-failure-fails uc-e2e FAIL
export STUB_TOTAL=NONE
run_gate --fast --powercut "${BASE[@]}"
expect missing-total-line-fails powercut FAIL
unset STUB_TOTAL
rm -f "$REPO/scripts/android/p7_soak.sh"
run_gate --fast --soak-report "${BASE[@]}"
expect asked-but-absent-script-fails soak-report FAIL

# ── 18. exit codes ──
new_repo codes
run_gate --fast "${BASE[@]}"
rc_ok=$RC
printf 'x\n' >"$REPO/docs/other.md"
run_gate --fast "${BASE[@]}"
check exit-codes-0-and-1 $([[ $rc_ok -eq 0 && $RC -eq 1 ]] && echo 0 || echo 1) "clean rc=$rc_ok dirty rc=$RC"

total=$((pass + fail))
if [[ "$fail" -eq 0 ]]; then
    printf 'TESTP7GATE | total | PASS | %d tests\n' "$total"
    exit 0
fi
printf 'TESTP7GATE | total | FAIL | %d of %d tests failed\n' "$fail" "$total"
exit 1
