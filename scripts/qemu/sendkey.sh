#!/usr/bin/env bash
# pili-pos - press a key in a running scripts/qemu/run-kiosk.sh guest,
# through its QMP socket (QEMU `send-key`, qcode names: ret, esc, a...).
#
# Usage: scripts/qemu/sendkey.sh <name> <qcode>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
NAME="${1:?usage: $0 <name> <qcode>}"
KEY="${2:?usage: $0 <name> <qcode>}"
QMP_SOCK="$SCRIPT_DIR/.run/$NAME/qmp.sock"
[[ -S "$QMP_SOCK" ]] || { echo "ERROR: no QMP socket at $QMP_SOCK" >&2; exit 1; }

python3 - "$QMP_SOCK" "$KEY" <<'PY'
import json
import socket
import sys

sock_path, key = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(15)
s.connect(sock_path)
f = s.makefile("rwb")


def call(cmd):
    f.write((json.dumps(cmd) + "\n").encode())
    f.flush()
    while True:
        msg = json.loads(f.readline())
        if "return" in msg or "error" in msg:
            return msg


json.loads(f.readline())  # greeting
call({"execute": "qmp_capabilities"})
resp = call({"execute": "send-key",
             "arguments": {"keys": [{"type": "qcode", "data": key}]}})
if "error" in resp:
    sys.exit(f"send-key failed: {resp['error']}")
PY
