#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/make_dev_bundle.sh
#
# Usage: bash scripts/android/test_make_dev_bundle.sh
#
# Runs make_dev_bundle.sh with a temporary HOME, a temporary key directory and a temporary stand-in for the
# committed public key (PILI_DEV_PUB), so the real development key is never touched. Cases: a bundle is
# created, verified and prehashed; --seed adds seed.sql to the manifest; an --out or --keydir inside a git
# work tree is refused; a different public key is refused; a non-empty foreign --out is refused; the
# manifest is sorted and identical between two runs; pos.json carries the given fdm url and printer; the demo
# payment terminal section appears only with PILI_BUNDLE_PAYMENT_TERMINAL_OPI and its address and device port are checked.
# Prints one line per case and a summary; exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SCRIPT="$ROOT/scripts/android/make_dev_bundle.sh"
TMP="$(cd "$(mktemp -d)" && pwd -P)"
trap 'rm -rf "$TMP"' EXIT
export HOME="$TMP/home"
mkdir -p "$HOME"
export PILI_DEV_PUB="$TMP/committed/dev-minisign.pub"
KEYS="$TMP/keys"
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TEST | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TEST | %s | FAIL | %s\n' "$1" "$2"; }

# Case 1: the first run creates the key pair, writes the stand-in public key and a verified bundle.
out1="$(bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/b1" --write-pub 2>&1)"
rc=$?
if [[ $rc -eq 0 && "$out1" == *"DEVBUNDLE | PASS | $TMP/b1"* ]]; then ok "bundle-created"; else bad "bundle-created" "rc=$rc: $out1"; fi

if minisign -V -p "$PILI_DEV_PUB" -m "$TMP/b1/MANIFEST.json" -x "$TMP/b1/MANIFEST.json.minisig" >/dev/null 2>&1 \
    && [[ "$(sed -n '2p' "$TMP/b1/MANIFEST.json.minisig")" == RU* ]] \
    && grep -q '^trusted comment: pili dev bundle$' "$TMP/b1/MANIFEST.json.minisig"; then
    ok "signature-verifies-prehashed"
else
    bad "signature-verifies-prehashed" "minisign -V failed or the signature is not prehashed"
fi

mode="$(stat -c '%a' "$KEYS/minisign.key" 2>/dev/null || stat -f '%Lp' "$KEYS/minisign.key")"
if [[ "$mode" == "600" ]]; then ok "secret-key-mode-600"; else bad "secret-key-mode-600" "mode $mode"; fi

if python3 -I - "$TMP/b1" <<'PY'
import json, sys
m = json.load(open(sys.argv[1] + "/MANIFEST.json"))
paths = [f["path"] for f in m["files"]]
assert m["format"] == 1 and paths == ["pos.json"], paths
PY
then ok "manifest-without-seed"; else bad "manifest-without-seed" "unexpected manifest"; fi

# Case 2: --seed lists seed.sql, sorted after pos.json, with the right hash.
out2="$(bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/b2" --seed --fdm-url http://10.0.2.2:19380/graphql --printer 10.0.2.2:19390 2>&1)"
rc=$?
if [[ $rc -eq 0 && "$out2" == *"DEVBUNDLE | PASS"* ]] && python3 -I - "$TMP/b2" <<'PY'
import hashlib, json, sys
root = sys.argv[1]
m = json.load(open(root + "/MANIFEST.json"))
paths = [f["path"] for f in m["files"]]
assert paths == ["pos.json", "seed.sql"], paths
for f in m["files"]:
    data = open(root + "/" + f["path"], "rb").read()
    assert f["size"] == len(data) and f["sha256"] == hashlib.sha256(data).hexdigest(), f
pos = json.load(open(root + "/pos.json"))
assert pos["fdm_url"] == "http://10.0.2.2:19380/graphql" and pos["printer_address"] == "10.0.2.2:19390"
assert pos["pos_id"] == "CPOS0031234567" and pos["terminal_id"] == "POS-2-ANDROID" and pos["table_count"] == 25
assert "token" not in json.dumps(pos)
PY
then ok "seed-and-pos-json"; else bad "seed-and-pos-json" "rc=$rc: $out2"; fi

# Case 3: an --out inside a git work tree is refused, and so is a --keydir.
git init -q "$TMP/repo"
out3="$(bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/repo/bundle" 2>&1)"
rc=$?
if [[ $rc -ne 0 && "$out3" == *"inside a git work tree"* && ! -e "$TMP/repo/bundle" ]]; then ok "out-in-repo-refused"; else bad "out-in-repo-refused" "rc=$rc: $out3"; fi
out3b="$(bash "$SCRIPT" --keydir "$TMP/repo/keys" --out "$TMP/b3" 2>&1)"
rc=$?
if [[ $rc -ne 0 && "$out3b" == *"inside a git work tree"* && ! -e "$TMP/repo/keys" ]]; then ok "keydir-in-repo-refused"; else bad "keydir-in-repo-refused" "rc=$rc: $out3b"; fi

# Case 4: a key pair whose public half is not the committed one is refused, with instructions.
out4="$(bash "$SCRIPT" --keydir "$TMP/other-keys" --out "$TMP/b4" 2>&1)"
rc=$?
if [[ $rc -ne 0 && "$out4" == *"--write-pub"* && ! -e "$TMP/b4/MANIFEST.json" ]]; then ok "other-public-key-refused"; else bad "other-public-key-refused" "rc=$rc: $out4"; fi

# Case 5: a non-empty directory that is not a bundle is never replaced.
mkdir -p "$TMP/foreign"
echo keep >"$TMP/foreign/file.txt"
out5="$(bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/foreign" 2>&1)"
rc=$?
if [[ $rc -ne 0 && -f "$TMP/foreign/file.txt" ]]; then ok "foreign-out-dir-kept"; else bad "foreign-out-dir-kept" "rc=$rc: $out5"; fi

# Case 6: the manifest order and content are deterministic between two runs (the signature is not).
bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/d1" --seed >/dev/null 2>&1
bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/d2" --seed >/dev/null 2>&1
if cmp -s "$TMP/d1/MANIFEST.json" "$TMP/d2/MANIFEST.json" && [[ -s "$TMP/d1/MANIFEST.json" ]]; then ok "manifest-deterministic"; else bad "manifest-deterministic" "two runs differ"; fi

# Case 7: a rerun on the same --out replaces the previous bundle.
out7="$(bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/d1" 2>&1)"
rc=$?
if [[ $rc -eq 0 && ! -e "$TMP/d1/seed.sql" ]]; then ok "rerun-replaces-bundle"; else bad "rerun-replaces-bundle" "rc=$rc: $out7"; fi

# Case 8: the demo terminal: PILI_BUNDLE_PAYMENT_TERMINAL_OPI adds a ccv_opi payment_terminal section, valid JSON,
# next to the other optional sections; without it pos.json has none; a malformed address or port is refused.
out8="$(PILI_BUNDLE_PAYMENT_TERMINAL_OPI=127.0.0.1:19410 PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT=19412 PILI_BUNDLE_HUBRISE_ENABLED=1 PILI_BUNDLE_KITCHEN_PRINTER=127.0.0.1:19392 \
    bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/t1" 2>&1)"
rc=$?
if [[ $rc -eq 0 && "$out8" == *"DEVBUNDLE | PASS"* ]] && python3 -I - "$TMP/t1" <<'PY'
import json, sys
pos = json.load(open(sys.argv[1] + "/pos.json"))
t = pos["payment_terminal"]
assert t == {"kind": "ccv_opi", "address": "127.0.0.1:19410", "device_port": 19412, "timeout_seconds": 60}, t
assert pos["hubrise_enabled"] is True and pos["kitchen_printer_address"] == "127.0.0.1:19392" and pos["table_count"] == 25
PY
then ok "payment-terminal-section"; else bad "payment-terminal-section" "rc=$rc: $out8"; fi
if python3 -I -c 'import json,sys; sys.exit("payment_terminal" in json.load(open(sys.argv[1]))) ' "$TMP/b2/pos.json"; then ok "no-payment-terminal-by-default"; else bad "no-payment-terminal-by-default" "pos.json has a terminal"; fi
out8b="$(PILI_BUNDLE_PAYMENT_TERMINAL_OPI='x"y:1' bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/t2" 2>&1)"
rc=$?
if [[ $rc -ne 0 && "$out8b" == *"PILI_BUNDLE_PAYMENT_TERMINAL_OPI"* && ! -e "$TMP/t2/MANIFEST.json" ]]; then ok "payment-terminal-address-checked"; else bad "payment-terminal-address-checked" "rc=$rc: $out8b"; fi
out8c="$(PILI_BUNDLE_PAYMENT_TERMINAL_OPI=127.0.0.1:19410 PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT=70000 bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/t3" 2>&1)"
rc=$?
if [[ $rc -ne 0 && "$out8c" == *"PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT"* && ! -e "$TMP/t3/MANIFEST.json" ]]; then ok "payment-terminal-device-port-checked"; else bad "payment-terminal-device-port-checked" "rc=$rc: $out8c"; fi
out8d="$(PILI_BUNDLE_PAYMENT_TERMINAL_OPI=192.168.1.50:4100 bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/t4" 2>&1)"
rc=$?
if [[ $rc -eq 0 ]] && python3 -I -c 'import json,sys; t=json.load(open(sys.argv[1]))["payment_terminal"]; sys.exit(0 if t=={"kind":"ccv_opi","address":"192.168.1.50:4100","timeout_seconds":60} else 1)' "$TMP/t4/pos.json"; then ok "payment-terminal-device-port-optional"; else bad "payment-terminal-device-port-optional" "rc=$rc: $out8d"; fi

# Case 8e: PILI_BUNDLE_PEPPOL_API adds a storecove peppol section (a valid sender id) and its key file; a malformed URL is refused.
out8e="$(PILI_BUNDLE_PEPPOL_API=http://127.0.0.1:19380 bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/t5" 2>&1)"
rc=$?
if [[ $rc -eq 0 && "$out8e" == *"DEVBUNDLE | PASS"* && -s "$TMP/t5/peppol_key" ]] && python3 -I - "$TMP/t5" <<'PY'
import json, sys
pos = json.load(open(sys.argv[1] + "/pos.json"))
p = pos["peppol"]
assert p["provider"] == "storecove" and p["base_url"] == "http://127.0.0.1:19380/storecove", p
assert p["api_key_file"] == "peppol_key" and p["sender_id"] == "0208:0000000097" and p["legal_entity_id"] == 1, p
assert pos["table_count"] == 25
PY
then ok "peppol-section"; else bad "peppol-section" "rc=$rc: $out8e"; fi
if python3 -I -c 'import json,sys; sys.exit("peppol" in json.load(open(sys.argv[1]))) ' "$TMP/b2/pos.json"; then ok "no-peppol-by-default"; else bad "no-peppol-by-default" "pos.json has a peppol section"; fi
out8f="$(PILI_BUNDLE_PEPPOL_API='x"y' bash "$SCRIPT" --keydir "$KEYS" --out "$TMP/t6" 2>&1)"
rc=$?
if [[ $rc -ne 0 && "$out8f" == *"PILI_BUNDLE_PEPPOL_API"* && ! -e "$TMP/t6/MANIFEST.json" ]]; then ok "peppol-url-checked"; else bad "peppol-url-checked" "rc=$rc: $out8f"; fi

# Case 9: a usage error.
bash "$SCRIPT" --nonsense >/dev/null 2>&1
rc=$?
if [[ $rc -eq 2 ]]; then ok "usage-error-2"; else bad "usage-error-2" "rc=$rc"; fi

printf 'TEST | total | %s | %d passed, %d failed\n' "$([[ $fail -eq 0 ]] && echo PASS || echo FAIL)" "$pass" "$fail"
[[ $fail -eq 0 ]]
