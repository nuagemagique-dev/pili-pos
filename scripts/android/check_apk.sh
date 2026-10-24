#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - check an Android APK (PD79)
#
# Usage: scripts/android/check_apk.sh APK
#
# Prints one line per check:
#     APK | <check> | PASS|FAIL|INFO | <detail>
# and exits 0 when every check passes, 1 when one fails, 2 on a usage error, a
# missing file or a missing tool. All checks run even after a failure.
#
# Checks
#   zipalign   zipalign -c -P 16 -v 4 APK (build-tools 35.0.0)
#   abis       the only native directory is lib/arm64-v8a/
#   libs       lib/arm64-v8a/libpos_ffi.so and lib/arm64-v8a/libjnidispatch.so exist
#   stored     every lib/**/*.so entry uses compression method 0 (stored)
#   so-align   every LOAD segment of every lib/arm64-v8a/*.so has alignment 0x4000
#   so-stripped  libpos_ffi.so has no .debug_* section and is under 20 MiB
# then INFO lines: sha1 and sha256 of the APK, sha1 and size of libpos_ffi.so.
#
# Tools: $ANDROID_HOME/build-tools/35.0.0/zipalign, llvm-readelf from
# $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/*/bin, python3 (zipfile).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
fi
if [[ $# -ne 1 ]]; then
    echo "usage: check_apk.sh APK" >&2
    exit 2
fi
APK="$1"
[[ -f "$APK" ]] || { echo "check_apk: no such file: $APK" >&2; exit 2; }

ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
NDK="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
ZIPALIGN="$ANDROID_HOME/build-tools/35.0.0/zipalign"
READELF=""
for r in "$NDK"/toolchains/llvm/prebuilt/*/bin/llvm-readelf; do
    [[ -x "$r" ]] && { READELF="$r"; break; }
done
[[ -x "$ZIPALIGN" ]] || { echo "check_apk: zipalign not found: $ZIPALIGN" >&2; exit 2; }
[[ -n "$READELF" ]] || { echo "check_apk: llvm-readelf not found under $NDK" >&2; exit 2; }
command -v python3 >/dev/null || { echo "check_apk: python3 not found" >&2; exit 2; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
failed=0

clean() { tr '\n|' ' /' | sed 's/  */ /g; s/^ //; s/ $//' | cut -c1-200; }
report() { # <check> <PASS|FAIL|INFO> <detail>
    printf 'APK | %s | %s | %s\n' "$1" "$2" "$3"
    [[ "$2" != "FAIL" ]] || failed=$((failed + 1))
}

# zipalign: 16 KB page alignment of the stored .so entries.
if out="$("$ZIPALIGN" -c -P 16 -v 4 "$APK" 2>&1)"; then
    report zipalign PASS "$(printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | tail -n 1 | clean)"
else
    report zipalign FAIL "$(printf '%s\n' "$out" | grep -iE 'BAD|fail|error' | head -n 2 | clean)"
fi

# Zip structure checks and extraction of lib/ with python3.
LIBDIR="$TMP/lib"
mkdir -p "$LIBDIR"
if ! py_out="$(python3 -I - "$APK" "$LIBDIR" <<'PY'
import hashlib
import sys
import zipfile

apk, libdir = sys.argv[1], sys.argv[2]
if not zipfile.is_zipfile(apk):
    print("zip|FAIL|not a zip file")
    sys.exit(0)
z = zipfile.ZipFile(apk)
names = [i.filename for i in z.infolist() if not i.is_dir()]
native = sorted({n.split("/")[1] for n in names if n.startswith("lib/") and n.count("/") >= 2})
if native == ["arm64-v8a"]:
    print("abis|PASS|lib/arm64-v8a/ only")
else:
    print("abis|FAIL|native directories: " + (", ".join("lib/" + d + "/" for d in native) or "none"))
need = ["lib/arm64-v8a/libpos_ffi.so", "lib/arm64-v8a/libjnidispatch.so"]
missing = [n for n in need if n not in names]
if missing:
    print("libs|FAIL|missing " + ", ".join(missing))
else:
    print("libs|PASS|libpos_ffi.so and libjnidispatch.so present")
sos = [i for i in z.infolist() if i.filename.startswith("lib/") and i.filename.endswith(".so")]
deflated = [i.filename for i in sos if i.compress_type != 0]
if not sos:
    print("stored|FAIL|no lib/**/*.so entry")
elif deflated:
    print("stored|FAIL|compressed: " + ", ".join(deflated))
else:
    print("stored|PASS|%d .so entries, all stored" % len(sos))
for i in sos:
    if i.filename.startswith("lib/arm64-v8a/") and i.filename.count("/") == 2:
        with open(libdir + "/" + i.filename.split("/")[-1], "wb") as f:
            f.write(z.read(i))
apk_bytes = open(apk, "rb").read()
print("sha1|INFO|" + hashlib.sha1(apk_bytes).hexdigest())
print("sha256|INFO|" + hashlib.sha256(apk_bytes).hexdigest())
if "lib/arm64-v8a/libpos_ffi.so" in names:
    d = z.read("lib/arm64-v8a/libpos_ffi.so")
    print("libpos_ffi.so|INFO|sha1 %s, %d bytes" % (hashlib.sha1(d).hexdigest(), len(d)))
PY
)"; then
    report zip FAIL "python3 failed to read the APK"
    py_out=""
fi

info_lines=""
while IFS='|' read -r chk res detail; do
    [[ -n "$chk" ]] || continue
    if [[ "$res" == "INFO" ]]; then
        info_lines+="$chk|$detail"$'\n'
    else
        report "$chk" "$res" "$detail"
    fi
done <<<"$py_out"

# so-align: every LOAD segment of every extracted lib/arm64-v8a/*.so is aligned to 0x4000.
shopt -s nullglob
sofiles=("$LIBDIR"/*.so)
shopt -u nullglob
if [[ ${#sofiles[@]} -eq 0 ]]; then
    report so-align FAIL "no lib/arm64-v8a/*.so entry"
else
    bad=""
    nload=0
    for so in "${sofiles[@]}"; do
        if ! rl="$("$READELF" -lW "$so" 2>&1)"; then
            bad+="$(basename "$so"): llvm-readelf failed; "
            continue
        fi
        while read -r a; do
            nload=$((nload + 1))
            [[ $((a)) -eq 16384 ]] || bad+="$(basename "$so") align $a; "
        done < <(printf '%s\n' "$rl" | awk '$1 == "LOAD" { print $NF }')
    done
    if [[ -n "$bad" ]]; then
        report so-align FAIL "not 0x4000: $bad"
    elif [[ $nload -eq 0 ]]; then
        report so-align FAIL "no LOAD segment found"
    else
        report so-align PASS "${#sofiles[@]} libs, $nload LOAD segments, all 0x4000"
    fi
fi

# so-stripped: libpos_ffi.so carries no .debug_* section and is under 20 MiB.
FFI="$LIBDIR/libpos_ffi.so"
if [[ ! -f "$FFI" ]]; then
    report so-stripped FAIL "no lib/arm64-v8a/libpos_ffi.so"
else
    size="$(wc -c <"$FFI" | tr -d ' ')"
    if ! secs="$("$READELF" -SW "$FFI" 2>&1)"; then
        report so-stripped FAIL "llvm-readelf failed"
    elif printf '%s\n' "$secs" | grep -Eq '\] +\.debug_'; then
        report so-stripped FAIL ".debug_ sections: $(printf '%s\n' "$secs" | grep -oE '\.debug_[a-z_]+' | sort -u | tr '\n' ' ' | clean)"
    elif [[ "$size" -ge 20971520 ]]; then
        report so-stripped FAIL "$size bytes, 20 MiB or more"
    else
        report so-stripped PASS "no .debug_ section, $size bytes"
    fi
fi

while IFS='|' read -r chk detail; do
    [[ -n "$chk" ]] || continue
    report "$chk" INFO "$detail"
done <<<"$info_lines"

[[ "$failed" -eq 0 ]] || exit 1
exit 0
