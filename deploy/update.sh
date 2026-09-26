#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos - Update Script
# Usage: sudo ./update.sh /path/to/moyun-pos --sha1 <declared SHA-1>
#
# Every modification of the POS requires an update certification
# request to SPF Finances (manual §II.E). Only install a binary whose
# SHA-1 matches the value declared for that update: the --sha1 check
# below refuses anything else.
#
# Run on the terminal's maintenance console (Ctrl+Alt+F2) - on site
# from a USB stick, or remotely during an on-demand RustDesk session
# (deploy/REMOTE-OPS.md).
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

POS_BIN_DIR="/opt/moyun-pos"
POS_HOME="/var/lib/moyun-pos"
POS_DB_PATH="${POS_HOME}/pos.db"
SERVICE="moyun-pos.service"

BINARY_PATH="${1:-}"
EXPECTED_SHA1=""

shift || true
while [[ $# -gt 0 ]]; do
    case "$1" in
        --sha1) EXPECTED_SHA1="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: Run as root." ; exit 1
fi

if [[ -z "$BINARY_PATH" || ! -f "$BINARY_PATH" || -z "$EXPECTED_SHA1" ]]; then
    echo "Usage: sudo $0 /path/to/moyun-pos --sha1 <declared SHA-1>"
    exit 1
fi

ACTUAL_SHA1=$(sha1sum "$BINARY_PATH" | awk '{print $1}')
if [[ "$ACTUAL_SHA1" != "$EXPECTED_SHA1" ]]; then
    echo "ERROR: SHA-1 mismatch - refusing to install."
    echo "  expected: $EXPECTED_SHA1"
    echo "  actual:   $ACTUAL_SHA1"
    exit 1
fi
echo "✓ SHA-1 matches the declared value ($ACTUAL_SHA1)"

# The kiosk image (deploy/image/) mounts root read-only; /opt/moyun-pos
# lives on that root partition, so the binary swap below needs a
# temporary rw remount. Tolerant of a root that is already read-write
# (plain deploy/install.sh targets, dev boxes): findmnt then reports no
# "ro" option and this block is a no-op.
ROOT_WAS_RO=0
ROOT_OPTS="$(findmnt -no OPTIONS /)"
if [[ ",$ROOT_OPTS," == *,ro,* ]]; then
    ROOT_WAS_RO=1
    echo "▸ Remounting / read-write..."
    mount -o remount,rw /
fi
restore_root_ro() {
    if [[ "$ROOT_WAS_RO" -eq 1 ]]; then
        echo "▸ Remounting / read-only..."
        mount -o remount,ro / || echo "  ⚠ could not remount / read-only - do it manually"
    fi
}
trap restore_root_ro EXIT

echo "▸ Stopping POS service..."
systemctl stop "$SERVICE" 2>/dev/null || true
sleep 1

echo "▸ Backing up current binary..."
if [[ -f "$POS_BIN_DIR/moyun-pos" ]]; then
    cp "$POS_BIN_DIR/moyun-pos" "$POS_BIN_DIR/moyun-pos.bak"
fi

echo "▸ Backing up database..."
if [[ -f "$POS_DB_PATH" ]]; then
    mkdir -p "$POS_HOME/backups"
    BACKUP="$POS_HOME/backups/pre-update-$(date +%Y%m%d-%H%M%S).db"
    sqlite3 "$POS_DB_PATH" ".backup '$BACKUP'"
    chown pos:pos "$BACKUP"
    echo "  ✓ $BACKUP"
fi

echo "▸ Installing new binary..."
install -m 755 "$BINARY_PATH" "$POS_BIN_DIR/moyun-pos"

# Apply schema migrations offline so a migration failure is caught
# here, with the service still stopped, rather than at kiosk boot.
echo "▸ Migrating database..."
runuser -u pos -- env MOYUN_POS_DATA_DIR="$POS_HOME" \
    "$POS_BIN_DIR/moyun-pos" --init-db

echo "▸ Starting POS service..."
systemctl start "$SERVICE"

echo "▸ Checking status..."
sleep 2
systemctl is-active "$SERVICE" && echo "✓ $("$POS_BIN_DIR/moyun-pos" --version) is running" \
    || echo "✗ POS failed to start - check: journalctl -u $SERVICE"
