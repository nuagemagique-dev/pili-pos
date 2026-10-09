#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P7 gate (end-to-end lane, e2e-parity)
#
# One command that decides whether a P7 branch (UC scenarios and runners, android/uccheck,
# soak, power cut, GMS run, runbook) is mergeable. It does NOT wrap p4_gate.sh or p6_gate.sh:
# their freeze-tag, linux-frozen, hygiene-root-lock and hygiene-paths checks are red by
# design while the freeze is reopened (P7D3). It runs its own checks instead.
# Prints one line per check:
#     GATE | <check> | PASS|FAIL|SKIP | <detail>
# runs every check even after a failure, ends with exactly one
#     GATE | total | PASS|FAIL | <n> failed (<m> checks)
# and exits 1 on any FAIL (2 on a usage error). The output of a failed check goes to stderr.
#
# Usage: scripts/android/p7_gate.sh [--fast] [--exit] [--root] [--uc-e2e] [--powercut]
#                                   [--soak-report] [--base REF] [--allow ERE]...
#   --fast         skip gradle-plan, selftest-p7-gate and selftest-lock
#   --exit         phase-exit mode: a tool check that would SKIP because its tool is absent
#                  is a FAIL, and uc-matrix runs without --allow-pending
#   --root         add root-inventory
#   --uc-e2e       add uc-e2e         (emulator; takes the P7 lock inside the script)
#   --powercut     add powercut       (emulator)
#   --soak-report  add soak-report    (reads the soak result, no emulator)
#   --base REF     base of the diff checks (default android/lane-p7)
#   --allow ERE    extra allowed path regex for the scope check, repeatable; matched with
#                  bash [[ $path =~ $re ]] (POSIX ERE, no lookahead, no backslash classes)
#
# Checks, in this order (all run from the repository root)
#   hygiene-subjects  every non-merge commit of BASE..HEAD has a subject matching
#                     ^(feat|fix|docs|style|refactor|perf|test|chore|build|ci)(\(scope\))?!?:<space>
#   hygiene-dash      no em-dash (U+2014) or en-dash (U+2013) in the added lines of
#                     'git diff BASE' (commits and working tree) nor in an untracked text file
#   hygiene-secrets   no added or untracked file named *.jks, *.keystore, *.p12, *minisign.key
#                     or .env*; no added line with a private key block marker or with a
#                     value assigned to PILI_LIVE_FDM_TOKEN (the name, then an equal sign,
#                     then any non-blank character)
#   scope             every changed path (git diff --name-only BASE...HEAD plus git status
#                     --porcelain --untracked-files=all) matches 'lane_paths.py --regex
#                     e2e-parity', 'lane_paths.py --regex shared' or an --allow
#   lanes             lane_paths.py --check
#   linux-frozen      check_linux_frozen.sh --tag linux-v1.0.0-rc1 when that tag exists, else
#                     --ref ${PILI_P7_FREEZE_REF:-8a678ff}
#   frozen-paths      no diff against BASE and no untracked file in Cargo.toml Cargo.lock
#                     rust-toolchain.toml src-tauri ui crates deploy scripts/build_and_hash.sh
#                     scripts/check_repro.sh scripts/uc_matrix.py scripts/uc_scope.toml
#                     android/rust android/pos-core android/smoke docs/SUBMISSION_ARTIFACTS
#                     docs/android/baseline
#   kotlin-tests      kotlin_test_inventory.py --base BASE: PASS on its 'KTESTS | total | PASS'
#   command-parity    check_command_parity.py --strict
#   ui-parity         check_ui_parity.py --strict
#   uccheck-fmt       cargo fmt --manifest-path android/uccheck/Cargo.toml --all --check
#   uccheck-clippy    cargo clippy ... --all-targets --locked --offline --target-dir
#                     target/uccheck -- -D warnings
#   uccheck-test      cargo test ... --locked --offline --target-dir target/uccheck
#                     (these three: SKIP while android/uccheck/Cargo.toml is absent)
#   uccheck-lock      bash scripts/android/uc_lock_parity.sh        (SKIP while absent)
#   uc-scenarios      python3 scripts/android/uc_scenarios_check.py (SKIP while absent)
#   uc-replay         pos-uc-check record of every scenario group into target/p7-gate, then
#                     compare --linux target/p7-gate/linux --android
#                     docs/android/uc/journals/android --fixtures; PASS on
#                     'UCCHECK | total | PASS'. SKIP while docs/android/uc/journals/android
#                     (or android/uccheck) is absent
#   uc-matrix         python3 scripts/android/uc_matrix_android.py --check, plus
#                     --allow-pending unless --exit    (SKIP while absent)
#   Full mode only (not --fast):
#   gradle-plan       in android/: heavy.sh ./gradlew --offline lint ktlintCheck detekt test
#                     koverVerifyDebug assembleRelease :app:assembleDebugAndroidTest
#                     (SKIP while android/gradlew is absent)
#   selftest-p7-gate  scripts/android/test_p7_gate.sh        (SKIP when PILI_P7_NESTED=1)
#   selftest-lock     scripts/android/test_p7_emulator_lock.sh (SKIP when PILI_P7_NESTED=1)
#   --root:
#   root-inventory    scripts/android/test_inventory.sh --diff exits 0 and its 'ok ... (ws N
#                     >= M)' line has N at least 1355
#   Emulator and report checks (only when asked; an absent script is a FAIL, not a SKIP):
#   uc-e2e            bash scripts/android/uc_e2e.sh; PASS on 'UCE2E | total | PASS'
#   powercut          bash scripts/android/p7_powercut.sh; PASS on 'POWERCUT | total | PASS'
#   soak-report       bash scripts/android/p7_soak.sh report --latest --min-hours
#                     ${PILI_P7_SOAK_MIN_HOURS:-4}; PASS on 'SOAK | total | PASS'
#
# In --exit mode every tool check that would SKIP because its tool is absent is a FAIL.
#
# Environment
#   PILI_P7_ROOT        repository root (default: the root of this script); the tools are the
#                       files of that tree, which is how the self-test swaps in stubs
#   PILI_P7_NESTED=1    the two self-test checks are SKIP (set by the self-tests themselves)
#   PILI_P7_STUB_DIR    a directory prepended to PATH (stub cargo, rustc, python3, git hooks...)
#   PILI_P7_FREEZE_REF  commit used by linux-frozen when the tag is absent (default 8a678ff)
#   PILI_P7_SOAK_MIN_HOURS  hours required by soak-report (default 4)
#   PILI_GRADLE_JVMARGS     JVM args of the Gradle build (default -Xmx2g -XX:MaxMetaspaceSize=1g
#                           -Dfile.encoding=UTF-8)
#   RUSTC_WRAPPER, CARGO_INCREMENTAL, JAVA_HOME, ANDROID_HOME, ANDROID_NDK_HOME: set to the
#                       values of the Mac build environment when unset and present
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

TOOLS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SELF="$TOOLS/$(basename "${BASH_SOURCE[0]}")"
ROOT="${PILI_P7_ROOT:-$(cd "$TOOLS/../.." && pwd -P)}"

fast=0
exit_mode=0
do_root=0
do_uc_e2e=0
do_powercut=0
do_soak=0
base="android/lane-p7"
allows=()
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast) fast=1 ;;
    --exit) exit_mode=1 ;;
    --root) do_root=1 ;;
    --uc-e2e) do_uc_e2e=1 ;;
    --powercut) do_powercut=1 ;;
    --soak-report) do_soak=1 ;;
    --base)
        [[ $# -ge 2 && -n "$2" ]] || { echo "--base needs a ref" >&2; exit 2; }
        base="$2"
        shift
        ;;
    --allow)
        [[ $# -ge 2 && -n "$2" ]] || { echo "--allow needs a regex" >&2; exit 2; }
        # [[ =~ ]] returns 2 for an invalid ERE
        rc=0
        [[ "" =~ $2 ]] || rc=$?
        [[ "$rc" -ne 2 ]] || { echo "--allow: invalid regular expression: $2" >&2; exit 2; }
        allows+=("$2")
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

export PATH="$HOME/.cargo/bin:$PATH"
# The stub directory goes in front of everything, ~/.cargo/bin included.
if [[ -n "${PILI_P7_STUB_DIR:-}" ]]; then
    export PATH="$PILI_P7_STUB_DIR:$PATH"
fi
# Mac build environment, only where the caller did not choose.
if [[ -z "${RUSTC_WRAPPER:-}" && -x /opt/homebrew/bin/sccache ]]; then
    export RUSTC_WRAPPER=/opt/homebrew/bin/sccache SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-25G}"
fi
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
if [[ -z "${ANDROID_HOME:-}" && -d "$HOME/Library/Android/sdk" ]]; then
    export ANDROID_HOME="$HOME/Library/Android/sdk"
fi
if [[ -z "${ANDROID_NDK_HOME:-}" && -d "${ANDROID_HOME:-/nonexistent}/ndk/27.1.12297006" ]]; then
    export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.1.12297006"
fi
if [[ -z "${JAVA_HOME:-}" && -x /usr/libexec/java_home ]]; then
    jh="$(/usr/libexec/java_home -v 17 2>/dev/null || true)"
    [[ -z "$jh" ]] || export JAVA_HOME="$jh"
fi

cd "$ROOT" || exit 2

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
n_checks=0
n_failed=0

FREEZE_TAG="linux-v1.0.0-rc1"
FREEZE_REF="${PILI_P7_FREEZE_REF:-8a678ff}"
UCCHECK=android/uccheck/Cargo.toml
JOURNALS=docs/android/uc/journals/android
HEAVY="scripts/android/heavy.sh"
EMDASH=$'\xe2\x80\x94'
ENDASH=$'\xe2\x80\x93'
FROZEN_PATHS=(Cargo.toml Cargo.lock rust-toolchain.toml src-tauri ui crates deploy
    scripts/build_and_hash.sh scripts/check_repro.sh scripts/uc_matrix.py scripts/uc_scope.toml
    android/rust android/pos-core android/smoke docs/SUBMISSION_ARTIFACTS docs/android/baseline)

clean_detail() {
    local d
    d="$(tr '\n|' ' /' | sed 's/  */ /g; s/^ //; s/ $//' | cut -c1-160)"
    printf '%s' "${d:-ok}"
}

last_line() { grep -v '^[[:space:]]*$' "$1" | tail -n 1 | clean_detail; }

# gate <check> <command...>: PASS when the command exits 0; the detail is its last output line.
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

fail_now() { # <check> <detail>
    n_checks=$((n_checks + 1))
    n_failed=$((n_failed + 1))
    printf 'GATE | %s | FAIL | %s\n' "$1" "$2"
}

# tool_gate <check> <required file> <command...>: gate when the tool exists; else SKIP, or FAIL in --exit.
tool_gate() {
    local name="$1" tool="$2"
    shift 2
    if [[ -e "$tool" ]]; then
        gate "$name" "$@"
    elif [[ "$exit_mode" -eq 1 ]]; then
        fail_now "$name" "$tool is absent (--exit)"
    else
        skip "$name" "$tool is absent"
    fi
}

# heavy <command...>: under a heavy.sh slot when the script is there.
heavy() {
    if [[ -f "$HEAVY" ]]; then
        bash "$HEAVY" "$@"
    else
        "$@"
    fi
}

have_base() { git rev-parse -q --verify "$base^{commit}" >/dev/null 2>&1; }

# ── added text: 'path:line' for every added line of git diff BASE and of untracked text files ──
added_text() {
    git diff --no-color -U0 "$base" -- | awk '
        /^diff --git / { hdr = 1; path = ""; next }
        hdr && /^\+\+\+ / { p = substr($0, 5); if (p == "/dev/null") p = ""; sub(/^b\//, "", p); path = p; next }
        /^@@/ { hdr = 0; next }
        !hdr && /^\+/ { print path ":" substr($0, 2) }
    '
    local f
    while IFS= read -r -d '' f; do
        [[ -f "$f" ]] && grep -I -H '' "$f" 2>/dev/null
    done < <(git ls-files -z --others --exclude-standard)
    return 0
}

# added_paths: added, copied or renamed files of git diff BASE, and untracked files, one per line.
added_paths() {
    git -c core.quotepath=off diff --name-only --diff-filter=ACR "$base" --
    git -c core.quotepath=off ls-files --others --exclude-standard
}

# ── hygiene-subjects ────────────────────────────────────────────
h_subjects() {
    have_base || { echo "base ref not found: $base"; return 1; }
    local re='^(feat|fix|docs|style|refactor|perf|test|chore|build|ci)(\([a-z0-9-]+\))?!?: ' s bad=0 n=0
    while IFS= read -r s; do
        n=$((n + 1))
        if ! [[ "$s" =~ $re ]]; then
            echo "not a Conventional Commit: $s"
            bad=$((bad + 1))
        fi
    done < <(git log --no-merges --format=%s "$base..HEAD")
    if [[ "$bad" -gt 0 ]]; then
        echo "$bad of $n commit subject(s) are not Conventional Commits"
        return 1
    fi
    echo "$n commit subject(s), all Conventional Commits"
}
gate hygiene-subjects h_subjects

# ── hygiene-dash ────────────────────────────────────────────────
h_dash() {
    have_base || { echo "base ref not found: $base"; return 1; }
    added_text >"$TMP/added.txt"
    local hits
    hits="$(grep -F -e "$EMDASH" -e "$ENDASH" "$TMP/added.txt" | cut -c1-200 || true)"
    if [[ -n "$hits" ]]; then
        printf '%s\n' "$hits" | head -n 20
        echo "$(printf '%s\n' "$hits" | wc -l | tr -d ' ') added line(s) with an em-dash or an en-dash"
        return 1
    fi
    echo "no em-dash or en-dash in the added lines"
}
gate hygiene-dash h_dash

# ── hygiene-secrets ─────────────────────────────────────────────
h_secrets() {
    have_base || { echo "base ref not found: $base"; return 1; }
    local bad="" p b hits
    while IFS= read -r p; do
        [[ -n "$p" ]] || continue
        b="${p##*/}"
        case "$b" in
        *.jks | *.keystore | *.p12 | *minisign.key | .env*) bad+="secret-looking file: $p"$'\n' ;;
        esac
    done < <(added_paths | LC_ALL=C sort -u)
    added_text >"$TMP/added-secrets.txt"
    hits="$(grep -E 'PRIVATE KEY-{5}' "$TMP/added-secrets.txt" | cut -d: -f1 | LC_ALL=C sort -u || true)"
    [[ -z "$hits" ]] || bad+="$(printf '%s\n' "$hits" | sed 's/^/private key block in: /')"$'\n'
    # built in two pieces so that this file does not match its own pattern
    hits="$(grep -E "PILI_LIVE_FDM_TOKE""N=[^[:space:]]" "$TMP/added-secrets.txt" | cut -d: -f1 | LC_ALL=C sort -u || true)"
    [[ -z "$hits" ]] || bad+="$(printf '%s\n' "$hits" | sed 's/^/live FDM token value in: /')"$'\n'
    bad="$(printf '%s' "$bad" | grep -v '^[[:space:]]*$' || true)"
    if [[ -n "$bad" ]]; then
        printf '%s\n' "$bad" | head -n 20
        echo "$(printf '%s\n' "$bad" | wc -l | tr -d ' ') secret finding(s)"
        return 1
    fi
    echo "no keystore, key block, .env file or live token value added"
}
gate hygiene-secrets h_secrets

# ── scope ───────────────────────────────────────────────────────
changed_paths() {
    git -c core.quotepath=off diff -z --name-only --no-renames "$base...HEAD" -- | tr '\0' '\n'
    local ent skip_next=0 xy
    while IFS= read -r -d '' ent; do
        if [[ "$skip_next" -eq 1 ]]; then
            skip_next=0
            printf '%s\n' "$ent"
            continue
        fi
        xy="${ent:0:2}"
        printf '%s\n' "${ent:3}"
        case "$xy" in *R* | *C*) skip_next=1 ;; esac
    done < <(git status --porcelain -z --untracked-files=all)
}

h_scope() {
    have_base || { echo "base ref not found: $base"; return 1; }
    local e2e shared p a ok bad="" n=0 files
    e2e="$(python3 scripts/android/lane_paths.py --regex e2e-parity)" || { echo "lane_paths.py --regex e2e-parity failed"; return 1; }
    shared="$(python3 scripts/android/lane_paths.py --regex shared)" || { echo "lane_paths.py --regex shared failed"; return 1; }
    [[ -n "$e2e" && -n "$shared" ]] || { echo "lane_paths.py printed an empty regex"; return 1; }
    files="$(changed_paths | grep -v '^$' | LC_ALL=C sort -u || true)"
    while IFS= read -r p; do
        [[ -n "$p" ]] || continue
        n=$((n + 1))
        ok=0
        if [[ "$p" =~ $e2e || "$p" =~ $shared ]]; then
            ok=1
        else
            for a in ${allows[@]+"${allows[@]}"}; do
                if [[ "$p" =~ $a ]]; then
                    ok=1
                    break
                fi
            done
        fi
        [[ "$ok" -eq 1 ]] || bad+="$p"$'\n'
    done <<<"$files"
    if [[ -n "$bad" ]]; then
        printf '%s' "$bad" | head -n 30
        echo "$(printf '%s' "$bad" | grep -c .) of $n changed path(s) outside the e2e-parity scope"
        return 1
    fi
    echo "$n changed path(s), all in scope"
}
gate scope h_scope

# ── lanes ───────────────────────────────────────────────────────
gate lanes python3 scripts/android/lane_paths.py --check

# ── linux-frozen ────────────────────────────────────────────────
linux_frozen() {
    if git rev-parse -q --verify "refs/tags/$FREEZE_TAG" >/dev/null 2>&1; then
        bash scripts/android/check_linux_frozen.sh --tag "$FREEZE_TAG"
    else
        bash scripts/android/check_linux_frozen.sh --ref "$FREEZE_REF"
    fi
}
gate linux-frozen linux_frozen

# ── frozen-paths ────────────────────────────────────────────────
frozen_paths() {
    have_base || { echo "base ref not found: $base"; return 1; }
    local changed untracked
    changed="$(git -c core.quotepath=off diff --name-only "$base" -- "${FROZEN_PATHS[@]}")"
    untracked="$(git -c core.quotepath=off ls-files --others --exclude-standard -- "${FROZEN_PATHS[@]}")"
    if [[ -n "$changed" || -n "$untracked" ]]; then
        [[ -z "$changed" ]] || printf '%s\n' "$changed" | sed 's/^/changed: /' | head -n 20
        [[ -z "$untracked" ]] || printf '%s\n' "$untracked" | sed 's/^/untracked: /' | head -n 20
        echo "a frozen path differs from $base"
        return 1
    fi
    echo "${#FROZEN_PATHS[@]} frozen paths equal $base, none untracked"
}
gate frozen-paths frozen_paths

# ── kotlin-tests, command-parity, ui-parity ─────────────────────
kotlin_tests() {
    local out rc
    out="$(python3 scripts/android/kotlin_test_inventory.py --base "$base" 2>"$TMP/kt.err")"
    rc=$?
    cat "$TMP/kt.err"
    printf '%s\n' "$out" | sed -n 's/^KTESTS | total | [A-Z]* | //p'
    [[ $rc -eq 0 ]] && printf '%s\n' "$out" | grep -q '^KTESTS | total | PASS'
}
gate kotlin-tests kotlin_tests
gate command-parity python3 scripts/android/check_command_parity.py --strict
gate ui-parity python3 scripts/android/check_ui_parity.py --strict

# ── android/uccheck ─────────────────────────────────────────────
UC_TARGET=target/uccheck
tool_gate uccheck-fmt "$UCCHECK" cargo fmt --manifest-path "$UCCHECK" --all --check
tool_gate uccheck-clippy "$UCCHECK" heavy cargo clippy --manifest-path "$UCCHECK" --all-targets --locked --offline \
    --target-dir "$UC_TARGET" -- -D warnings
tool_gate uccheck-test "$UCCHECK" heavy cargo test --manifest-path "$UCCHECK" --locked --offline --target-dir "$UC_TARGET"
tool_gate uccheck-lock scripts/android/uc_lock_parity.sh bash scripts/android/uc_lock_parity.sh
tool_gate uc-scenarios scripts/android/uc_scenarios_check.py python3 scripts/android/uc_scenarios_check.py

uc_replay() {
    local out="$TMP/replay.out" rc=0
    rm -rf target/p7-gate
    heavy cargo run --manifest-path "$UCCHECK" --locked --offline --release --target-dir "$UC_TARGET" -p pos-uc-check -- \
        record --scenarios android/app/src/androidTest/assets/uc --out target/p7-gate || return 1
    heavy cargo run --manifest-path "$UCCHECK" --locked --offline --release --target-dir "$UC_TARGET" -p pos-uc-check -- \
        compare --linux target/p7-gate/linux --android "$JOURNALS" --fixtures >"$out" 2>&1 || rc=$?
    grep -E '^UCCHECK \| .* \| FAIL' "$out" | head -n 10
    if [[ "$rc" -ne 0 ]] || ! grep -q '^UCCHECK | total | PASS' "$out"; then
        tail -n 10 "$out"
        echo "compare exited $rc without 'UCCHECK | total | PASS'"
        return 1
    fi
    grep '^UCCHECK | total | PASS' "$out" | tail -n 1 | sed 's/^UCCHECK | total | PASS | //'
}
if [[ ! -f "$UCCHECK" ]]; then
    tool_gate uc-replay "$UCCHECK" uc_replay
else
    tool_gate uc-replay "$JOURNALS" uc_replay
fi

matrix_args=(--check)
[[ "$exit_mode" -eq 1 ]] || matrix_args+=(--allow-pending)
tool_gate uc-matrix scripts/android/uc_matrix_android.py python3 scripts/android/uc_matrix_android.py "${matrix_args[@]}"

# ── full mode: Gradle plan and self-tests ───────────────────────
gradle_plan() {
    local jvmargs="${PILI_GRADLE_JVMARGS:--Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8}"
    (
        cd android || exit 2
        if [[ -f ../scripts/android/heavy.sh ]]; then
            bash ../scripts/android/heavy.sh ./gradlew --no-daemon --console=plain -Dorg.gradle.jvmargs="$jvmargs" --offline \
                lint ktlintCheck detekt test koverVerifyDebug assembleRelease :app:assembleDebugAndroidTest
        else
            ./gradlew --no-daemon --console=plain -Dorg.gradle.jvmargs="$jvmargs" --offline \
                lint ktlintCheck detekt test koverVerifyDebug assembleRelease :app:assembleDebugAndroidTest
        fi
    )
}
if [[ "$fast" -eq 0 ]]; then
    tool_gate gradle-plan android/gradlew gradle_plan
    if [[ -n "${PILI_P7_NESTED:-}" ]]; then
        skip selftest-p7-gate "nested in a self-test"
        skip selftest-lock "nested in a self-test"
    else
        export PILI_P7_NESTED=1
        tool_gate selftest-p7-gate scripts/android/test_p7_gate.sh bash scripts/android/test_p7_gate.sh
        tool_gate selftest-lock scripts/android/test_p7_emulator_lock.sh bash scripts/android/test_p7_emulator_lock.sh
        unset PILI_P7_NESTED
    fi
fi

# ── --root ──────────────────────────────────────────────────────
root_inventory() {
    local out="$TMP/inv.out" rc=0 ws
    heavy bash scripts/android/test_inventory.sh --diff >"$out" 2>&1 || rc=$?
    tail -n 5 "$out"
    [[ "$rc" -eq 0 ]] || { echo "test_inventory.sh --diff exited $rc"; return 1; }
    ws="$(sed -n 's/^ok .*(ws \([0-9][0-9]*\) >= [0-9]*).*/\1/p' "$out" | tail -n 1)"
    [[ -n "$ws" ]] || { echo "no 'ws N' count in the output of test_inventory.sh"; return 1; }
    [[ "$ws" -ge 1355 ]] || { echo "ws count $ws is below 1355"; return 1; }
    echo "no test disappeared, ws $ws >= 1355"
}
if [[ "$do_root" -eq 1 ]]; then
    tool_gate root-inventory scripts/android/test_inventory.sh root_inventory
fi

# ── emulator and report checks ──────────────────────────────────
# total_gate <check> <script> <TAG> <command...>: PASS on '<TAG> | total | PASS'. An absent script is a FAIL.
total_gate() {
    local name="$1" script="$2" tag="$3"
    shift 3
    if [[ ! -e "$script" ]]; then
        fail_now "$name" "$script is absent"
        return
    fi
    gate "$name" total_run "$tag" "$@"
}
total_run() {
    local tag="$1" out="$TMP/total.$n_checks.out" rc=0
    shift
    "$@" >"$out" 2>&1 || rc=$?
    grep -E "^$tag \\| .* \\| FAIL" "$out" | head -n 10
    if [[ "$rc" -ne 0 ]] || ! grep -q "^$tag | total | PASS" "$out"; then
        tail -n 10 "$out"
        echo "exit $rc without '$tag | total | PASS'"
        return 1
    fi
    grep "^$tag | total | PASS" "$out" | tail -n 1 | sed "s/^$tag | total | PASS | //"
}
if [[ "$do_uc_e2e" -eq 1 ]]; then
    total_gate uc-e2e scripts/android/uc_e2e.sh UCE2E bash scripts/android/uc_e2e.sh
fi
if [[ "$do_powercut" -eq 1 ]]; then
    total_gate powercut scripts/android/p7_powercut.sh POWERCUT bash scripts/android/p7_powercut.sh
fi
if [[ "$do_soak" -eq 1 ]]; then
    total_gate soak-report scripts/android/p7_soak.sh SOAK \
        bash scripts/android/p7_soak.sh report --latest --min-hours "${PILI_P7_SOAK_MIN_HOURS:-4}"
fi

if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks)\n' "$n_checks"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks)\n' "$n_failed" "$n_checks"
exit 1
