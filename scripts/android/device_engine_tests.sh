#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - the engine's own Rust tests on the Android emulator (plan P3.3,
# DECISIONS 11.12, PD82)
#
# Cross-builds the test binaries of pos-engine (--no-default-features --features cert,
# the certified and Android feature set) and of the other crates of the cert build
# graph for aarch64-linux-android (API 33), pushes each one to the emulator, runs it
# through adb, reruns every failed test once alone, classifies the failures and checks
# the result against scripts/android/device_tests_exclude.txt.
#
# Usage: scripts/android/device_engine_tests.sh [--build-only | --run-only]
#                                               [--check] [--report-only]
#                                               [--crates LIST] [--keep] [--host-list]
#                                               [--fixtures]
#   --build-only   cross-build only; writes target/android-device-tests/manifest/<crate>.tsv
#   --run-only     skip the build, run the binaries of the last build
#   --report-only  skip build and run, classify the raw results of the last run
#   --check        the verdict: exit 1 when the run is not clean (see below). Without it
#                  the same lines are printed and the exit status is 0 unless the
#                  infrastructure failed (no emulator, build error, bad exclusion file)
#   --crates LIST  comma or space separated crate names (default: the cert build graph:
#                  pos-engine pos-core pos-db pos-fiscal pos-print pos-mail pos-hubrise
#                  pos-catalog pos-hubrise-types)
#   --keep         keep /data/local/tmp/pili-p3-device-tests on the device and the
#                  staging directory of stripped binaries
#   --fixtures     give the tests the fixtures they read from the host checkout (see
#                  below). Own target directory target/android-device-tests-fx and own
#                  results; the host-fixture lines of the exclusion file are not applied
#   --host-list    also build the same test binaries for the host (target
#                  android-device-tests-host, debug info off) and compare their --list
#                  count with the device count (a WARN line when they differ; informational)
#
# Features per crate (the cert build graph, read with
#   cargo tree -p pili-pos --no-default-features --features cert -e features):
#   pos-engine  --no-default-features --features cert      pos-db  --features hubrise
#   the others  default features
#
# Why not `cargo ndk test`: cargo-ndk 4.1.2 swallows cargo's --message-format=json output,
# and the executables are read from it. The script therefore sources `cargo ndk-env`
# (the compiler, linker and runner variables cargo-ndk itself sets) and runs
# `cargo test --no-run --target aarch64-linux-android` directly, with the NDK clang
# (aarch64-linux-android33-clang) as the linker: the same compiler invocations. The root Cargo.lock is never touched (--locked, and
# git diff --quiet Cargo.lock is checked after the build). Profile tweaks are
# environment only (CARGO_PROFILE_DEV_DEBUG=0).
#
# Fixtures. Tests read their fixtures (docs/20260821_UC_*, crates/*/fixtures, crates/*/tests/
# fixtures) through env!("CARGO_MANIFEST_DIR"), an absolute path of the host checkout that
# does not exist on the device. Default mode: those tests fail, and each is listed in
# scripts/android/device_tests_exclude.txt as host-fixture (nothing in crates/ changes).
# --fixtures mode: rustc runs behind a wrapper that sets CARGO_MANIFEST_DIR to
# /data/local/tmp/pili-p3-device-tests/repo/<same relative path>, and the fixture
# directories are pushed there, so the very same test sources run with their data. Only
# the compile-time value of one environment variable differs; no source or fixture is edited.
#
# Output (always the same lines; --check only decides the exit status)
#   DEVICE | <crate>/<target> | PASS|FAIL | passed N, failed N, excluded N, ignored N, flaky N
#   DEVICE | WARN | android-difference | <crate>/<target>::<test> | <reason>
#   DEVICE | WARN | flaky | <crate>/<target>::<test>
#   DEVICE | WARN | host-count | <crate>/<target> | device N, host N        (--host-list)
#   DEVICE | total | PASS|FAIL | <n> binaries, passed N, failed N, excluded N, ignored N, flaky N
# "failed" counts every test that still fails after the lone rerun, excluded ones
# included; "excluded" is the part of "failed" listed in the exclusion file. passed +
# failed + ignored equals the --list count of the binary. A test that fails in the
# full run and passes alone counts as passed and as flaky. Doc-tests are not run (the
# crates have none), #[ignore] tests never run (live tests stay ignored).
# Failure details go to stderr. The machine-readable result is written to
# target/android-device-tests/results/report.json.
#
# --check exits 1 when: a test fails that the exclusion file does not list; an excluded
# test passes or does not exist (stale entry); passed + failed + ignored differs from the
# --list count of a binary; a binary did not run to its end (crash, timeout, no "test
# result" line); a host count differs (never; WARN only). Exit 2: usage or bad exclusion
# file; exit 3: infrastructure (no emulator, build error, Cargo.lock changed, disk).
#
# Device side: everything lives in /data/local/tmp/pili-p3-device-tests/ (<crate>/<bin>,
# tmp/ = TMPDIR of the tests) and is removed at the end (trap) unless --keep. The
# emulator is reused or started by scripts/android/emulator.sh and never stopped.
# Cost on an M1 with 16 GB: see docs/android/DEVICE_TESTS.md.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT" || exit 3

DEFAULT_CRATES="pos-engine pos-core pos-db pos-fiscal pos-print pos-mail pos-hubrise pos-catalog pos-hubrise-types"
EXCLUDE_FILE="$ROOT/scripts/android/device_tests_exclude.txt"
OUT="$ROOT/target/android-device-tests"
HOST_TARGET_DIR="$ROOT/target/android-device-tests-host"
MANIFEST="$OUT/manifest"
RES="$OUT/results"
STAGE="$OUT/staging"
DEV_DIR="/data/local/tmp/pili-p3-device-tests"
TRIPLE="aarch64-linux-android"
RUN_TIMEOUT=1800
MIN_FREE_GB="${DEVICE_TESTS_MIN_FREE_GB:-5}"

do_build=1
do_run=1
do_report=1
check=0
keep=0
host_list=0
fixtures=0
crates="$DEFAULT_CRATES"

usage() {
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//' >&2
    exit 2
}

while [[ $# -gt 0 ]]; do
    case "$1" in
    --build-only) do_run=0; do_report=0 ;;
    --run-only) do_build=0 ;;
    --report-only) do_build=0; do_run=0 ;;
    --check) check=1 ;;
    --keep) keep=1 ;;
    --host-list) host_list=1 ;;
    --fixtures) fixtures=1 ;;
    --crates)
        [[ $# -ge 2 ]] || usage
        crates="${2//,/ }"
        shift
        ;;
    -h | --help) usage ;;
    *) echo "device_engine_tests.sh: unknown option $1" >&2; usage ;;
    esac
    shift
done

log() { printf 'device_engine_tests.sh: %s\n' "$*" >&2; }
die() { # <code> <message>
    log "$2"
    exit "$1"
}
now() { date +%s; }

features_for() { # <crate> -> cargo feature arguments
    case "$1" in
    pos-engine) echo "--no-default-features --features cert" ;;
    pos-db) echo "--features hubrise" ;;
    *) echo "" ;;
    esac
}

setup_env() {
    export PATH="$HOME/.cargo/bin:$PATH"
    unset CARGO_TARGET_DIR
    export CARGO_INCREMENTAL=0
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
    export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
    export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
    export CARGO_PROFILE_DEV_DEBUG=0
    export PATH="$ANDROID_HOME/platform-tools:$PATH"
}

free_gb() {
    df -k "$ROOT" | awk 'NR==2 { printf "%d", $4 / 1048576 }'
}

check_disk() {
    local f
    f="$(free_gb)"
    [[ "$f" -ge "$MIN_FREE_GB" ]] || die 3 "only $f GB free (minimum $MIN_FREE_GB GB, DEVICE_TESTS_MIN_FREE_GB)"
}

# ── python helper: executables of a cargo JSON stream ──────────
# stdin: cargo --message-format=json. stdout: crate<TAB>target<TAB>kind<TAB>path
extract_executables() { # <crate>
    python3 -I -c '
import json, sys
crate = sys.argv[1]
seen = set()
for line in sys.stdin:
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") != "compiler-artifact":
        continue
    if not m.get("profile", {}).get("test") or not m.get("executable"):
        continue
    t = m["target"]
    kinds = t.get("kind", [])
    # only the targets of the requested package
    if ("/crates/" + crate + "/") not in t.get("src_path", ""):
        continue
    kind = "lib" if "lib" in kinds or "rlib" in kinds else kinds[0] if kinds else "test"
    name = "lib" if kind == "lib" else t["name"]
    if kind == "bin":
        name = "bin-" + t["name"]
    exe = m["executable"]
    if exe in seen:
        continue
    seen.add(exe)
    print("\t".join([crate, name, kind, exe]))
' "$1"
}

# ── build ───────────────────────────────────────────────────────
# rustc wrapper of --fixtures: env!("CARGO_MANIFEST_DIR") expands to the device copy of the
# checkout. Only the process environment of rustc changes.
write_wrapper() {
    mkdir -p "$OUT"
    cat >"$OUT/rustc-manifest-dir.sh" <<'WRAP'
#!/bin/bash
rustc="$1"
shift
case "${CARGO_MANIFEST_DIR:-}" in
"$FX_REPO"/*) export CARGO_MANIFEST_DIR="$FX_DEV${CARGO_MANIFEST_DIR#"$FX_REPO"}" ;;
esac
exec "$rustc" "$@"
WRAP
    chmod +x "$OUT/rustc-manifest-dir.sh"
}

build_crate() { # <crate> <target-kind: device|host>
    local crate="$1" kind="$2" feats json err s rc tdir
    feats="$(features_for "$crate")"
    json="$OUT/build-$kind-$crate.json"
    err="$OUT/build-$kind-$crate.log"
    check_disk
    s="$(now)"
    if [[ "$kind" == "device" ]]; then
        tdir="$OUT"
        # shellcheck disable=SC2086
        (
            eval "$(cargo ndk-env -t arm64-v8a --platform 33 2>/dev/null | grep '^export ')"
            # cargo-ndk is only a linker wrapper under `cargo ndk`; here the NDK clang links
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${CC_aarch64_linux_android%/clang}/aarch64-linux-android33-clang"
            [[ -x "$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER" ]] || exit 97
            if [[ $fixtures -eq 1 ]]; then
                write_wrapper
                export FX_REPO="$ROOT" FX_DEV="$DEV_DIR/repo" RUSTC_WRAPPER="$OUT/rustc-manifest-dir.sh"
            fi
            cargo test --no-run --locked --target "$TRIPLE" -p "$crate" $feats \
                --target-dir "$tdir" --message-format=json >"$json" 2>"$err"
        )
        rc=$?
    else
        tdir="$HOST_TARGET_DIR"
        # shellcheck disable=SC2086
        cargo test --no-run --locked -p "$crate" $feats \
            --target-dir "$tdir" --message-format=json >"$json" 2>"$err"
        rc=$?
    fi
    if [[ $rc -ne 0 ]]; then
        tail -30 "$err" >&2
        die 3 "build of $crate ($kind) failed (rc=$rc), log $err"
    fi
    mkdir -p "$MANIFEST"
    extract_executables "$crate" <"$json" | sort >"$MANIFEST/$kind-$crate.tsv"
    local n
    n="$(wc -l <"$MANIFEST/$kind-$crate.tsv" | tr -d ' ')"
    [[ "$n" -gt 0 ]] || die 3 "no test executable found in the cargo output of $crate ($kind)"
    log "built $crate ($kind): $n test binaries in $(( $(now) - s )) s"
    echo "$crate $kind $(( $(now) - s ))" >>"$OUT/build-timings.txt"
}

do_build_phase() {
    command -v cargo >/dev/null || die 3 "cargo not found"
    command -v cargo-ndk >/dev/null || die 3 "cargo-ndk not found (cargo install cargo-ndk)"
    mkdir -p "$OUT"
    : >"$OUT/build-timings.txt"
    local c s
    s="$(now)"
    for c in $crates; do
        build_crate "$c" device
    done
    if ! git -C "$ROOT" diff --quiet Cargo.lock; then
        die 3 "Cargo.lock changed during the build (the build must be --locked)"
    fi
    echo "TOTAL device $(( $(now) - s ))" >>"$OUT/build-timings.txt"
    if [[ $host_list -eq 1 ]]; then
        for c in $crates; do
            build_crate "$c" host
        done
        git -C "$ROOT" diff --quiet Cargo.lock || die 3 "Cargo.lock changed during the host build"
    fi
    log "build done in $(( $(now) - s )) s, $(free_gb) GB free"
}

# ── device ──────────────────────────────────────────────────────
SERIAL=""
adbd() { adb -s "$SERIAL" "$@" </dev/null; } # adb must not eat the stdin of the manifest loops

device_cleanup() {
    if [[ -n "$SERIAL" && $keep -eq 0 ]]; then
        adb -s "$SERIAL" shell "rm -rf $DEV_DIR" >/dev/null 2>&1
        rm -rf "$STAGE"
    fi
}

ensure_device() {
    if [[ -n "${ANDROID_SERIAL:-}" ]] && adb -s "$ANDROID_SERIAL" get-state >/dev/null 2>&1; then
        SERIAL="$ANDROID_SERIAL"
    else
        SERIAL="$("$ROOT/scripts/android/emulator.sh" ensure)" || die 3 "emulator.sh ensure failed"
    fi
    [[ -n "$SERIAL" ]] || die 3 "no emulator serial"
    export ANDROID_SERIAL="$SERIAL"
    log "device $SERIAL ($(adbd shell getprop ro.product.cpu.abi | tr -d '\r'), API $(adbd shell getprop ro.build.version.sdk | tr -d '\r'))"
}

find_strip() {
    local f
    for f in "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-strip; do
        [[ -x "$f" ]] && { echo "$f"; return 0; }
    done
    return 1
}

# libtest names of the FAILED lines of a run file (stdin: run output)
failed_names() {
    sed -n 's/^test \(.*\) \.\.\. FAILED$/\1/p' | sed 's/ - should panic$//'
}

run_binary() { # <crate> <target> <host-path>
    local crate="$1" target="$2" path="$3" d="$RES/$1__$2" strip stripped rdir
    mkdir -p "$d" "$STAGE/$crate"
    rm -f "$d"/*.txt
    strip="$(find_strip)" || die 3 "llvm-strip not found in the NDK"
    stripped="$STAGE/$crate/$target"
    "$strip" --strip-debug -o "$stripped" "$path" || die 3 "llvm-strip failed on $path"
    echo "$(stat -c %s "$path" 2>/dev/null || stat -f %z "$path") $(stat -c %s "$stripped" 2>/dev/null || stat -f %z "$stripped")" >"$d/sizes.txt"
    rdir="$DEV_DIR/$crate"
    local cwd="$rdir"
    [[ $fixtures -eq 0 ]] || cwd="$DEV_DIR/repo/crates/$crate"
    adbd shell "mkdir -p $rdir $DEV_DIR/tmp" || die 3 "adb mkdir failed"
    adbd push -q "$stripped" "$rdir/$target" >/dev/null 2>&1 || die 3 "adb push of $crate/$target failed"
    adbd shell "chmod 755 $rdir/$target"
    [[ $keep -eq 1 ]] || rm -f "$stripped"
    local bin="$rdir/$target" env="TMPDIR=$DEV_DIR/tmp" t0
    # list (tests, then the ignored ones)
    adbd shell "cd $cwd && $env $bin --list 2>&1; echo EXIT=\$?" >"$d/list.txt"
    adbd shell "cd $cwd && $env $bin --list --ignored 2>&1; echo EXIT=\$?" >"$d/list_ignored.txt"
    # run
    t0="$(now)"
    adbd shell "cd $cwd && $env timeout $RUN_TIMEOUT $bin 2>&1; echo EXIT=\$?" >"$d/run.txt"
    echo "SECS=$(( $(now) - t0 ))" >>"$d/run.txt"
    # rerun each failed test alone
    local name n=0
    : >"$d/rerun.txt"
    while IFS= read -r name; do
        [[ -n "$name" ]] || continue
        n=$((n + 1))
        printf '##### RERUN %s\n' "$name" >>"$d/rerun.txt"
        adbd shell "cd $cwd && $env timeout $RUN_TIMEOUT $bin '$name' --exact --test-threads=1 2>&1; echo EXIT=\$?" >>"$d/rerun.txt"
    done < <(failed_names <"$d/run.txt")
    adbd shell "rm -f $rdir/$target" >/dev/null 2>&1
    log "ran $crate/$target ($(grep -c '^test .* \.\.\. ' "$d/run.txt") results, $n rerun, $(sed -n 's/^SECS=//p' "$d/run.txt") s)"
}

# --fixtures: the fixture directories of the checkout, at the path the tests were compiled with
push_fixtures() {
    local d c s base="$DEV_DIR/repo"
    adbd shell "mkdir -p $base/docs $base/crates/pos-engine" || die 3 "cannot create $base"
    for d in "$ROOT"/docs/20260821_UC_*; do
        [[ -d "$d" ]] && { adbd push -q "$d" "$base/docs/" >/dev/null 2>&1 || die 3 "push of $d failed"; }
    done
    for c in "$ROOT"/crates/*; do
        [[ -d "$c" ]] || continue
        adbd shell "mkdir -p $base/crates/$(basename "$c")"
        for s in tests fixtures; do
            [[ -d "$c/$s" ]] && { adbd push -q "$c/$s" "$base/crates/$(basename "$c")/" >/dev/null 2>&1 || die 3 "push of $c/$s failed"; }
        done
    done
    log "fixtures pushed to $base ($(adbd shell "du -sk $base" | awk '{print $1}') KB)"
}

do_run_phase() {
    ensure_device
    trap device_cleanup EXIT
    local t0 c line target path
    t0="$(now)"
    adbd shell "rm -rf $DEV_DIR; mkdir -p $DEV_DIR/tmp" || die 3 "cannot create $DEV_DIR"
    [[ $fixtures -eq 0 ]] || push_fixtures
    mkdir -p "$RES"
    for c in $crates; do
        [[ -f "$MANIFEST/device-$c.tsv" ]] || die 3 "no build manifest for $c (run without --run-only, or --build-only first)"
        while IFS=$'\t' read -r _ target _ path; do
            [[ -f "$path" ]] || die 3 "missing test binary $path (rebuild)"
            run_binary "$c" "$target" "$path"
        done <"$MANIFEST/device-$c.tsv"
    done
    echo "$(( $(now) - t0 ))" >"$OUT/run-secs.txt"
    log "run done in $(( $(now) - t0 )) s"
}

# ── host --list counts ─────────────────────────────────────────
do_host_list() {
    local c target path d n
    mkdir -p "$RES"
    for c in $crates; do
        [[ -f "$MANIFEST/host-$c.tsv" ]] || continue
        while IFS=$'\t' read -r _ target _ path; do
            d="$RES/${c}__${target}"
            mkdir -p "$d"
            (cd "$ROOT/crates/$c" && "$path" --list 2>/dev/null) | grep -c ': test$' >"$d/host_count.txt"
        done <"$MANIFEST/host-$c.tsv"
    done
}

# ── report ──────────────────────────────────────────────────────
do_report_phase() {
    [[ -d "$RES" ]] || die 3 "no results in $RES (run first)"
    python3 -I - "$RES" "$EXCLUDE_FILE" "$check" "$crates" "$ROOT" "$MANIFEST" "$fixtures" <<'PY'
import json
import os
import re
import sys

res_dir, excl_file, check, crates, root, manifest, fixtures = sys.argv[1:8]
check = check == "1"
fixtures = fixtures == "1"
crates = crates.split()

CATEGORIES = ("host-fixture", "android-difference")


def read(path):
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            return f.read()
    except OSError:
        return None


# ── exclusion file ──────────────────────────────────────────────
excl = []  # dicts
bad = []
text = read(excl_file)
if text is None:
    bad.append("%s is missing" % excl_file)
else:
    for n, line in enumerate(text.splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        f = line.split("\t")
        if len(f) != 5 or not all(x.strip() for x in f):
            bad.append("%s:%d: expected 5 tab separated non empty fields (crate, target, test, category, reason)" % (excl_file, n))
            continue
        if f[3] not in CATEGORIES:
            bad.append("%s:%d: unknown category %r (host-fixture or android-difference)" % (excl_file, n, f[3]))
            continue
        excl.append({"crate": f[0], "target": f[1], "test": f[2], "category": f[3], "reason": f[4], "line": n, "hit": []})
if fixtures:
    # the host-fixture entries describe the default mode (no fixtures on the device)
    excl = [e for e in excl if e["category"] != "host-fixture"]
if bad:
    for b in bad:
        print("device_engine_tests.sh: " + b, file=sys.stderr)
    sys.exit(2)

# ── binaries, in manifest order ─────────────────────────────────
binaries = []
for c in crates:
    m = read(os.path.join(manifest, "device-%s.tsv" % c))
    if m is None:
        print("device_engine_tests.sh: no manifest for %s" % c, file=sys.stderr)
        sys.exit(3)
    for line in m.splitlines():
        f = line.split("\t")
        if len(f) == 4:
            binaries.append((f[0], f[1], f[2]))

RESULT = re.compile(r"^test (.+?) \.\.\. (ok|FAILED|ignored(?:, .*)?|bench:.*)$")
SUMMARY = re.compile(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored")
HEADER = re.compile(r"^---- (.+) stdout ----$")
EXIT = re.compile(r"^EXIT=(\d+)\s*$")


def norm(name):
    return re.sub(r" - should panic$", "", name)


def list_tests(txt):
    if txt is None:
        return None
    out = []
    for line in txt.splitlines():
        if line.endswith(": test"):
            out.append(line[: -len(": test")])
    return out


def failure_sections(txt):
    """test name -> failure text of the '---- name stdout ----' sections."""
    sec = {}
    cur = None
    for line in txt.splitlines():
        h = HEADER.match(line)
        if h:
            cur = norm(h.group(1))
            sec[cur] = []
            continue
        if cur is not None and (line == "failures:" or line.startswith("test result:") or line.startswith("EXIT=")):
            cur = None
            continue
        if cur is not None:
            sec[cur].append(line)
    return {k: "\n".join(v).strip("\n") for k, v in sec.items()}


def first_host_path(txt):
    m = re.search(r"(/Users/\S+|/home/\S+|/private/\S+)", txt)
    if not m:
        return None
    p = m.group(1).rstrip("'\",:)")
    return p.replace(root, "<repo>")


rows = []
problems = []  # (binary key, message)
totals = dict(passed=0, failed=0, excluded=0, ignored=0, flaky=0)

for crate, target, kind in binaries:
    key = "%s/%s" % (crate, target)
    d = os.path.join(res_dir, "%s__%s" % (crate, target))
    run = read(os.path.join(d, "run.txt"))
    listed = list_tests(read(os.path.join(d, "list.txt")))
    listed_ign = list_tests(read(os.path.join(d, "list_ignored.txt"))) or []
    rer = read(os.path.join(d, "rerun.txt")) or ""
    host = read(os.path.join(d, "host_count.txt"))
    sizes = (read(os.path.join(d, "sizes.txt")) or "0 0").split()
    row = dict(crate=crate, target=target, kind=kind, key=key, listed=None, passed=0, failed=0,
               excluded=0, ignored=0, flaky=0, ok=True, issues=[], secs=0, exit=None,
               host=int(host) if host and host.strip().isdigit() else None,
               size_unstripped=int(sizes[0]), size_stripped=int(sizes[1]),
               failures=[], flaky_tests=[], excluded_tests=[], stale=[])
    if run is None or listed is None:
        row["ok"] = False
        row["issues"].append("did not run (no raw result in %s)" % d)
        rows.append(row)
        continue
    row["listed"] = len(listed)
    mexit = [EXIT.match(l) for l in run.splitlines()]
    mexit = [m for m in mexit if m]
    row["exit"] = int(mexit[-1].group(1)) if mexit else None
    m = re.search(r"^SECS=(\d+)", run, re.M)
    row["secs"] = int(m.group(1)) if m else 0
    summ = [SUMMARY.match(l) for l in run.splitlines()]
    summ = [s for s in summ if s]
    if not summ:
        row["ok"] = False
        why = "timeout after 1800 s" if row["exit"] == 124 else "crash or no 'test result' line (exit %s)" % row["exit"]
        row["issues"].append("did not run to its end: " + why)
    elif row["exit"] not in (0, 101):
        row["ok"] = False
        row["issues"].append("unexpected exit code %s" % row["exit"])

    first = {}
    ignored = set()
    for line in run.splitlines():
        r = RESULT.match(line)
        if not r:
            continue
        name, st = norm(r.group(1)), r.group(2)
        if st == "ok":
            first[name] = "ok"
        elif st == "FAILED":
            first[name] = "FAILED"
        elif st.startswith("ignored"):
            ignored.add(name)

    # reruns
    rerun = {}
    parts = re.split(r"^##### RERUN (.*)$", rer, flags=re.M)
    for i in range(1, len(parts), 2):
        name, body = norm(parts[i]), parts[i + 1]
        ok = re.search(r"^test .* \.\.\. ok$", body, re.M) is not None
        rerun[name] = (ok, body)

    passed = set()
    failed = {}
    flaky = []
    for name, st in first.items():
        if st == "ok":
            passed.add(name)
        else:
            ok, body = rerun.get(name, (False, ""))
            if ok:
                passed.add(name)
                flaky.append(name)
            else:
                secs = failure_sections(body)
                txt = secs.get(name)
                if txt is None:
                    secs = failure_sections(run)
                    txt = secs.get(name, "")
                if not txt:
                    txt = body.strip() or "(no failure text)"
                failed[name] = txt
    missing = [t for t in listed if t not in first and t not in ignored]
    if missing and summ:
        row["issues"].append("%d listed tests have no result line, e.g. %s" % (len(missing), missing[0]))
        row["ok"] = False
    if len(listed_ign) != len(ignored) and summ:
        row["issues"].append("ignored: --list --ignored gives %d, the run %d" % (len(listed_ign), len(ignored)))
        row["ok"] = False
    if summ:
        s = summ[-1]
        if int(s.group(2)) != len([n for n, v in first.items() if v == "ok"]) or int(s.group(3)) != len([n for n, v in first.items() if v == "FAILED"]):
            row["issues"].append("'test result' line (%s passed, %s failed) disagrees with the result lines" % (s.group(2), s.group(3)))
            row["ok"] = False

    # exclusion matching
    mine = [e for e in excl if e["crate"] == crate and e["target"] == target]
    star = [e for e in mine if e["test"] == "*"]
    excluded_names = {}
    for name, txt in failed.items():
        hit = None
        for e in mine:
            if e["test"] == name or e["test"] == "*":
                hit = e
                break
        if hit:
            excluded_names[name] = hit
            hit["hit"].append(name)
        else:
            row["ok"] = False
            hp = first_host_path(txt)
            row["failures"].append(dict(test=name, text=txt, host_path=hp))
    for e in mine:
        if e["test"] != "*":
            if e["test"] not in listed:
                row["ok"] = False
                row["stale"].append("%s: no such test (device_tests_exclude.txt line %d)" % (e["test"], e["line"]))
            elif e["test"] in passed:
                row["ok"] = False
                row["stale"].append("%s: passes on the device (device_tests_exclude.txt line %d)" % (e["test"], e["line"]))
            elif e["test"] in ignored:
                row["ok"] = False
                row["stale"].append("%s: is ignored (device_tests_exclude.txt line %d)" % (e["test"], e["line"]))
    for e in star:
        runnable = [t for t in listed if t not in ignored]
        ok_ones = sorted(t for t in runnable if t in passed)
        if ok_ones:
            row["ok"] = False
            row["stale"].append("* entry (line %d) but %d tests pass, e.g. %s" % (e["line"], len(ok_ones), ok_ones[0]))
        if not e["hit"]:
            row["ok"] = False
            row["stale"].append("* entry (line %d) matches no failing test" % e["line"])

    row["passed"] = len(passed)
    row["failed"] = len(failed)
    row["excluded"] = len(excluded_names)
    row["ignored"] = len(ignored)
    row["flaky"] = len(flaky)
    row["flaky_tests"] = sorted(flaky)
    row["excluded_tests"] = [dict(test=n, category=e["category"], reason=e["reason"], text=failed[n]) for n, e in sorted(excluded_names.items())]
    if row["passed"] + row["failed"] + row["ignored"] != row["listed"] and summ:
        row["ok"] = False
        row["issues"].append("passed %d + failed %d + ignored %d != --list count %d" % (row["passed"], row["failed"], row["ignored"], row["listed"]))
    rows.append(row)

# entries that match no binary of this run are checked only when their crate was run
for e in excl:
    if e["crate"] in crates and not any(r["crate"] == e["crate"] and r["target"] == e["target"] for r in rows):
        problems.append("exclusion line %d: %s/%s is not a test binary of this run" % (e["line"], e["crate"], e["target"]))

all_ok = not problems
for r in rows:
    all_ok = all_ok and r["ok"]
    for k in ("passed", "failed", "excluded", "ignored", "flaky"):
        totals[k] += r[k]
    print("DEVICE | %s | %s | passed %d, failed %d, excluded %d, ignored %d, flaky %d" % (
        r["key"], "PASS" if r["ok"] else "FAIL", r["passed"], r["failed"], r["excluded"], r["ignored"], r["flaky"]))
for r in rows:
    for t in r["excluded_tests"]:
        if t["category"] == "android-difference":
            print("DEVICE | WARN | android-difference | %s::%s | %s" % (r["key"], t["test"], t["reason"]))
    for t in r["flaky_tests"]:
        print("DEVICE | WARN | flaky | %s::%s" % (r["key"], t))
    if r["host"] is not None and r["listed"] is not None and r["host"] != r["listed"]:
        print("DEVICE | WARN | host-count | %s | device %d, host %d" % (r["key"], r["listed"], r["host"]))
print("DEVICE | total | %s | %d binaries, passed %d, failed %d, excluded %d, ignored %d, flaky %d" % (
    "PASS" if all_ok else "FAIL", len(rows), totals["passed"], totals["failed"], totals["excluded"], totals["ignored"], totals["flaky"]))

# details on stderr
for r in rows:
    for i in r["issues"]:
        print("DEVICE detail | %s | %s" % (r["key"], i), file=sys.stderr)
    for s in r["stale"]:
        print("DEVICE detail | %s | stale exclusion | %s" % (r["key"], s), file=sys.stderr)
    for f in r["failures"]:
        print("DEVICE detail | %s | unexcluded failure | %s" % (r["key"], f["test"]), file=sys.stderr)
        print("\n".join("    " + l for l in f["text"].splitlines()[:12]), file=sys.stderr)
        cat = "host-fixture" if f["host_path"] else "android-difference"
        reason = ("reads %s of the host checkout" % f["host_path"]) if f["host_path"] else "FILL IN"
        print("    candidate: %s\t%s\t%s\t%s\t%s" % (r["crate"], r["target"], f["test"], cat, reason), file=sys.stderr)
for p in problems:
    print("DEVICE detail | " + p, file=sys.stderr)

with open(os.path.join(res_dir, "report.json"), "w") as f:
    json.dump(dict(rows=rows, totals=totals, ok=all_ok, problems=problems), f, indent=1)

sys.exit(1 if (check and not all_ok) else 0)
PY
}

# ── main ────────────────────────────────────────────────────────
if [[ $fixtures -eq 1 ]]; then
    OUT="$ROOT/target/android-device-tests-fx"
    HOST_TARGET_DIR="$ROOT/target/android-device-tests-fx-host"
fi
MANIFEST="$OUT/manifest"
RES="$OUT/results"
STAGE="$OUT/staging"
setup_env
for c in $crates; do
    [[ -d "$ROOT/crates/$c" ]] || die 2 "unknown crate $c (crates/$c does not exist)"
done
if [[ $do_build -eq 1 ]]; then
    do_build_phase
fi
[[ $do_run -eq 0 && $do_report -eq 0 ]] && exit 0
if [[ $do_run -eq 1 ]]; then
    do_run_phase
    trap - EXIT
    device_cleanup
fi
if [[ $host_list -eq 1 ]]; then
    do_host_list
fi
do_report_phase
rc=$?
if [[ $keep -eq 0 && $host_list -eq 1 ]]; then
    rm -rf "$HOST_TARGET_DIR"
fi
exit $rc
