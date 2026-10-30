#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - make a DEVELOPMENT provisioning bundle (PD108)
#
# Usage: scripts/android/make_dev_bundle.sh [--out DIR] [--keydir DIR] [--fdm-url URL]
#                                           [--printer HOST:PORT] [--seed | --seed-file F] [--write-pub]
#   --out DIR       bundle directory (default $HOME/.cache/pili-android/dev-bundle). Replaced when it
#                   already holds a bundle (a MANIFEST.json) or nothing; any other content is refused.
#   --keydir DIR    holds minisign.key (secret) and minisign.pub (default
#                   $HOME/.cache/pili-android/dev-minisign). Created, with a key pair without password,
#                   when minisign.key is missing.
#   --fdm-url URL   fdm_url of pos.json (default http://127.0.0.1:19380/graphql)
#   --printer H:P   printer_address of pos.json (default 127.0.0.1:19390)
#   --seed          also ship src-tauri/seed_spf_test.sql as seed.sql
#   --seed-file F   ship F as seed.sql instead (implies --seed; e.g. the demo seed)
# Environment (demo only): PILI_BUNDLE_RESTAURANT_NAME, PILI_BUNDLE_RESTAURANT_ADDRESS, PILI_BUNDLE_KITCHEN_PRINTER (adds
#   kitchen_printer_address), PILI_BUNDLE_HUBRISE_ENABLED=1 (adds "hubrise_enabled": true), PILI_BUNDLE_EXPORT_URL + _RECIPIENT + _TOKEN (adds the nightly export section and an
#   export_token file), PILI_BUNDLE_PAYMENT_TERMINAL_OPI (host:port; adds a payment_terminal
#   section of kind ccv_opi whose address is that value, a CCV terminal on the LAN; the pos-eft-mock OPI side) with
#   PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT (the till's channel 1 port, optional: 4102 when absent). Defaults keep the test identity.
#   PILI_BUNDLE_PRINTER_PAPER_MM (58 or 80; adds printer_paper_mm, the ticket printer's roll: 58 lays every document out
#   on 35 columns),
#   PILI_BUNDLE_VAT (restaurant_vat, default BE0000000097; a malformed value makes the import refuse the bundle: capture bench),
#   PILI_BUNDLE_DIGITAL_TICKET_API (base URL of the Scaleway TEM API, e.g. the pos-fdm-mock on http://127.0.0.1:19380; adds a
#   digital_ticket section with allow_digital_only and a tem_key file), PILI_BUNDLE_HUBRISE_MOCK (base URL of pos-hubrise-mock,
#   e.g. http://127.0.0.1:3001; adds a hubrise section pointing at it with a hubrise_secret file),
#   PILI_BUNDLE_PEPPOL_API (base URL of the pos-fdm-mock, e.g. http://127.0.0.1:19380; adds a peppol section of provider
#   storecove at <URL>/storecove (the mock's Storecove routes), a peppol_key file and the sender 0208:0000000097 of the
#   test identity: the Peppol panel of Settings and the delivery of invoices then work against the mock).
#   --write-pub     copy keydir/minisign.pub to android/app/provisioning/dev-minisign.pub (the committed
#                   public key); used once when the development key is (re)generated
#
# The bundle holds pos.json (the test identity of the Android till, no token: the mocks need none),
# optionally seed.sql, MANIFEST.json (format 1, files sorted by path with size and SHA-256) and
# MANIFEST.json.minisig (minisign -S, prehashed, trusted comment 'pili dev bundle'), verified with
# minisign -V before the script says PASS.
#
# The secret key never enters a git work tree: --out and --keydir inside one are refused. The public half
# must be android/app/provisioning/dev-minisign.pub (what debug builds verify with); when it differs the
# script fails and says how to regenerate. PILI_DEV_PUB overrides that path (self-test only).
#
# Prints 'DEVBUNDLE | PASS | <dir>' on success. Exit: 0 ok, 1 failure, 2 usage error.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO="$(cd "$SELF_DIR/../.." && pwd -P)"
COMMITTED_PUB="${PILI_DEV_PUB:-$REPO/android/app/provisioning/dev-minisign.pub}"
SEED_SOURCE="$REPO/src-tauri/seed_spf_test.sql"

OUT="$HOME/.cache/pili-android/dev-bundle"
KEYDIR="$HOME/.cache/pili-android/dev-minisign"
FDM_URL="http://127.0.0.1:19380/graphql"
PRINTER="127.0.0.1:19390"
SEED=0
WRITE_PUB=0

usage() {
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
}

die() {
    echo "DEVBUNDLE | FAIL | $*" >&2
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage; exit 0 ;;
        --out) [[ $# -ge 2 ]] || { usage >&2; exit 2; }; OUT="$2"; shift 2 ;;
        --keydir) [[ $# -ge 2 ]] || { usage >&2; exit 2; }; KEYDIR="$2"; shift 2 ;;
        --fdm-url) [[ $# -ge 2 ]] || { usage >&2; exit 2; }; FDM_URL="$2"; shift 2 ;;
        --printer) [[ $# -ge 2 ]] || { usage >&2; exit 2; }; PRINTER="$2"; shift 2 ;;
        --seed) SEED=1; shift ;;
        --seed-file) [[ $# -ge 2 ]] || die "--seed-file needs a path"; SEED=1; SEED_SOURCE="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
        --write-pub) WRITE_PUB=1; shift ;;
        *) echo "make_dev_bundle: unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
done

MINISIGN="$(command -v minisign || true)"
[[ -n "$MINISIGN" ]] || die "minisign not found (brew install minisign)"
command -v python3 >/dev/null 2>&1 || die "python3 not found"

# A value that ends up inside pos.json must not need JSON escaping.
for v in "$FDM_URL" "$PRINTER"; do
    [[ "$v" =~ ^[A-Za-z0-9:/._?=\&%@-]+$ ]] || die "unsafe character in '$v'"
done

# Absolute form of a path that may not exist yet (nearest existing ancestor, resolved).
abspath() {
    local p="$1" tail=""
    [[ "$p" == /* ]] || p="$PWD/$p"
    while [[ ! -d "$p" ]]; do
        tail="/$(basename "$p")$tail"
        p="$(dirname "$p")"
    done
    echo "$(cd "$p" && pwd -P)$tail"
}

inside_git() {
    local p="$1"
    while [[ ! -d "$p" ]]; do p="$(dirname "$p")"; done
    git -C "$p" rev-parse --is-inside-work-tree >/dev/null 2>&1
}

OUT="$(abspath "$OUT")"
KEYDIR="$(abspath "$KEYDIR")"
inside_git "$OUT" && die "--out $OUT lies inside a git work tree; refusing"
inside_git "$KEYDIR" && die "--keydir $KEYDIR lies inside a git work tree; refusing"

# The key pair: created once, never printed.
if [[ ! -f "$KEYDIR/minisign.key" ]]; then
    old_umask="$(umask)"
    umask 077
    mkdir -p "$KEYDIR"
    chmod 700 "$KEYDIR"
    rm -f "$KEYDIR/minisign.pub"
    "$MINISIGN" -G -W -p "$KEYDIR/minisign.pub" -s "$KEYDIR/minisign.key" >/dev/null 2>&1 \
        || die "minisign could not create the development key pair in $KEYDIR"
    chmod 600 "$KEYDIR/minisign.key"
    umask "$old_umask"
    echo "make_dev_bundle: created the development key pair in $KEYDIR" >&2
fi
[[ -f "$KEYDIR/minisign.pub" ]] || die "$KEYDIR/minisign.pub is missing (remove $KEYDIR/minisign.key to regenerate the pair)"

# The public key line of a minisign.pub file (the last non-empty line).
key_line() {
    grep -v '^untrusted comment:' "$1" | grep -v '^[[:space:]]*$' | tail -n 1 | tr -d '[:space:]'
}

if [[ "$WRITE_PUB" == "1" ]]; then
    mkdir -p "$(dirname "$COMMITTED_PUB")"
    cp "$KEYDIR/minisign.pub" "$COMMITTED_PUB"
    echo "make_dev_bundle: wrote $COMMITTED_PUB" >&2
fi
if [[ ! -f "$COMMITTED_PUB" ]]; then
    die "$COMMITTED_PUB is missing. Run once with --write-pub, then commit it (development key only)"
fi
if [[ "$(key_line "$KEYDIR/minisign.pub")" != "$(key_line "$COMMITTED_PUB")" ]]; then
    die "the public key in $KEYDIR/minisign.pub is not $COMMITTED_PUB. Debug builds verify with the committed key: use the keydir that holds its secret half, or regenerate the pair (rm $KEYDIR/minisign.key), run with --write-pub and commit the new public key"
fi

if [[ "$SEED" == "1" ]]; then
    [[ -f "$SEED_SOURCE" ]] || die "$SEED_SOURCE not found"
fi

# The output directory: empty, absent or a previous bundle.
if [[ -e "$OUT" ]]; then
    [[ -d "$OUT" ]] || die "$OUT exists and is not a directory"
    if [[ -n "$(ls -A "$OUT")" && ! -f "$OUT/MANIFEST.json" ]]; then
        die "$OUT is not empty and holds no bundle; refusing to replace it"
    fi
    rm -rf "$OUT"
fi
mkdir -p "$OUT"

KITCHEN_LINE=""
HUBRISE_LINE=""
[[ "${PILI_BUNDLE_HUBRISE_ENABLED:-0}" == 1 ]] && HUBRISE_LINE=$'\n  "hubrise_enabled": true,'
EXPORT_LINE=""
if [[ -n "${PILI_BUNDLE_EXPORT_URL:-}" ]]; then
    [[ -n "${PILI_BUNDLE_EXPORT_RECIPIENT:-}" && -n "${PILI_BUNDLE_EXPORT_TOKEN:-}" ]] ||
        die "PILI_BUNDLE_EXPORT_URL needs PILI_BUNDLE_EXPORT_RECIPIENT and PILI_BUNDLE_EXPORT_TOKEN"
    printf '%s\n' "$PILI_BUNDLE_EXPORT_TOKEN" >"$OUT/export_token"
    EXPORT_LINE=$'\n  "export": {"url": "'"$PILI_BUNDLE_EXPORT_URL"'", "token_file": "export_token", "owner_recipient": "'"$PILI_BUNDLE_EXPORT_RECIPIENT"'", "keep_days": 7},'
fi
TERMINAL_LINE=""
if [[ -n "${PILI_BUNDLE_PAYMENT_TERMINAL_OPI:-}" ]]; then
    [[ "$PILI_BUNDLE_PAYMENT_TERMINAL_OPI" =~ ^[A-Za-z0-9._-]+:[0-9]+$ ]] ||
        die "PILI_BUNDLE_PAYMENT_TERMINAL_OPI must look like 127.0.0.1:19410, got '$PILI_BUNDLE_PAYMENT_TERMINAL_OPI'"
    DEVICE_PORT_PART=""
    if [[ -n "${PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT:-}" ]]; then
        [[ "$PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT" =~ ^[0-9]{1,5}$ && "$PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT" -le 65535 ]] ||
            die "PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT must be a port, got '$PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT'"
        DEVICE_PORT_PART=', "device_port": '"$PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT"
    fi
    TERMINAL_LINE=$'\n  "payment_terminal": {"kind": "ccv_opi", "address": "'"$PILI_BUNDLE_PAYMENT_TERMINAL_OPI"'"'"$DEVICE_PORT_PART"', "timeout_seconds": 60},'
fi
DIGITAL_LINE=""
if [[ -n "${PILI_BUNDLE_DIGITAL_TICKET_API:-}" ]]; then
    [[ "$PILI_BUNDLE_DIGITAL_TICKET_API" =~ ^http://[A-Za-z0-9._-]+:[0-9]+$ ]] || die "PILI_BUNDLE_DIGITAL_TICKET_API must look like http://127.0.0.1:19380"
    printf 'dev-tem-key\n' >"$OUT/tem_key"
    DIGITAL_LINE=$'\n  "digital_ticket": {"project_id": "11111111-1111-1111-1111-111111111111", "from_email": "tickets@restaurant-test.be", "from_name": "Restaurant Test", "api_base": "'"$PILI_BUNDLE_DIGITAL_TICKET_API"'", "api_key_file": "tem_key", "allow_digital_only": true},'
fi
HUBRISE_MOCK_LINE=""
if [[ -n "${PILI_BUNDLE_HUBRISE_MOCK:-}" ]]; then
    [[ "$PILI_BUNDLE_HUBRISE_MOCK" =~ ^http://[A-Za-z0-9._-]+:[0-9]+$ ]] || die "PILI_BUNDLE_HUBRISE_MOCK must look like http://127.0.0.1:3001"
    printf 'dev-hubrise-secret\n' >"$OUT/hubrise_secret"
    HUBRISE_MOCK_LINE=$'\n  "hubrise": {"client_id": "pili-pos", "client_secret_file": "hubrise_secret", "oauth_base_url": "'"$PILI_BUNDLE_HUBRISE_MOCK"'/oauth2/v1", "api_base_url": "'"$PILI_BUNDLE_HUBRISE_MOCK"'/v1"},'
fi
PEPPOL_LINE=""
if [[ -n "${PILI_BUNDLE_PEPPOL_API:-}" ]]; then
    [[ "$PILI_BUNDLE_PEPPOL_API" =~ ^http://[A-Za-z0-9._-]+:[0-9]+$ ]] || die "PILI_BUNDLE_PEPPOL_API must look like http://127.0.0.1:19380"
    printf 'dev-peppol-key\n' >"$OUT/peppol_key"
    PEPPOL_LINE=$'\n  "peppol": {"provider": "storecove", "base_url": "'"$PILI_BUNDLE_PEPPOL_API"'/storecove", "api_key_file": "peppol_key", "legal_entity_id": 1, "sender_id": "0208:0000000097", "supplier_street": "1 Rue du Test", "supplier_postal_code": "4000", "supplier_city": "Liege"},'
fi
[[ -n "${PILI_BUNDLE_KITCHEN_PRINTER:-}" ]] && KITCHEN_LINE=$'\n  "kitchen_printer_address": "'"$PILI_BUNDLE_KITCHEN_PRINTER"'",'
PAPER_LINE=""
if [[ -n "${PILI_BUNDLE_PRINTER_PAPER_MM:-}" ]]; then
    [[ "$PILI_BUNDLE_PRINTER_PAPER_MM" =~ ^(58|80)$ ]] || die "PILI_BUNDLE_PRINTER_PAPER_MM must be 58 or 80"
    PAPER_LINE=$'\n  "printer_paper_mm": '"$PILI_BUNDLE_PRINTER_PAPER_MM"','
fi
cat >"$OUT/pos.json" <<JSON
{
  "pos_id": "CPOS0031234567",
  "terminal_id": "POS-2-ANDROID",
  "device_id": "pos-json-device",
  "restaurant_name": "${PILI_BUNDLE_RESTAURANT_NAME:-Restaurant Test}",
  "restaurant_address": "${PILI_BUNDLE_RESTAURANT_ADDRESS:-1 Rue du Test, 4000 Liege}",
  "restaurant_vat": "${PILI_BUNDLE_VAT:-BE0000000097}",
  "restaurant_est_no": "2000000042",
  "fdm_url": "$FDM_URL",
  "printer_address": "$PRINTER",$KITCHEN_LINE$PAPER_LINE$HUBRISE_LINE$EXPORT_LINE$TERMINAL_LINE$DIGITAL_LINE$HUBRISE_MOCK_LINE$PEPPOL_LINE
  "table_count": 25
}
JSON
if [[ "$SEED" == "1" ]]; then
    cp "$SEED_SOURCE" "$OUT/seed.sql"
fi

# MANIFEST.json: every file but the two manifest files, sorted by path.
python3 -I - "$OUT" <<'PY' || die "could not write MANIFEST.json"
import hashlib
import json
import os
import sys

root = sys.argv[1]
files = []
for dirpath, _dirs, names in os.walk(root):
    for name in names:
        full = os.path.join(dirpath, name)
        rel = os.path.relpath(full, root).replace(os.sep, "/")
        if rel in ("MANIFEST.json", "MANIFEST.json.minisig"):
            continue
        with open(full, "rb") as f:
            data = f.read()
        files.append({"path": rel, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()})
files.sort(key=lambda e: e["path"].encode("utf-8"))
with open(os.path.join(root, "MANIFEST.json"), "w", encoding="utf-8", newline="\n") as f:
    f.write(json.dumps({"format": 1, "files": files}, separators=(",", ":")) + "\n")
PY

"$MINISIGN" -S -s "$KEYDIR/minisign.key" -m "$OUT/MANIFEST.json" -x "$OUT/MANIFEST.json.minisig" -t "pili dev bundle" >/dev/null 2>&1 \
    || die "minisign -S failed"
# Only prehashed signatures are accepted by the app: their algorithm bytes are 'ED' (base64 'RU...', a legacy signature starts with 'RW').
sig_line="$(sed -n '2p' "$OUT/MANIFEST.json.minisig")"
[[ "$sig_line" == RU* ]] || die "the signature is not a prehashed minisign signature"
"$MINISIGN" -V -p "$COMMITTED_PUB" -m "$OUT/MANIFEST.json" -x "$OUT/MANIFEST.json.minisig" >/dev/null 2>&1 \
    || die "minisign -V failed with $COMMITTED_PUB"

echo "DEVBUNDLE | PASS | $OUT"
