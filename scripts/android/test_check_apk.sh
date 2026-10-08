#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/check_apk.sh
#
# Usage: bash scripts/android/test_check_apk.sh
#
# Builds APK-shaped zip fixtures with python3 in a mktemp dir (no emulator, no
# network). The shared objects of the passing fixture are compiled with the clang of
# the NDK (aarch64-linux-android33-clang, -Wl,-z,max-page-size=16384, stripped), so
# the PASS case has real LOAD segments aligned to 0x4000 and goes through zipalign
# -P 16 -f 4. The system libraries of the NDK sysroot are not used: their LOAD
# alignment is the NDK default (4 KB), not 16 KB. Without that clang the cases that
# need a real ELF print SKIP (and do not fail the test).
# Prints one line per case and a summary; exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
CHECK="$ROOT/scripts/android/check_apk.sh"
ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
NDK="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
ZIPALIGN="$ANDROID_HOME/build-tools/35.0.0/zipalign"
CC=""
for c in "$NDK"/toolchains/llvm/prebuilt/*/bin/aarch64-linux-android33-clang; do
    [[ -x "$c" ]] && { CC="$c"; break; }
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
pass=0
fail=0
skipped=0

ok() { pass=$((pass + 1)); printf 'TEST | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TEST | %s | FAIL | %s\n' "$1" "$2"; }
skip() { skipped=$((skipped + 1)); printf 'TEST | %s | SKIP | %s\n' "$1" "$2"; }

# mkzip OUT NAME:SRC:METHOD ... (SRC "-" = a few bytes; METHOD stored|deflated)
mkzip() {
    local out="$1"
    shift
    python3 -I - "$out" "$@" <<'PY'
import sys
import zipfile

out = sys.argv[1]
with zipfile.ZipFile(out, "w") as z:
    for spec in sys.argv[2:]:
        name, src, method = spec.split(":")
        data = b"x" * 64 if src == "-" else open(src, "rb").read()
        z.writestr(
            zipfile.ZipInfo(name),
            data,
            compress_type=zipfile.ZIP_STORED if method == "stored" else zipfile.ZIP_DEFLATED,
        )
PY
}

# expect_fail NAME APK CHECK: check_apk.sh exits 1 and the CHECK line is FAIL.
expect_fail() {
    local name="$1" apk="$2" chk="$3" out rc
    out="$("$CHECK" "$apk" 2>&1)"
    rc=$?
    if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -q "^APK | $chk | FAIL | "; then
        ok "$name"
    else
        bad "$name" "rc=$rc, want 1 with '$chk' FAIL: $(printf '%s' "$out" | tr '\n' ';' | cut -c1-300)"
    fi
}

# usage errors
"$CHECK" >/dev/null 2>&1
rc=$?
[[ $rc -eq 2 ]] && ok "no argument exits 2" || bad "no argument exits 2" "rc=$rc"
"$CHECK" "$TMP/missing.apk" >/dev/null 2>&1
rc=$?
[[ $rc -eq 2 ]] && ok "missing file exits 2" || bad "missing file exits 2" "rc=$rc"
"$CHECK" a b >/dev/null 2>&1
rc=$?
[[ $rc -eq 2 ]] && ok "two arguments exit 2" || bad "two arguments exit 2" "rc=$rc"

# structure cases (no real ELF needed: the abis, libs and stored lines are independent)
mkzip "$TMP/v7a.apk" "lib/armeabi-v7a/x.so:-:stored" "classes.dex:-:deflated"
expect_fail "lib/armeabi-v7a/ fails abis" "$TMP/v7a.apk" abis
mkzip "$TMP/nolib.apk" "lib/arm64-v8a/libjnidispatch.so:-:stored" "classes.dex:-:deflated"
expect_fail "no libpos_ffi.so fails libs" "$TMP/nolib.apk" libs
mkzip "$TMP/deflated.apk" "lib/arm64-v8a/libpos_ffi.so:-:deflated" "lib/arm64-v8a/libjnidispatch.so:-:stored"
expect_fail "deflated .so fails stored" "$TMP/deflated.apk" stored
printf 'not a zip' >"$TMP/garbage.apk"
expect_fail "garbage file fails zip" "$TMP/garbage.apk" zip

# cases with real ELF files
if [[ -z "$CC" ]]; then
    for n in "aligned, stripped APK passes" "extra lib/x86_64/ fails abis" "4 KB aligned .so fails so-align" \
        "unstripped libpos_ffi.so fails so-stripped"; do
        skip "$n" "no aarch64-linux-android33-clang in the NDK"
    done
else
    mkdir -p "$TMP/elf"
    printf 'int pos_probe(void) { return 7; }\n' >"$TMP/elf/p.c"
    "$CC" -shared -fPIC -s -Wl,-z,max-page-size=16384 -o "$TMP/elf/good.so" "$TMP/elf/p.c" 2>"$TMP/elf/cc.log" &&
        "$CC" -shared -fPIC -s -Wl,-z,max-page-size=4096 -o "$TMP/elf/small.so" "$TMP/elf/p.c" 2>>"$TMP/elf/cc.log" &&
        "$CC" -shared -fPIC -g -Wl,-z,max-page-size=16384 -o "$TMP/elf/debug.so" "$TMP/elf/p.c" 2>>"$TMP/elf/cc.log"
    if [[ ! -f "$TMP/elf/debug.so" ]]; then
        bad "build ELF fixtures" "$(tail -n 3 "$TMP/elf/cc.log" | tr '\n' ';')"
    else
        align() { "$ZIPALIGN" -P 16 -f 4 "$1" "$2" >/dev/null; }
        mkzip "$TMP/raw_good.apk" "lib/arm64-v8a/libpos_ffi.so:$TMP/elf/good.so:stored" \
            "lib/arm64-v8a/libjnidispatch.so:$TMP/elf/good.so:stored" "classes.dex:-:deflated"
        align "$TMP/raw_good.apk" "$TMP/good.apk"
        out="$("$CHECK" "$TMP/good.apk" 2>&1)"
        rc=$?
        npass="$(printf '%s\n' "$out" | grep -c '^APK | [a-z.-]* | PASS | ' || true)"
        ninfo="$(printf '%s\n' "$out" | grep -c '^APK | [A-Za-z0-9._-]* | INFO | ' || true)"
        if [[ $rc -eq 0 && "$npass" -eq 6 && "$ninfo" -eq 3 ]] &&
            printf '%s\n' "$out" | grep -q '^APK | sha1 | INFO | ' && printf '%s\n' "$out" | grep -q '^APK | sha256 | INFO | ' &&
            printf '%s\n' "$out" | grep -q '^APK | libpos_ffi.so | INFO | sha1 [0-9a-f]\{40\}, [0-9]* bytes'; then
            ok "aligned, stripped APK passes (6 PASS, 3 INFO)"
        else
            bad "aligned, stripped APK passes" "rc=$rc pass=$npass info=$ninfo: $(printf '%s' "$out" | tr '\n' ';' | cut -c1-400)"
        fi

        mkzip "$TMP/unaligned.apk" "lib/arm64-v8a/libpos_ffi.so:$TMP/elf/good.so:stored" \
            "lib/arm64-v8a/libjnidispatch.so:$TMP/elf/good.so:stored" "a.txt:-:stored"
        # python stores the entries back to back, so the .so data is not 16 KB aligned
        if "$ZIPALIGN" -c -P 16 4 "$TMP/unaligned.apk" >/dev/null 2>&1; then
            skip "unaligned zip fails zipalign" "fixture happens to be aligned"
        else
            expect_fail "unaligned zip fails zipalign" "$TMP/unaligned.apk" zipalign
        fi

        mkzip "$TMP/raw_x86.apk" "lib/arm64-v8a/libpos_ffi.so:$TMP/elf/good.so:stored" \
            "lib/arm64-v8a/libjnidispatch.so:$TMP/elf/good.so:stored" "lib/x86_64/libpos_ffi.so:$TMP/elf/good.so:stored"
        align "$TMP/raw_x86.apk" "$TMP/x86.apk"
        expect_fail "extra lib/x86_64/ fails abis" "$TMP/x86.apk" abis

        mkzip "$TMP/raw_small.apk" "lib/arm64-v8a/libpos_ffi.so:$TMP/elf/good.so:stored" \
            "lib/arm64-v8a/libjnidispatch.so:$TMP/elf/small.so:stored"
        align "$TMP/raw_small.apk" "$TMP/small.apk"
        expect_fail "4 KB aligned .so fails so-align" "$TMP/small.apk" so-align

        mkzip "$TMP/raw_debug.apk" "lib/arm64-v8a/libpos_ffi.so:$TMP/elf/debug.so:stored" \
            "lib/arm64-v8a/libjnidispatch.so:$TMP/elf/good.so:stored"
        align "$TMP/raw_debug.apk" "$TMP/debug.apk"
        expect_fail "unstripped libpos_ffi.so fails so-stripped" "$TMP/debug.apk" so-stripped
    fi
fi

printf 'TEST | summary | %s | %d passed, %d failed, %d skipped\n' "$([[ $fail -eq 0 ]] && echo PASS || echo FAIL)" "$pass" "$fail" "$skipped"
[[ $fail -eq 0 ]]
