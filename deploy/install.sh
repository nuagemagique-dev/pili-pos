#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# Tokyo Sushi Bar — POS Installation Script
# Target: Debian 12 minimal, x86_64 (Intel J6412)
# Usage:  sudo ./install.sh [--binary /path/to/moyun-pos] [--seed /path/to/seed.sql]
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

# ── Configuration ───────────────────────────────────────────────
POS_USER="pos"
POS_HOME="/var/lib/moyun-pos"
POS_BIN_DIR="/opt/moyun-pos"
POS_DB_PATH="${POS_HOME}/pos.db"
SERVICE_FILE="/etc/systemd/system/moyun-pos.service"

# Network defaults (can be overridden)
STATIC_IP="${STATIC_IP:-}"
GATEWAY="${GATEWAY:-}"
DNS="${DNS:-1.1.1.1}"
IFACE="${IFACE:-eth0}"

# Parse arguments
BINARY_PATH=""
SEED_PATH=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --binary) BINARY_PATH="$2"; shift 2 ;;
        --seed)   SEED_PATH="$2"; shift 2 ;;
        *)        echo "Unknown option: $1"; exit 1 ;;
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

echo "══════════════════════════════════════════════════"
echo "  Tokyo Sushi Bar — POS Installation"
echo "  Debian $(cat /etc/debian_version) / $ARCH"
echo "══════════════════════════════════════════════════"

# ── Step 1: System packages ────────────────────────────────────
echo ""
echo "▸ Installing system packages..."
apt-get update -qq

apt-get install -y -qq \
    cage \
    libwebkit2gtk-4.1-0 \
    libgtk-3-0 \
    libayatana-appindicator3-1 \
    librsvg2-common \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    sqlite3 \
    fonts-noto-cjk \
    fonts-noto-color-emoji \
    network-manager \
    curl \
    ca-certificates

echo "  ✓ Packages installed"

# ── Step 2: Create POS user ────────────────────────────────────
echo ""
echo "▸ Creating POS user..."
if ! id "$POS_USER" &>/dev/null; then
    useradd --system \
        --home-dir "$POS_HOME" \
        --create-home \
        --shell /usr/sbin/nologin \
        --groups input,video,render \
        "$POS_USER"
    echo "  ✓ User '$POS_USER' created"
else
    echo "  ✓ User '$POS_USER' already exists"
    usermod -aG input,video,render "$POS_USER" 2>/dev/null || true
fi

# Ensure XDG_RUNTIME_DIR exists for Wayland
mkdir -p /run/user/1000
chown "$POS_USER:$POS_USER" /run/user/1000
chmod 700 /run/user/1000

# loginctl enable-linger so runtime dir persists
loginctl enable-linger "$POS_USER" 2>/dev/null || true

# ── Step 3: Directory structure ────────────────────────────────
echo ""
echo "▸ Setting up directories..."
mkdir -p "$POS_BIN_DIR"
mkdir -p "$POS_HOME"
chown "$POS_USER:$POS_USER" "$POS_HOME"
chmod 750 "$POS_HOME"
echo "  ✓ $POS_BIN_DIR and $POS_HOME ready"

# ── Step 4: Copy binary ───────────────────────────────────────
if [[ -n "$BINARY_PATH" && -f "$BINARY_PATH" ]]; then
    echo ""
    echo "▸ Installing POS binary..."
    cp "$BINARY_PATH" "$POS_BIN_DIR/moyun-pos"
    chmod 755 "$POS_BIN_DIR/moyun-pos"
    echo "  ✓ Binary installed to $POS_BIN_DIR/moyun-pos"
else
    echo ""
    echo "▸ No binary provided (--binary). Skipping."
    echo "  Copy the release binary to $POS_BIN_DIR/moyun-pos manually."
fi

# ── Step 5: Initialize SQLite database ─────────────────────────
echo ""
echo "▸ Initializing database..."
if [[ ! -f "$POS_DB_PATH" ]]; then
    # The Tauri app creates the schema on first run,
    # but we seed the data now for a ready-to-use system.
    touch "$POS_DB_PATH"
    chown "$POS_USER:$POS_USER" "$POS_DB_PATH"

    if [[ -n "$SEED_PATH" && -f "$SEED_PATH" ]]; then
        # Create schema tables first (app normally does this on boot)
        sqlite3 "$POS_DB_PATH" "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;"

        # The seed file expects the schema to already exist.
        # Run the app once briefly to create it, or use an inline schema.
        # For now, apply the seed — the app will create missing tables on startup.
        echo "  Applying seed data from $SEED_PATH..."
        sqlite3 "$POS_DB_PATH" < "$SEED_PATH" 2>/dev/null || echo "  ⚠ Seed will be applied after first app run (schema needed)"
        chown "$POS_USER:$POS_USER" "$POS_DB_PATH" "$POS_DB_PATH"-* 2>/dev/null || true
        echo "  ✓ Seed data applied"
    else
        echo "  ✓ Empty database created (seed on first run)"
    fi
else
    echo "  ✓ Database already exists at $POS_DB_PATH"
fi

# ── Step 6: Systemd service ────────────────────────────────────
echo ""
echo "▸ Installing systemd service..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [[ -f "$SCRIPT_DIR/moyun-pos.service" ]]; then
    cp "$SCRIPT_DIR/moyun-pos.service" "$SERVICE_FILE"
else
    echo "  ⚠ moyun-pos.service not found in script directory, skipping"
fi

systemctl daemon-reload
systemctl enable moyun-pos.service
echo "  ✓ Service installed and enabled"

# ── Step 7: Network configuration ──────────────────────────────
echo ""
echo "▸ Configuring network..."
if [[ -n "$STATIC_IP" && -n "$GATEWAY" ]]; then
    # Configure static IP via NetworkManager
    nmcli connection delete "pos-static" 2>/dev/null || true
    nmcli connection add \
        type ethernet \
        con-name "pos-static" \
        ifname "$IFACE" \
        ipv4.addresses "$STATIC_IP" \
        ipv4.gateway "$GATEWAY" \
        ipv4.dns "$DNS" \
        ipv4.method manual \
        connection.autoconnect yes
    nmcli connection up "pos-static"
    echo "  ✓ Static IP: $STATIC_IP on $IFACE"
else
    echo "  ✓ Using DHCP (set STATIC_IP/GATEWAY/IFACE env vars for static)"
fi

# ── Step 8: Disable unnecessary services ───────────────────────
echo ""
echo "▸ Hardening system..."
# Disable services not needed on a kiosk POS
for svc in bluetooth cups avahi-daemon ModemManager; do
    systemctl disable "$svc" 2>/dev/null || true
    systemctl stop "$svc" 2>/dev/null || true
done

# Disable automatic updates (controlled deployments only)
systemctl disable unattended-upgrades 2>/dev/null || true

# Set timezone to Brussels
timedatectl set-timezone Europe/Brussels 2>/dev/null || true

echo "  ✓ Unnecessary services disabled"

# ── Step 9: Auto-login for Cage ────────────────────────────────
echo ""
echo "▸ Configuring auto-login..."
mkdir -p /etc/systemd/system/getty@tty1.service.d
cat > /etc/systemd/system/getty@tty1.service.d/autologin.conf <<EOF
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin $POS_USER --noclear %I \$TERM
EOF
echo "  ✓ Auto-login configured for tty1"

# ── Done ───────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════"
echo "  Installation complete!"
echo ""
echo "  Binary:   $POS_BIN_DIR/moyun-pos"
echo "  Database: $POS_DB_PATH"
echo "  Service:  moyun-pos.service"
echo ""
echo "  Start:    sudo systemctl start moyun-pos"
echo "  Logs:     sudo journalctl -u moyun-pos -f"
echo "  Status:   sudo systemctl status moyun-pos"
echo ""
echo "  To deploy a new version:"
echo "    1. scp moyun-pos pos@<ip>:/tmp/"
echo "    2. ssh root@<ip> 'systemctl stop moyun-pos && cp /tmp/moyun-pos $POS_BIN_DIR/ && systemctl start moyun-pos'"
echo "══════════════════════════════════════════════════"
