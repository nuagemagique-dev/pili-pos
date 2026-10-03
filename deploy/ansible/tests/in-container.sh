#!/usr/bin/env bash
# See run.sh. Runs inside debian:trixie as root.
set -euo pipefail

apt-get update -qq >/dev/null
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    ansible-core minisign python3 sqlite3 iproute2 util-linux >/dev/null
echo "ansible-core: $(ansible-playbook --version | head -n1)"

PASS=0
FAIL=0
ok()   { echo "  ✓ $*"; PASS=$((PASS + 1)); }
bad()  { echo "  ✗ $*"; FAIL=$((FAIL + 1)); }
check() { local what="$1"; shift; if "$@"; then ok "$what"; else bad "$what"; fi; }

# ── Terminal layout ───────────────────────────────────────────────
useradd --system --home-dir /var/lib/pili-pos --shell /usr/sbin/nologin pos
mkdir -p /opt/pili-pos/deploy /var/lib/pili-pos /etc/pili-pos /usr/share/pili-pos
chown pos:pos /var/lib/pili-pos
cp -R /src/deploy/ansible /opt/pili-pos/deploy/ansible
install -m 755 /src/deploy/update.sh /opt/pili-pos/deploy/update.sh

fake_pos() { # fake_pos <version> <out>
    cat > "$2" <<FAKE
#!/bin/sh
# fake pili-pos $1
case "\$1" in
    --version) echo "pili-pos $1" ;;
    --check-config) grep -q '"pos_id"' /etc/pili-pos/pos.json 2>/dev/null || { echo "no configuration" >&2; exit 1; } ;;
    --init-db) exit 0 ;;
esac
FAKE
    chmod 755 "$2"
}
fake_pos 1.0.0 /opt/pili-pos/pili-pos
EMBEDDED_SHA1="$(sha1sum /opt/pili-pos/pili-pos | awk '{print $1}')"

# ── Owner key + releases server ───────────────────────────────────
KEYS=/root/keys
mkdir -p "$KEYS"
minisign -G -W -p "$KEYS/pub" -s "$KEYS/sec" >/dev/null
cp "$KEYS/pub" /usr/share/pili-pos/declared.pub

SRV=/srv/releases
mkdir -p "$SRV/declared"
publish() { # publish <version> [sha1 override] -> tarball + signed manifest
    local v="$1" dir="$SRV/v$1" sha1 sha256
    mkdir -p "$dir/pkg"
    fake_pos "$v" "$dir/pkg/pili-pos"
    printf 'x' > "$dir/pkg/install.sh"
    tar -C "$dir/pkg" -czf "$dir/pili-pos-v$v-x86_64.tar.gz" pili-pos install.sh
    sha1="${2:-$(sha1sum "$dir/pkg/pili-pos" | awk '{print $1}')}"
    sha256="$(sha256sum "$dir/pili-pos-v$v-x86_64.tar.gz" | awk '{print $1}')"
    cat > "$SRV/declared/declared.json" <<JSON
{"version": "$v", "sha1": "$sha1", "sha256": "$sha256",
 "asset_url": "http://127.0.0.1:8000/v$v/pili-pos-v$v-x86_64.tar.gz"}
JSON
    rm -f "$SRV/declared/declared.json.minisig"
    minisign -S -s "$KEYS/sec" -m "$SRV/declared/declared.json" >/dev/null
    sha1sum "$dir/pkg/pili-pos" | awk '{print $1}'
}
( cd "$SRV" && exec python3 -m http.server 8000 --bind 127.0.0.1 >/dev/null 2>&1 ) &
sleep 1

# ── Site directory (as make-key.sh writes it on the ESP) ──────────
new_site() {
    rm -rf /run/site
    mkdir -p /run/site/secrets/fdm
    cat > /run/site/site.yml <<'YML'
network:
  mode: dhcp
pos:
  pos_id: CPOS0031234567
  terminal_id: POS1
  restaurant_name: Restaurant Exemple
  restaurant_address: Rue de l'Exemple 1, 4000 Liège
  restaurant_vat: BE0499999960
  restaurant_est_no: "2123456791"
  fdm_url: https://127.0.0.1:9443/graphql
  printer_address: 127.0.0.1:9100
  hubrise_enabled: false
  hubrise:
    client_id: test.clients.hubrise.com
YML
    echo token > /run/site/secrets/fdm/token
    echo secret > /run/site/secrets/hubrise.secret
}

play() { # play [extra -e args...]
    rm -f /var/lib/pili-pos/bootstrap-report.txt
    ANSIBLE_CONFIG=/opt/pili-pos/deploy/ansible/ansible.cfg \
    ansible-playbook -i localhost, -c local /opt/pili-pos/deploy/ansible/site.yml \
        -e pili_site_dir=/run/site \
        -e pili_first_boot=true \
        -e pili_declared_base_url=http://127.0.0.1:8000/declared \
        -e pili_fetch_timeout=5 \
        "$@" > /tmp/play.log 2>&1
    local rc=$?
    sed 's/^/      | /' /var/lib/pili-pos/bootstrap-report.txt 2>/dev/null || true
    [[ -n "${VERBOSE:-}" ]] && cat /tmp/play.log
    return $rc
}
installed_sha1() { sha1sum /opt/pili-pos/pili-pos | awk '{print $1}'; }
report_has() { grep -q -- "$1" /var/lib/pili-pos/bootstrap-report.txt; }

echo "▸ 1. declared 1.1.0, signed: installed"
new_site
V11_SHA1="$(publish 1.1.0)"
if play; then ok "playbook ran"; else bad "playbook failed"; tail -40 /tmp/play.log; fi
check "binary is the declared 1.1.0" test "$(installed_sha1)" = "$V11_SHA1"
check "previous binary kept as .bak" test "$(sha1sum /opt/pili-pos/pili-pos.bak | awk '{print $1}')" = "$EMBEDDED_SHA1"
check "pos.json has the site identity" grep -q '"pos_id": "CPOS0031234567"' /etc/pili-pos/pos.json
check "pos.json device_id filled in" grep -Eq '"device_id": "([0-9a-f]{2}:){5}[0-9a-f]{2}"' /etc/pili-pos/pos.json
check "fdm_token_file set" grep -q '"fdm_token_file": "/etc/pili-pos/fdm/token"' /etc/pili-pos/pos.json
check "hubrise secret path merged" grep -q '"client_secret_file": "/etc/pili-pos/secrets/hubrise.secret"' /etc/pili-pos/pos.json
check "hubrise client_id kept" grep -q '"client_id": "test.clients.hubrise.com"' /etc/pili-pos/pos.json
check "token installed 0640 root:pos" test "$(stat -c '%a %U %G' /etc/pili-pos/fdm/token)" = "640 root pos"
check "secrets deleted from the site dir" test ! -e /run/site/secrets
check "site.yml copy kept in /etc/pili-pos" test -f /etc/pili-pos/site.yml
check "report names the update" report_has "mis à jour vers la version déclarée 1.1.0"
check "report warns: FDM unreachable" report_has "FDM injoignable"

echo "▸ 2. same manifest again (re-run): nothing to do"
if play -e pili_first_boot=false; then ok "playbook ran"; else bad "playbook failed"; tail -40 /tmp/play.log; fi
check "report: already installed" report_has "déjà installée"
check "pos.json still there (secrets from the earlier run)" grep -q '"fdm_token_file"' /etc/pili-pos/pos.json

echo "▸ 3. tampered manifest: embedded binary kept"
fake_pos 1.0.0 /opt/pili-pos/pili-pos
new_site
publish 1.2.0 >/dev/null
sed -i 's/"1.2.0"/"1.2.1"/' "$SRV/declared/declared.json"
if play; then ok "playbook ran"; else bad "playbook failed"; tail -40 /tmp/play.log; fi
check "binary unchanged" test "$(installed_sha1)" = "$EMBEDDED_SHA1"
check "report warns about the signature" report_has "Check the owner's signature"

echo "▸ 4. signed manifest, wrong SHA-1: refused"
new_site
publish 1.3.0 0000000000000000000000000000000000000000 >/dev/null
if play; then ok "playbook ran"; else bad "playbook failed"; tail -40 /tmp/play.log; fi
check "binary unchanged" test "$(installed_sha1)" = "$EMBEDDED_SHA1"
check "report: SHA-1 refused" report_has "It must be the declared binary"

echo "▸ 5. declared version older than the image: kept"
new_site
publish 0.9.0 >/dev/null
if play; then ok "playbook ran"; else bad "playbook failed"; tail -40 /tmp/play.log; fi
check "binary unchanged" test "$(installed_sha1)" = "$EMBEDDED_SHA1"
check "report: older" report_has "plus ancienne"

echo "▸ 6. offline: no download at all"
new_site
publish 1.4.0 >/dev/null
if play -e pili_offline=true; then ok "playbook ran"; else bad "playbook failed"; tail -40 /tmp/play.log; fi
check "binary unchanged" test "$(installed_sha1)" = "$EMBEDDED_SHA1"
check "report: embedded" report_has "binaire embarqué"

echo "▸ 7. server unreachable: embedded binary kept, run still succeeds"
new_site
if play -e pili_declared_base_url=http://127.0.0.1:1/declared; then ok "playbook ran"; else bad "playbook failed"; tail -40 /tmp/play.log; fi
check "binary unchanged" test "$(installed_sha1)" = "$EMBEDDED_SHA1"
check "report warns: not fetched" report_has "non récupérée"

echo "▸ 8. no site.yml: warning, no failure"
rm -rf /run/site /etc/pili-pos/pos.json /etc/pili-pos/site.yml
mkdir -p /run/site
if play -e pili_offline=true; then ok "playbook ran"; else bad "playbook failed"; tail -40 /tmp/play.log; fi
check "report warns: no site.yml" report_has "pas de site.yml"

echo
echo "passed: $PASS  failed: $FAIL"
[[ "$FAIL" -eq 0 ]]
