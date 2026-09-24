#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos — Installation Script
# Target: Debian 12 minimal, x86_64 (Intel J6412)
# Usage:  sudo ./install.sh --binary /path/to/moyun-pos [--seed /path/to/seed.sql]
#                           [--rustdesk /path/to/rustdesk.AppImage]
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

# ── Configuration ───────────────────────────────────────────────
POS_USER="pos"
POS_HOME="/var/lib/moyun-pos"
POS_BIN_DIR="/opt/moyun-pos"
POS_DB_PATH="${POS_HOME}/pos.db"
SERVICE_FILE="/etc/systemd/system/moyun-pos.service"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Remote-support client (GARANTIE_MAJ_LANGUES_FR.md §2.2): portable
# RustDesk, pinned. Installed but never started at boot.
RUSTDESK_VERSION="1.4.9"
RUSTDESK_URL="https://github.com/rustdesk/rustdesk/releases/download/${RUSTDESK_VERSION}/rustdesk-${RUSTDESK_VERSION}-x86_64.AppImage"
RUSTDESK_SHA256="7902cd60a4f29817eebe2668a15c9a1952ac690e8f7b07bfe7620fedd4e28217"

# Network defaults (can be overridden)
STATIC_IP="${STATIC_IP:-}"
GATEWAY="${GATEWAY:-}"
DNS="${DNS:-1.1.1.1}"
IFACE="${IFACE:-eth0}"

# Parse arguments
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
echo "  moyun-pos — Installation"
echo "  Debian $(cat /etc/debian_version) / $ARCH"
echo "══════════════════════════════════════════════════"

# ── Step 1: System packages ────────────────────────────────────
echo ""
echo "▸ Installing system packages..."
apt-get update -qq

# chrony: fiscal timestamps must be accurate to the second.
# xdg-desktop-portal-wlr + pipewire + libfuse2: screen capture and
# AppImage runtime for the on-demand RustDesk support session.
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
    chrony \
    dbus-user-session \
    xdg-desktop-portal-wlr \
    pipewire \
    wireplumber \
    libfuse2 \
    curl \
    ca-certificates

echo "  ✓ Packages installed"

# ── Step 2: Create POS user ────────────────────────────────────
echo ""
echo "▸ Creating POS user..."
if ! id "$POS_USER" &>/dev/null; then
    useradd --system \
        --home-dir "$POS_HOME" \
        --shell /usr/sbin/nologin \
        --groups input,video,render \
        "$POS_USER"
    echo "  ✓ User '$POS_USER' created"
else
    echo "  ✓ User '$POS_USER' already exists"
    usermod -aG input,video,render "$POS_USER" 2>/dev/null || true
fi

# ── Step 3: Directory structure ────────────────────────────────
# /var/lib/moyun-pos is also declared as StateDirectory= in the unit;
# creating it here lets --init-db run before the first service start.
echo ""
echo "▸ Setting up directories..."
mkdir -p "$POS_BIN_DIR/bin" "$POS_HOME"
chown "$POS_USER:$POS_USER" "$POS_HOME"
chmod 750 "$POS_HOME"
echo "  ✓ $POS_BIN_DIR and $POS_HOME ready"

# ── Step 4: Copy binary ───────────────────────────────────────
echo ""
echo "▸ Installing POS binary..."
install -m 755 "$BINARY_PATH" "$POS_BIN_DIR/moyun-pos"
install -D -m 755 "$SCRIPT_DIR/update.sh" "$POS_BIN_DIR/deploy/update.sh"
echo "  ✓ $("$POS_BIN_DIR/moyun-pos" --version) installed to $POS_BIN_DIR/moyun-pos"
echo "    SHA-1: $(sha1sum "$POS_BIN_DIR/moyun-pos" | awk '{print $1}')"

# ── Step 5: Initialize SQLite database ─────────────────────────
# `moyun-pos --init-db` creates the schema (same migrations as the
# app's boot path) and applies the seed. It refuses to seed a
# database that already holds fiscal events.
echo ""
echo "▸ Initializing database..."
INIT_ARGS=(--init-db)
if [[ -n "$SEED_PATH" ]]; then
    install -m 644 "$SEED_PATH" "$POS_HOME/.seed.sql"
    INIT_ARGS+=(--seed "$POS_HOME/.seed.sql")
fi
runuser -u "$POS_USER" -- env MOYUN_POS_DATA_DIR="$POS_HOME" \
    "$POS_BIN_DIR/moyun-pos" "${INIT_ARGS[@]}"
rm -f "$POS_HOME/.seed.sql"
echo "  ✓ Database ready at $POS_DB_PATH"

# ── Step 5b: POS configuration file ────────────────────────────
echo ""
echo "▸ Installing POS configuration..."
mkdir -p /etc/moyun-pos
if [[ ! -f /etc/moyun-pos/pos.json ]]; then
    cat > /etc/moyun-pos/pos.json <<'POSCONF'
{
  "pos_id": "CHANGE_ME_14CH",
  "terminal_id": "POS1",
  "device_id": "DEV001",
  "restaurant_name": "CHANGE ME",
  "restaurant_address": "CHANGE ME",
  "restaurant_vat": "BE0000000000",
  "restaurant_est_no": "0000000000",
  "fdm_url": "http://192.168.1.20/graphql",
  "printer_address": "192.168.1.30:9100",
  "training_mode": false,
  "language": "fr",
  "backup_dir": "/var/lib/moyun-pos/backups"
}
POSCONF
    chmod 640 /etc/moyun-pos/pos.json
    chown root:"$POS_USER" /etc/moyun-pos/pos.json
    echo "  ⚠ Template written to /etc/moyun-pos/pos.json — EDIT the"
    echo "    identity fields (pos_id, restaurant_*, fdm_url) and set"
    echo "    'language' to the establishment's linguistic region"
    echo "    (MANUEL_INSTALLATION_FR.md §4.2) before starting the service."
else
    echo "  ✓ /etc/moyun-pos/pos.json already present, left untouched"
fi

# ── Step 6: Kiosk session (systemd unit + PAM) ─────────────────
echo ""
echo "▸ Installing kiosk service..."
install -m 644 "$SCRIPT_DIR/moyun-pos.service" "$SERVICE_FILE"
install -m 644 "$SCRIPT_DIR/kiosk/pam.d-moyun-pos" /etc/pam.d/moyun-pos

# Earlier versions of this script set up tty1 autologin; the kiosk
# unit now owns tty1 directly.
rm -f /etc/systemd/system/getty@tty1.service.d/autologin.conf
rmdir /etc/systemd/system/getty@tty1.service.d 2>/dev/null || true

systemctl daemon-reload
systemctl set-default graphical.target
systemctl enable moyun-pos.service
echo "  ✓ moyun-pos.service enabled on tty1 (graphical.target)"

# ── Step 6b: Remote-support client (not started) ───────────────
echo ""
echo "▸ Installing RustDesk ${RUSTDESK_VERSION} (on-demand support)..."
RUSTDESK_TMP="$(mktemp)"
if [[ -n "$RUSTDESK_PATH" ]]; then
    cp "$RUSTDESK_PATH" "$RUSTDESK_TMP"
else
    curl -fsSL -o "$RUSTDESK_TMP" "$RUSTDESK_URL"
fi
echo "${RUSTDESK_SHA256}  ${RUSTDESK_TMP}" | sha256sum -c --quiet - \
    || { echo "ERROR: RustDesk checksum mismatch"; rm -f "$RUSTDESK_TMP"; exit 1; }
install -m 755 "$RUSTDESK_TMP" "$POS_BIN_DIR/bin/rustdesk"
rm -f "$RUSTDESK_TMP"
echo "  ✓ $POS_BIN_DIR/bin/rustdesk installed — no service, not started at boot"

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

# ── Step 8: Time sync + hardening ──────────────────────────────
echo ""
echo "▸ Hardening system..."
systemctl enable --now chrony
timedatectl set-timezone Europe/Brussels 2>/dev/null || true

# Disable services not needed on a kiosk POS
for svc in bluetooth cups avahi-daemon ModemManager ssh; do
    systemctl disable "$svc" 2>/dev/null || true
    systemctl stop "$svc" 2>/dev/null || true
done

# Disable automatic updates (controlled deployments only)
systemctl disable unattended-upgrades 2>/dev/null || true

# Mask apt timers — no unattended package activity on a fiscal
# appliance (GARANTIE_MAJ_LANGUES_FR.md §1.3: apt disabled via
# systemd-mask; updates go through deploy/update.sh only).
systemctl mask apt-daily.timer apt-daily-upgrade.timer 2>/dev/null || true

echo "  ✓ chrony enabled, unnecessary services disabled"

# ── Done ───────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════"
echo "  Installation complete!"
echo ""
echo "  Binary:   $POS_BIN_DIR/moyun-pos"
echo "  Database: $POS_DB_PATH"
echo "  Service:  moyun-pos.service (tty1)"
echo ""
echo "  Start:    sudo systemctl start moyun-pos"
echo "  Logs:     sudo journalctl -u moyun-pos -f"
echo "            $POS_HOME/logs/pos.log.*"
echo ""
echo "  BEFORE production use:"
echo "    1. Edit /etc/moyun-pos/pos.json (identity, FDM URL, language)."
echo "    2. Seal the appliance (GARANTIE_MAJ_LANGUES_FR.md §1.3):"
echo "       - root filesystem read-only: add 'ro' to the root line"
echo "         in /etc/fstab (keep $POS_HOME on a rw mount)."
echo "       - verify no SSH server: dpkg -l openssh-server → absent."
echo "    3. Updates: on site with deploy/update.sh from a USB stick, or"
echo "       remotely through an on-demand RustDesk session"
echo "       (deploy/REMOTE-OPS.md). No permanent remote channel."
echo "══════════════════════════════════════════════════"
