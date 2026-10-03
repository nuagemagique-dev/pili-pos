#!/usr/bin/env bash
# Connect the POS's HubRise OAuth client to a HubRise test location,
# once, for the live tests (scripts/live_hubrise.sh).
#
#   scripts/live_hubrise_login.sh             # prints the authorize URL
#   scripts/live_hubrise_login.sh CODE        # exchanges the code shown
#                                             # after login for a token
#
# Reads HUBRISE_OAUTH_BASE_URL, HUBRISE_POS_CLIENT_ID,
# HUBRISE_POS_CLIENT_SECRET, HUBRISE_POS_REDIRECT_URI (the `oob` flow)
# and HUBRISE_POS_SCOPE from .env, then .env.live (both git-ignored).
# The live tests load their own test catalog into the test location,
# so the login asks for HUBRISE_LIVE_SCOPE (default
# location[orders.write,catalog.write]) instead of the POS scope.
# The token goes to target/live-hubrise/token.json (mode 600, ignored
# by git with the rest of target/). Neither the secret nor the token is
# ever printed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

for f in .env .env.live; do
    if [[ -f $f ]]; then
        set -a
        # shellcheck disable=SC1090
        . "./$f"
        set +a
    fi
done
for v in HUBRISE_OAUTH_BASE_URL HUBRISE_POS_CLIENT_ID HUBRISE_POS_CLIENT_SECRET \
         HUBRISE_POS_REDIRECT_URI HUBRISE_POS_SCOPE; do
    if [[ -z "${!v:-}" ]]; then
        echo "✗ $v is not set (put it in .env)" >&2
        exit 1
    fi
done

OUT="$ROOT/target/live-hubrise"
mkdir -p "$OUT"
chmod 700 "$OUT"

if [[ $# -eq 0 ]]; then
    python3 - <<'EOF'
import os, urllib.parse
q = urllib.parse.urlencode({
    "redirect_uri": os.environ["HUBRISE_POS_REDIRECT_URI"],
    "client_id": os.environ["HUBRISE_POS_CLIENT_ID"],
    "scope": os.environ.get("HUBRISE_LIVE_SCOPE")
        or "location[orders.write,catalog.write]",
})
print(f'{os.environ["HUBRISE_OAUTH_BASE_URL"]}/authorize?{q}')
EOF
    exit 0
fi

CODE="$1"
umask 077
status=$(curl -sS -o "$OUT/token.json" -w '%{http_code}' \
    -u "${HUBRISE_POS_CLIENT_ID}:${HUBRISE_POS_CLIENT_SECRET}" \
    --data-urlencode "code=${CODE}" \
    "${HUBRISE_OAUTH_BASE_URL}/token")
if [[ $status != 200 ]]; then
    echo "✗ token exchange answered HTTP $status" >&2
    # The error body holds no token; show its message only.
    python3 -c 'import json,sys
try: print(json.load(open(sys.argv[1])).get("message", "(no message)"), file=sys.stderr)
except Exception: print("(unreadable body)", file=sys.stderr)' "$OUT/token.json"
    rm -f "$OUT/token.json"
    exit 1
fi
python3 - "$OUT/token.json" <<'EOF'
import json, sys
t = json.load(open(sys.argv[1]))
print("✓ connected:", t.get("account_name"), "/", t.get("location_name"),
      f'(location {t.get("location_id")}, catalog {t.get("catalog_id") or "none"})')
EOF
