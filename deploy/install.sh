#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos - Installation Script (thin wrapper)
# Target: an ALREADY INSTALLED, booted Debian 12 minimal, x86_64
# (Intel J6412).
#
# The kiosk image (deploy/image/, built with mkosi) is the primary
# deployment path: a distributor writes it to the terminal's disk from a
# USB stick (docs/SUBMISSION_ARTIFACTS/MANUEL_INSTALLATION_FR.md §3).
# This script remains for installing moyun-pos directly onto a plain
# Debian 12 box - dev/test machines, or before the image flow is used -
# and now just runs all three deploy/provision/provision.sh phases in
# order; see that script for what each phase actually does.
#
# Usage:  sudo ./install.sh --binary /path/to/moyun-pos [--seed /path/to/seed.sql]
#                            [--rustdesk /path/to/rustdesk.AppImage]
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROVISION="$SCRIPT_DIR/provision/provision.sh"

BINARY_PATH=""
SEED_PATH=""
RUSTDESK_PATH=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --binary)   BINARY_PATH="$2"; shift 2 ;;
        --seed)     SEED_PATH="$2"; shift 2 ;;
        --rustdesk) RUSTDESK_PATH="$2"; shift 2 ;;
        *)          echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ── Pre-flight checks ──────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then
    echo "ERROR: This script must be run as root (sudo)."
    exit 1
fi

if [[ ! -f /etc/debian_version ]]; then
    echo "ERROR: This script targets Debian 12."
    exit 1
fi

ARCH=$(uname -m)
if [[ "$ARCH" != "x86_64" ]]; then
    echo "ERROR: Expected x86_64, got $ARCH"
    exit 1
fi

if [[ -z "$BINARY_PATH" || ! -f "$BINARY_PATH" ]]; then
    echo "ERROR: --binary /path/to/moyun-pos is required."
    exit 1
fi

if [[ -n "$SEED_PATH" && ! -f "$SEED_PATH" ]]; then
    echo "ERROR: seed file not found: $SEED_PATH"
    exit 1
fi

echo "══════════════════════════════════════════════════"
echo "  moyun-pos - Installation"
echo "  Debian $(cat /etc/debian_version) / $ARCH"
echo "══════════════════════════════════════════════════"

"$PROVISION" --phase packages

IMAGE_ARGS=(--phase image --variant production --binary "$BINARY_PATH")
[[ -n "$SEED_PATH" ]] && IMAGE_ARGS+=(--seed "$SEED_PATH")
[[ -n "$RUSTDESK_PATH" ]] && IMAGE_ARGS+=(--rustdesk "$RUSTDESK_PATH")
"$PROVISION" "${IMAGE_ARGS[@]}"

"$PROVISION" --phase instance

# ── Done ───────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════"
echo "  Installation complete!"
echo ""
echo "  Binary:   /opt/moyun-pos/moyun-pos"
echo "  Database: /var/lib/moyun-pos/pos.db"
echo "  Service:  moyun-pos.service (tty1)"
echo ""
echo "  Start:    sudo systemctl start moyun-pos"
echo "  Logs:     sudo journalctl -u moyun-pos -f"
echo "            /var/lib/moyun-pos/logs/pos.log.*"
echo ""
echo "  BEFORE production use:"
echo "    1. Edit /etc/moyun-pos/pos.json (identity, FDM URL, language)."
echo "       device_id was auto-filled from the NIC MAC; leave it as is"
echo "       unless the network card is replaced."
echo "       Then check it: /opt/moyun-pos/moyun-pos --check-config"
echo "       (lists what the FDM would refuse; the POS also shows it)."
echo "    2. Seal the appliance (GARANTIE_MAJ_LANGUES_FR.md §1.3):"
echo "       - root filesystem read-only: add 'ro' to the root line"
echo "         in /etc/fstab (keep /var/lib/moyun-pos on a rw mount)."
echo "       - verify no SSH server: dpkg -l openssh-server → absent."
echo "    3. Updates: on site with deploy/update.sh from a USB stick, or"
echo "       remotely through an on-demand RustDesk session"
echo "       (deploy/REMOTE-OPS.md). No permanent remote channel."
echo "══════════════════════════════════════════════════"
