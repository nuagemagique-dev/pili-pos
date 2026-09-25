#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos — capture a screenshot from a running scripts/qemu/run-kiosk.sh
# instance via its QMP socket, for
# docs/SUBMISSION_ARTIFACTS/CAPTURES_ECRAN_RUNBOOK_FR.md.
#
# Speaks QMP directly (qmp_capabilities handshake, then the `screendump`
# command) over the unix socket run-kiosk.sh creates at
# scripts/qemu/.run/<name>/qmp.sock. Tries format=png first; QEMU
# versions before ~7.1 only support the raw PPM dump, so on any error
# this falls back to a PPM capture converted to PNG with `sips` (macOS)
# or `magick`/`convert` (ImageMagick, Linux/CI).
#
# Usage: screendump.sh <name> [output-basename]
#   <name>            matches run-kiosk.sh's --name (or the image's
#                      basename if --name was not given)
#   [output-basename]  defaults to <name>; written as
#                      docs/SUBMISSION_ARTIFACTS/screenshots/<output-basename>.png
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SHOTS_DIR="$REPO_ROOT/docs/SUBMISSION_ARTIFACTS/screenshots"

NAME="${1:-}"
OUT_BASENAME="${2:-$NAME}"
if [[ -z "$NAME" ]]; then
    echo "Usage: $0 <name> [output-basename]" >&2
    exit 1
fi

QMP_SOCK="$SCRIPT_DIR/.run/$NAME/qmp.sock"
if [[ ! -S "$QMP_SOCK" ]]; then
    echo "ERROR: no QMP socket at $QMP_SOCK — is 'run-kiosk.sh --name $NAME' running?" >&2
    exit 1
fi

mkdir -p "$SHOTS_DIR"
OUT_PNG="$SHOTS_DIR/${OUT_BASENAME}.png"

command -v python3 >/dev/null 2>&1 || { echo "ERROR: python3 not found (needed to speak QMP)" >&2; exit 1; }

# qmp_dump.py <socket> <target-file> <format: png|ppm>
# Prints "OK" on success, "ERROR: <message>" and exits 1 otherwise.
qmp_dump() {
    local sock="$1" target="$2" fmt="$3"
    python3 - "$sock" "$target" "$fmt" <<'PY'
import json
import socket
import sys

sock_path, target, fmt = sys.argv[1], sys.argv[2], sys.argv[3]

s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(15)
s.connect(sock_path)
f = s.makefile("rwb")


def read_json():
    line = f.readline()
    if not line:
        raise RuntimeError("QMP socket closed unexpectedly")
    return json.loads(line)


def send(cmd):
    f.write((json.dumps(cmd) + "\n").encode())
    f.flush()


read_json()  # greeting
send({"execute": "qmp_capabilities"})
resp = read_json()
if "error" in resp:
    print(f"ERROR: qmp_capabilities: {resp['error']}")
    sys.exit(1)

args = {"filename": target}
if fmt == "png":
    args["format"] = "png"
send({"execute": "screendump", "arguments": args})
resp = read_json()
if "error" in resp:
    print(f"ERROR: screendump: {resp['error']}")
    sys.exit(1)

print("OK")
PY
}

RESULT="$(qmp_dump "$QMP_SOCK" "$OUT_PNG" png || true)"
if [[ "$RESULT" == "OK" && -s "$OUT_PNG" ]]; then
    echo "✓ $OUT_PNG (native PNG screendump)"
    exit 0
fi
echo "▸ PNG screendump unavailable (${RESULT:-no response}) — falling back to PPM…"

PPM_TMP="$(mktemp -t moyun-screendump-XXXXXX).ppm"
RESULT="$(qmp_dump "$QMP_SOCK" "$PPM_TMP" ppm || true)"
if [[ "$RESULT" != "OK" || ! -s "$PPM_TMP" ]]; then
    echo "ERROR: screendump failed: ${RESULT:-no response}" >&2
    rm -f "$PPM_TMP"
    exit 1
fi

if command -v sips >/dev/null 2>&1; then
    sips -s format png "$PPM_TMP" --out "$OUT_PNG" >/dev/null
elif command -v magick >/dev/null 2>&1; then
    magick "$PPM_TMP" "$OUT_PNG"
elif command -v convert >/dev/null 2>&1; then
    convert "$PPM_TMP" "$OUT_PNG"
else
    echo "ERROR: no PPM->PNG converter found (need 'sips' on macOS or" >&2
    echo "       ImageMagick's 'magick'/'convert' on Linux)" >&2
    rm -f "$PPM_TMP"
    exit 1
fi
rm -f "$PPM_TMP"
echo "✓ $OUT_PNG (PPM -> PNG fallback)"
