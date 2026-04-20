#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# Tokyo Sushi Bar — POS Update Script
# Usage: sudo ./update.sh /path/to/tsb-pos [--seed /path/to/seed.sql]
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

POS_BIN_DIR="/opt/tsb-pos"
POS_HOME="/var/lib/tsb-pos"
POS_DB_PATH="${POS_HOME}/pos.db"
SERVICE="tsb-pos.service"

BINARY_PATH="${1:-}"
SEED_PATH=""

shift || true
while [[ $# -gt 0 ]]; do
    case "$1" in
        --seed) SEED_PATH="$2"; shift 2 ;;
        *) shift ;;
    esac
done

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: Run as root." ; exit 1
fi

if [[ -z "$BINARY_PATH" || ! -f "$BINARY_PATH" ]]; then
    echo "Usage: sudo $0 /path/to/tsb-pos [--seed /path/to/seed.sql]"
    exit 1
fi

echo "▸ Stopping POS service..."
systemctl stop "$SERVICE" 2>/dev/null || true
sleep 1

echo "▸ Backing up current binary..."
if [[ -f "$POS_BIN_DIR/tsb-pos" ]]; then
    cp "$POS_BIN_DIR/tsb-pos" "$POS_BIN_DIR/tsb-pos.bak"
fi

echo "▸ Backing up database..."
if [[ -f "$POS_DB_PATH" ]]; then
    cp "$POS_DB_PATH" "$POS_DB_PATH.bak.$(date +%Y%m%d-%H%M%S)"
fi

echo "▸ Installing new binary..."
cp "$BINARY_PATH" "$POS_BIN_DIR/tsb-pos"
chmod 755 "$POS_BIN_DIR/tsb-pos"

if [[ -n "$SEED_PATH" && -f "$SEED_PATH" ]]; then
    echo "▸ Applying seed update..."
    sqlite3 "$POS_DB_PATH" < "$SEED_PATH"
    chown pos:pos "$POS_DB_PATH" "$POS_DB_PATH"-* 2>/dev/null || true
fi

echo "▸ Starting POS service..."
systemctl start "$SERVICE"

echo "▸ Checking status..."
sleep 2
systemctl is-active "$SERVICE" && echo "✓ POS is running" || echo "✗ POS failed to start — check: journalctl -u $SERVICE"
