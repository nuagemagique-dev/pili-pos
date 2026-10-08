#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P2 gate
#
# One command that decides whether a P2 branch (the FFI facade pos-ffi in
# android/rust and the Gradle build in android/) is mergeable. Prints one line
# per check:
#     GATE | <check> | PASS|FAIL|SKIP | <detail>
# runs every check even after a failure, ends with exactly one
#     GATE | total | PASS|FAIL | <n> failed (<m> checks)
# and exits 1 on any FAIL. The output of a failed check is printed on stderr.
# A check whose inputs do not exist yet (android/rust, a tool script) is SKIP.
#
# Usage: scripts/android/p2_gate.sh [--fast] [--android] [--root] [--exit]
#                                   [--base REF] [--allow REGEX]
#                                   [--require-modules LIST]
#   --fast       skip the self-tests and the JVM tests (card gates)
#   --android    also build the arm64 library and check it (ndk-build, so-align,
#                bindgen-android); slow, for the integrator's phase gate
#   --root       also run the root test inventory (test-inventory); needs ui/dist
#   --exit       phase exit: also run jvm-check, ffi-doc and pass --strict to the
#                command parity check (84/84 required)
#   --base REF   branch the checks compare with (default android/int)
#   --allow REGEX  every changed file must also match REGEX (hygiene-paths); a
#                card gate gives the list of the card's allowed paths
#   --require-modules LIST  passed to check_command_parity.py (modules whose
#                facade must be complete in this card)
#
# Checks, in order
#   hygiene-paths     every file changed in BASE...HEAD plus every modified or
#                     untracked file of the working tree matches
#                     ^(android/|scripts/android/|docs/android/), not
#                     ^docs/android/baseline/, and REGEX when --allow is given
#   hygiene-emdash    no U+2014 in the added lines of git diff BASE...HEAD, of the
#                     working tree diff and in untracked files
#   hygiene-artifacts no tracked file under android/ or scripts/android/ that is a
#                     build output, key, local config or env file (build/, .gradle/,
#                     target/, .so .dylib .a .jks .keystore .p12, local.properties,
#                     /.env*), except android/gradle/wrapper/gradle-wrapper.jar
#   hygiene-subjects  every commit subject in BASE..HEAD (merges included) is a
#                     Conventional Commit
#   hygiene-root-lock Cargo.lock and Cargo.toml unchanged against BASE
#   frozen-provisional  check_linux_frozen.sh --ref 042d911... (PD48, PD63)
#   linux-frozen      check_linux_frozen.sh against the freeze tag; SKIP while there
#                     is none (exit 2)
#   rust-fmt, rust-clippy, rust-tests   in android/rust: cargo fmt --all --check,
#                     cargo clippy --workspace --all-targets --locked -D warnings,
#                     cargo test --workspace --locked (detail: summed passed count);
#                     SKIP when android/rust/Cargo.toml is absent
#   ffi-deps          cargo tree -p pos-ffi in android/rust lists pos-engine and no
#                     pos-peppol and no tauri; SKIP when android/rust is absent
#   lock-parity       check_lock_parity.sh (exit 2 = SKIP)
#   command-parity    check_command_parity.py [--require-modules LIST] [--strict with
#                     --exit]; SKIP when the script is absent
#   ffi-hygiene       check_command_parity.py --hygiene; SKIP when absent
#   Full mode only (not --fast):
#   selftest-frozen, selftest-lock-parity, selftest-command-parity (SKIP when the
#                     test is absent), jvm-tests (:pos-core:test in android/; SKIP
#                     while android/settings.gradle.kts is absent)
#   With --android:
#   ndk-build         cargo ndk -t arm64-v8a build --locked --profile android-release
#                     -p pos-ffi in android/rust
#   so-align          every LOAD segment of libpos_ffi.so has an alignment that is a
#                     multiple of 0x4000 (16 KB pages; 0x4000 is what the build sets)
#   bindgen-android   uniffi-bindgen generate --library that .so --language kotlin
#                     --config android-uniffi.toml gives pos_ffi.kt with `class PosEngine`
#   With --root:
#   test-inventory    scripts/android/test_inventory.sh --diff (root workspace)
#   With --exit:
#   jvm-check         :pos-core:check in android/ (includes apiCheck and mismatchTest)
#   ffi-doc           the block of docs/android/FFI.md between <!-- parity:begin -->
#                     and <!-- parity:end --> equals check_command_parity.py
#                     --markdown; SKIP while FFI.md is absent
#
# Needs python3, git and cargo (cargo-ndk and the NDK for --android, a JDK for the
# JVM checks). Cargo runs in android/rust without CARGO_TARGET_DIR, so the target
# directory is the one of android/rust/.cargo/config.toml (<repo>/target/android-rust).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=never

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT" || exit 2

PROVISIONAL_REF=042d91135940cb3f678a3f5e2947744a9d05ce4d

fast=0
do_android=0
do_root=0
exit_mode=0
base="android/int"
allow_re=""
require_modules=""
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast) fast=1 ;;
    --android) do_android=1 ;;
    --root) do_root=1 ;;
    --exit) exit_mode=1 ;;
    --base)
        base="${2:?--base needs a ref}"
        shift
        ;;
    --allow)
        allow_re="${2:?--allow needs a regex}"
        shift
        ;;
    --require-modules)
        require_modules="${2:?--require-modules needs a list}"
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

# gate_skip2 <check> <skip detail or -> <command...>: like gate, but exit 2 is SKIP
# (detail: the given text, or the last output line when it is `-`).
gate_skip2() {
    local name="$1" skipmsg="$2" log rc
    shift 2
    n_checks=$((n_checks + 1))
    log="$TMP/check.$n_checks.log"
    "$@" >"$log" 2>&1
    rc=$?
    if [[ $rc -eq 0 ]]; then
        printf 'GATE | %s | PASS | %s\n' "$name" "$(last_line "$log")"
    elif [[ $rc -eq 2 ]]; then
        [[ "$skipmsg" != "-" ]] || skipmsg="$(last_line "$log")"
        printf 'GATE | %s | SKIP | %s\n' "$name" "$skipmsg"
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

have_base() { git rev-parse -q --verify "$base^{commit}" >/dev/null 2>&1; }
have_rust() { [[ -f android/rust/Cargo.toml ]]; }

# in_rust <command...>: run in android/rust, target dir from its .cargo/config.toml.
in_rust() { (cd android/rust && env -u CARGO_TARGET_DIR "$@"); }

# ── hygiene ─────────────────────────────────────────────────────
# Files changed in BASE...HEAD, plus modified (staged or not) and untracked ones.
changed_files() {
    git diff --name-only --no-renames "$base...HEAD"
    git diff --name-only --no-renames HEAD
    git ls-files --others --exclude-standard
}

h_paths() {
    have_base || { echo "base ref not found: $base"; return 1; }
    local bad="" f
    while IFS= read -r f; do
        [[ -n "$f" ]] || continue
        if ! [[ "$f" =~ ^(android/|scripts/android/|docs/android/) ]] || [[ "$f" =~ ^docs/android/baseline/ ]]; then
            bad+="$f"$'\n'
        elif [[ -n "$allow_re" ]] && ! [[ "$f" =~ $allow_re ]]; then
            bad+="$f (not allowed by --allow)"$'\n'
        fi
    done < <(changed_files | LC_ALL=C sort -u)
    if [[ -n "$bad" ]]; then
        printf '%s' "$bad"
        echo "$(printf '%s' "$bad" | grep -c '^') changed path(s) outside the allowed set"
        return 1
    fi
    echo "$(changed_files | LC_ALL=C sort -u | grep -c .) changed path(s), all allowed"
}
gate hygiene-paths h_paths

EMDASH=$'\xe2\x80\x94'
h_emdash() {
    have_base || { echo "base ref not found: $base"; return 1; }
    local hits="" f
    hits+="$(git diff --no-color -U0 "$base...HEAD" | grep '^+' | grep -v '^+++ ' | grep -F -- "$EMDASH" || true)"$'\n'
    hits+="$(git diff --no-color -U0 HEAD | grep '^+' | grep -v '^+++ ' | grep -F -- "$EMDASH" || true)"$'\n'
    while IFS= read -r f; do
        [[ -n "$f" ]] || continue
        hits+="$(grep -IHF -- "$EMDASH" "$f" 2>/dev/null || true)"$'\n'
    done < <(git ls-files --others --exclude-standard)
    hits="$(printf '%s' "$hits" | grep -v '^[[:space:]]*$' || true)"
    if [[ -n "$hits" ]]; then
        printf '%s\n' "$hits" | cut -c1-200 | head -n 20
        echo "$(printf '%s\n' "$hits" | wc -l | tr -d ' ') added line(s) with an em-dash"
        return 1
    fi
    echo "no em-dash in added lines"
}
gate hygiene-emdash h_emdash

h_artifacts() {
    local bad
    bad="$(git ls-files -- android scripts/android |
        grep -v '^android/gradle/wrapper/gradle-wrapper\.jar$' |
        sed 's#^#/#' |
        grep -E '/build/|/\.gradle/|/target/|\.so$|\.dylib$|\.a$|\.jks$|\.keystore$|\.p12$|local\.properties$|/\.env' || true)"
    if [[ -n "$bad" ]]; then
        printf '%s\n' "$bad" | head -n 20
        echo "$(printf '%s\n' "$bad" | wc -l | tr -d ' ') tracked build output, key or local file(s)"
        return 1
    fi
    echo "no tracked build output, key or local file"
}
gate hygiene-artifacts h_artifacts

h_subjects() {
    have_base || { echo "base ref not found: $base"; return 1; }
    local bad n
    n="$(git log --format=%s "$base..HEAD" | wc -l | tr -d ' ')"
    bad="$(git log --format='%h %s' "$base..HEAD" |
        grep -vE '^[0-9a-f]+ (feat|fix|docs|style|refactor|perf|test|chore|build|ci)(\([a-z0-9._/-]+\))?!?: .+' || true)"
    if [[ -n "$bad" ]]; then
        printf '%s\n' "$bad"
        echo "$(printf '%s\n' "$bad" | wc -l | tr -d ' ') commit subject(s) are not Conventional Commits"
        return 1
    fi
    echo "$n commit subject(s) are Conventional Commits"
}
gate hygiene-subjects h_subjects

h_root_lock() {
    have_base || { echo "base ref not found: $base"; return 1; }
    git diff --quiet --no-renames "$base...HEAD" -- Cargo.lock Cargo.toml || { echo "Cargo.lock or Cargo.toml changed against $base"; return 1; }
    echo "Cargo.lock and Cargo.toml unchanged"
}
gate hygiene-root-lock h_root_lock

# ── freeze ──────────────────────────────────────────────────────
gate frozen-provisional bash scripts/android/check_linux_frozen.sh --ref "$PROVISIONAL_REF"
gate_skip2 linux-frozen "no freeze tag" bash scripts/android/check_linux_frozen.sh

# ── android/rust ────────────────────────────────────────────────
rust_tests() {
    local log="$TMP/rust_tests.out" rc sum
    in_rust cargo test --workspace --locked >"$log" 2>&1
    rc=$?
    [[ $rc -eq 0 ]] || tail -n 60 "$log"
    sum="$(sed -n 's/^test result: [a-z]*\. \([0-9][0-9]*\) passed.*/\1/p' "$log" | awk '{ s += $1 } END { print s + 0 }')"
    echo "$sum passed"
    return $rc
}

ffi_deps() {
    local tree="$TMP/ffi_deps.out"
    in_rust cargo tree --locked --offline -p pos-ffi -e normal,build --prefix none >"$tree" 2>&1 ||
        { tail -n 20 "$tree"; echo "cargo tree failed"; return 1; }
    grep -q '^pos-engine v' "$tree" || { echo "pos-engine is not a dependency of pos-ffi"; return 1; }
    if grep -qE '^pos-peppol v' "$tree"; then echo "pos-peppol is in the dependency graph of pos-ffi"; return 1; fi
    if grep -qE '^tauri(-[a-z0-9_-]+)? v' "$tree"; then
        grep -E '^tauri(-[a-z0-9_-]+)? v' "$tree" | head -n 3
        echo "tauri is in the dependency graph of pos-ffi"
        return 1
    fi
    echo "pos-engine yes, pos-peppol no, tauri no"
}

if have_rust; then
    gate rust-fmt in_rust cargo fmt --all --check -- --color never
    gate rust-clippy in_rust cargo clippy --workspace --all-targets --locked -- -D warnings
    gate rust-tests rust_tests
    gate ffi-deps ffi_deps
else
    skip rust-fmt "no android/rust yet"
    skip rust-clippy "no android/rust yet"
    skip rust-tests "no android/rust yet"
    skip ffi-deps "no android/rust yet"
fi

gate_skip2 lock-parity - bash scripts/android/check_lock_parity.sh

# ── parity tools ────────────────────────────────────────────────
PARITY=scripts/android/check_command_parity.py
if [[ -f "$PARITY" ]]; then
    parity_args=()
    [[ -z "$require_modules" ]] || parity_args+=(--require-modules "$require_modules")
    [[ "$exit_mode" -eq 0 ]] || parity_args+=(--strict)
    gate command-parity python3 "$PARITY" ${parity_args[@]+"${parity_args[@]}"}
    gate ffi-hygiene python3 "$PARITY" --hygiene
else
    skip command-parity "$PARITY is absent"
    skip ffi-hygiene "$PARITY is absent"
fi

# ── full mode ───────────────────────────────────────────────────
if [[ "$fast" -eq 0 ]]; then
    gate selftest-frozen bash scripts/android/test_check_linux_frozen.sh
    gate selftest-lock-parity python3 scripts/android/test_lock_parity.py
    if [[ -f scripts/android/test_check_command_parity.py ]]; then
        gate selftest-command-parity python3 scripts/android/test_check_command_parity.py
    else
        skip selftest-command-parity "scripts/android/test_check_command_parity.py is absent"
    fi
    if [[ -f android/settings.gradle.kts ]]; then
        gate jvm-tests bash -c 'cd android && ./gradlew --no-daemon --console=plain :pos-core:test'
    else
        skip jvm-tests "no android/settings.gradle.kts yet"
    fi
fi

# ── --android ───────────────────────────────────────────────────
SO="$ROOT/target/android-rust/aarch64-linux-android/android-release/libpos_ffi.so"

find_readelf() {
    local ndk="${ANDROID_NDK_HOME:-}" r
    [[ -n "$ndk" ]] || ndk="$HOME/Library/Android/sdk/ndk/27.1.12297006"
    for r in "$ndk"/toolchains/llvm/prebuilt/*/bin/llvm-readelf; do
        [[ -x "$r" ]] && { printf '%s' "$r"; return 0; }
    done
    return 1
}

ndk_build() {
    [[ -n "${ANDROID_NDK_HOME:-}" ]] || export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/27.1.12297006"
    command -v cargo-ndk >/dev/null || { echo "cargo-ndk is not installed"; return 1; }
    in_rust cargo ndk -t arm64-v8a build --locked --profile android-release -p pos-ffi || return 1
    [[ -f "$SO" ]] || { echo "missing $SO"; return 1; }
    echo "built libpos_ffi.so ($(wc -c <"$SO" | tr -d ' ') bytes)"
}

so_align() {
    [[ -f "$SO" ]] || { echo "missing $SO (run the ndk-build check first)"; return 1; }
    local readelf out n bad
    readelf="$(find_readelf)" || { echo "llvm-readelf not found under ANDROID_NDK_HOME"; return 1; }
    out="$("$readelf" -lW "$SO")" || { echo "llvm-readelf failed"; return 1; }
    n="$(printf '%s\n' "$out" | awk '$1 == "LOAD" { c++ } END { print c + 0 }')"
    [[ "$n" -gt 0 ]] || { echo "no LOAD segment found"; return 1; }
    bad="$(printf '%s\n' "$out" | awk '$1 == "LOAD" { print $NF }' | while read -r a; do
        v=$((a))
        [[ $((v % 16384)) -eq 0 && $v -gt 0 ]] || echo "$a"
    done)"
    if [[ -n "$bad" ]]; then
        printf '%s\n' "$bad"
        echo "LOAD segment alignment is not a multiple of 0x4000"
        return 1
    fi
    echo "$n LOAD segments, all aligned to $(printf '%s\n' "$out" | awk '$1 == "LOAD" { print $NF }' | sort -u | tr '\n' ' ')"
}

bindgen_android() {
    [[ -f "$SO" ]] || { echo "missing $SO (run the ndk-build check first)"; return 1; }
    local outdir="$TMP/bindgen" kt
    mkdir -p "$outdir"
    in_rust cargo run --locked -p uniffi-bindgen -- generate --library "$SO" --language kotlin \
        --config android-uniffi.toml --out-dir "$outdir" --no-format || return 1
    kt="$outdir/uniffi/pos_ffi/pos_ffi.kt"
    [[ -f "$kt" ]] || { echo "missing uniffi/pos_ffi/pos_ffi.kt"; return 1; }
    grep -q 'class PosEngine' "$kt" || { echo "pos_ffi.kt has no class PosEngine"; return 1; }
    echo "pos_ffi.kt generated ($(wc -l <"$kt" | tr -d ' ') lines, class PosEngine)"
}

if [[ "$do_android" -eq 1 ]]; then
    if have_rust; then
        gate ndk-build ndk_build
        gate so-align so_align
        gate bindgen-android bindgen_android
    else
        skip ndk-build "no android/rust yet"
        skip so-align "no android/rust yet"
        skip bindgen-android "no android/rust yet"
    fi
fi

# ── --root ──────────────────────────────────────────────────────
if [[ "$do_root" -eq 1 ]]; then
    gate test-inventory bash scripts/android/test_inventory.sh --diff
fi

# ── --exit ──────────────────────────────────────────────────────
ffi_doc() {
    local doc=docs/android/FFI.md block want
    block="$(awk '/^<!-- parity:end -->$/ { on = 0 } on { print } /^<!-- parity:begin -->$/ { on = 1 }' "$doc")"
    grep -qx '<!-- parity:begin -->' "$doc" && grep -qx '<!-- parity:end -->' "$doc" ||
        { echo "$doc lacks the parity:begin / parity:end markers"; return 1; }
    [[ -f "$PARITY" ]] || { echo "$PARITY is absent"; return 1; }
    want="$(python3 "$PARITY" --markdown)" || { echo "check_command_parity.py --markdown failed"; return 1; }
    if [[ "$block" != "$want" ]]; then
        diff <(printf '%s\n' "$block") <(printf '%s\n' "$want") | head -n 20
        echo "the parity block of $doc differs from check_command_parity.py --markdown"
        return 1
    fi
    echo "parity block of $doc is current ($(printf '%s\n' "$want" | wc -l | tr -d ' ') lines)"
}

if [[ "$exit_mode" -eq 1 ]]; then
    if [[ -f android/settings.gradle.kts ]]; then
        gate jvm-check bash -c 'cd android && ./gradlew --no-daemon --console=plain :pos-core:check'
    else
        skip jvm-check "no android/settings.gradle.kts yet"
    fi
    if [[ -f docs/android/FFI.md ]]; then
        gate ffi-doc ffi_doc
    else
        skip ffi-doc "docs/android/FFI.md is absent"
    fi
fi

if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks)\n' "$n_checks"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks)\n' "$n_failed" "$n_checks"
exit 1
