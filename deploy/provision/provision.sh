#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - Provisioning (packages / image / instance)
#
# Split out of the old monolithic deploy/install.sh so the exact same
# logic provisions both:
#
#   - the mkosi kiosk image (deploy/image/mkosi.postinst.chroot calls
#     `--phase image` inside the build chroot: OFFLINE, no network, no
#     `systemctl --now`, no `nmcli` - only file installation and
#     `systemctl enable`/`mask`, which are symlink operations that work
#     without a running init or network, chroot-safe by design)
#
#   - a plain, already-booted Debian 13 machine (deploy/install.sh, the
#     thin wrapper next to this script, runs all three phases in order)
#
# Usage:
#   provision.sh --phase packages
#   provision.sh --phase image --variant admission|production --binary <path> \
#                [--seed <path>] [--rustdesk <path-to-AppImage>]
#   provision.sh --phase instance
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

POS_USER="pos"
POS_HOME="/var/lib/pili-pos"
POS_BIN_DIR="/opt/pili-pos"
FACTORY_DIR="/usr/share/pili-pos/factory"
SERVICE_FILE="/etc/systemd/system/pili-pos.service"

# Remote-support client (deploy/REMOTE-OPS.md): portable RustDesk, pinned.
# Extracted (never mounted/run as a live AppImage) so it works with no FUSE:
# see install_rustdesk() below.
RUSTDESK_VERSION="1.4.9"
RUSTDESK_URL="https://github.com/rustdesk/rustdesk/releases/download/${RUSTDESK_VERSION}/rustdesk-${RUSTDESK_VERSION}-x86_64.AppImage"
RUSTDESK_SHA256="7902cd60a4f29817eebe2668a15c9a1952ac690e8f7b07bfe7620fedd4e28217"

# Placeholder patched with the terminal's real NIC MAC once one can be
# read: never true at mkosi image-build time (the build chroot has no
# access to the target terminal's hardware), always true at --phase
# instance (a live machine) and at first boot (pili-provision.service,
# deploy/image/mkosi.extra/usr/local/lib/pili-pos/pili-provision - kept
# in sync with detect_device_id() below by hand, see the comment there).
DEVICE_ID_PLACEHOLDER="CHANGE_ME_MAC"

PHASE=""
VARIANT="production"
BINARY_PATH=""
SEED_PATH=""
RUSTDESK_PATH=""
FDM_MOCK_PATH=""

usage() {
    cat <<'USAGE'
Usage:
  provision.sh --phase packages
  provision.sh --phase image --variant admission|production --binary <path> \
               [--seed <path>] [--rustdesk <path>] [--fdm-mock <path>]
  provision.sh --phase instance
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --phase)    PHASE="$2"; shift 2 ;;
        --variant)  VARIANT="$2"; shift 2 ;;
        --binary)   BINARY_PATH="$2"; shift 2 ;;
        --seed)     SEED_PATH="$2"; shift 2 ;;
        --rustdesk) RUSTDESK_PATH="$2"; shift 2 ;;
        --fdm-mock) FDM_MOCK_PATH="$2"; shift 2 ;;
        -h|--help)  usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
    esac
done

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: provision.sh must run as root." >&2
    exit 1
fi

case "$VARIANT" in
    admission|production) ;;
    *) echo "ERROR: --variant must be admission or production (got: $VARIANT)" >&2; exit 1 ;;
esac

# ── device_id: the terminal's NIC MAC (manual §VI row VII, "Appareil:
#    <deviceId>" on every printed ticket) ──────────────────────────────
#
# Prints "aa:bb:cc:dd:ee:ff" (lowercase) on stdout, or exits non-zero with
# a message on stderr. Never called from phase_image(): a build chroot has
# no relevant hardware NIC. Called from phase_instance() below, and - a
# second, intentionally duplicated copy, since the two run in unrelated
# filesystem trees (build chroot vs. the booted kiosk image) - from
# pili-provision (deploy/image/mkosi.extra/usr/local/lib/pili-pos/pili-provision).
# Keep both copies in sync.
detect_device_id() {
    local iface candidate name mac

    # Prefer the interface carrying the default route.
    iface="$(ip -o route show default 2>/dev/null \
        | awk '!found {for (i=1;i<NF;i++) if ($i=="dev") {print $(i+1); found=1; break}}')"

    # Otherwise, the first non-loopback interface that has a MAC address.
    if [[ -z "$iface" ]]; then
        for candidate in /sys/class/net/*; do
            name="$(basename "$candidate")"
            [[ "$name" == "lo" ]] && continue
            if [[ -s "$candidate/address" ]]; then
                iface="$name"
                break
            fi
        done
    fi

    if [[ -z "$iface" ]]; then
        echo "ERROR: no network interface found to derive device_id from" >&2
        return 1
    fi
    if [[ ! -r "/sys/class/net/$iface/address" ]]; then
        echo "ERROR: /sys/class/net/$iface/address is not readable" >&2
        return 1
    fi

    mac="$(tr '[:upper:]' '[:lower:]' < "/sys/class/net/$iface/address")"
    if [[ -z "$mac" || "$mac" == "00:00:00:00:00:00" ]]; then
        echo "ERROR: interface $iface has no usable MAC address" >&2
        return 1
    fi
    echo "$mac"
}

# ── phase: packages ─────────────────────────────────────────────────
phase_packages() {
    echo "▸ [packages] apt-get install (needs network)…"
    apt-get update -qq
    mapfile -t pkgs < <(grep -vE '^[[:space:]]*(#|$)' "$SCRIPT_DIR/packages.txt")
    apt-get install -y -qq "${pkgs[@]}"
    echo "  ✓ ${#pkgs[@]} packages installed"
}

# ── phase: image (chroot-safe: no network, no --now, no nmcli) ─────────
install_rustdesk() {
    local tmp_appimage
    tmp_appimage="$(mktemp)"
    if [[ -n "$RUSTDESK_PATH" ]]; then
        cp "$RUSTDESK_PATH" "$tmp_appimage"
    else
        curl -fsSL -o "$tmp_appimage" "$RUSTDESK_URL"
    fi
    echo "${RUSTDESK_SHA256}  ${tmp_appimage}" | sha256sum -c --quiet - \
        || { echo "ERROR: RustDesk checksum mismatch" >&2; rm -f "$tmp_appimage"; exit 1; }
    chmod +x "$tmp_appimage"

    rm -rf "$POS_BIN_DIR/rustdesk" "$POS_BIN_DIR/.rustdesk-extract"
    mkdir -p "$POS_BIN_DIR/.rustdesk-extract"
    (
        cd "$POS_BIN_DIR/.rustdesk-extract"
        # --appimage-extract never mounts anything (no FUSE, no user
        # namespace), which is why it works under NoNewPrivileges=yes
        # (pili-pos.service does not run RustDesk, but the same
        # constraint applies inside an unprivileged build chroot).
        "$tmp_appimage" --appimage-extract >/dev/null
    )
    mv "$POS_BIN_DIR/.rustdesk-extract/squashfs-root" "$POS_BIN_DIR/rustdesk"
    rmdir "$POS_BIN_DIR/.rustdesk-extract"
    rm -f "$tmp_appimage"
    echo "  ✓ $POS_BIN_DIR/rustdesk/AppRun - no service, not started at boot"
}

install_admission_extras() {
    echo "▸ [image:admission] spf account + pos-fdm-mock…"
    if ! id spf &>/dev/null; then
        useradd --create-home --shell /bin/bash --groups pos spf
    fi
    # Locked until pili-provision.service sets a random password on
    # first boot and displays it once (deploy/image design).
    passwd -l spf 2>/dev/null || true

    if [[ -n "$FDM_MOCK_PATH" && -f "$FDM_MOCK_PATH" ]]; then
        install -m 755 "$FDM_MOCK_PATH" "$POS_BIN_DIR/bin/pos-fdm-mock"
    else
        echo "  ⚠ --fdm-mock not given - deploy/image/build.sh should stage and pass it" >&2
        exit 1
    fi

    # tty2 console login for the spf role (manual §II.E: SPF may want
    # console access on the admission machine). The password is set by
    # pili-provision.service on first boot, not here.
    systemctl enable getty@tty2.service 2>/dev/null || true

    # Installed by mkosi.extra; only enable it here (chroot-safe). Never
    # started - it is a manual fallback if the virtual FDM is unreachable
    # during the admission demo.
    if [[ -f /usr/lib/systemd/system/pos-fdm-mock.service || -f /etc/systemd/system/pos-fdm-mock.service ]]; then
        echo "  ✓ pos-fdm-mock.service installed, left disabled (manual start only)"
    fi
}

materialize_or_symlink_etc() {
    echo "▸ [image:$VARIANT] /etc/pili-pos…"
    if [[ -L /etc/pili-pos ]]; then
        echo "  ✓ symlink already in place (kiosk image layout - resolved by"
        echo "    pili-provision.service at first boot, see deploy/image/)"
        return
    fi
    # Plain Debian 13 install (no mkosi read-only-root/LUKS layout): make
    # /etc/pili-pos and /var/lib/pili-pos real directories now, seeded
    # from the same factory artifacts the kiosk image uses.
    mkdir -p /etc/pili-pos "$POS_HOME"
    if [[ ! -f /etc/pili-pos/pos.json ]]; then
        install -m 640 "$FACTORY_DIR/pos.json" /etc/pili-pos/pos.json
        chown root:"$POS_USER" /etc/pili-pos/pos.json
        echo "  ⚠ template written to /etc/pili-pos/pos.json - edit before starting the service"
    fi
    if [[ -d "$FACTORY_DIR/secrets" && ! -d /etc/pili-pos/secrets ]]; then
        install -d -m 750 -o root -g "$POS_USER" /etc/pili-pos/secrets
        for f in "$FACTORY_DIR"/secrets/*; do
            [[ -f "$f" ]] && install -m 640 -o root -g "$POS_USER" "$f" /etc/pili-pos/secrets/
        done
    fi
    # The FDM vendor's TLS files (fdm_ca_file, fdm_client_cert_file,
    # fdm_client_key_file in pos.json), copied here by the technician.
    install -d -m 750 -o root -g "$POS_USER" /etc/pili-pos/fdm
    if [[ ! -f "$POS_HOME/pos.db" ]]; then
        install -m 640 -o "$POS_USER" -g "$POS_USER" "$FACTORY_DIR/pos.db" "$POS_HOME/pos.db"
    fi
    chown -R "$POS_USER:$POS_USER" "$POS_HOME"
    chmod 750 "$POS_HOME"
    echo "  ✓ $POS_HOME and /etc/pili-pos ready"
}

phase_image() {
    if [[ -z "$BINARY_PATH" || ! -f "$BINARY_PATH" ]]; then
        echo "ERROR: --phase image needs --binary /path/to/pili-pos" >&2
        exit 1
    fi

    echo "▸ [image:$VARIANT] Creating POS user…"
    if ! id "$POS_USER" &>/dev/null; then
        useradd --system \
            --home-dir "$POS_HOME" \
            --shell /usr/sbin/nologin \
            --groups input,video,render \
            "$POS_USER"
    else
        usermod -aG input,video,render "$POS_USER" 2>/dev/null || true
    fi
    echo "  ✓ user '$POS_USER'"

    echo "▸ [image:$VARIANT] Installing binary and directories…"
    mkdir -p "$POS_BIN_DIR/bin" "$POS_BIN_DIR/deploy" "$FACTORY_DIR"
    install -m 755 "$BINARY_PATH" "$POS_BIN_DIR/pili-pos"
    # In the kiosk image build this script already runs from
    # $POS_BIN_DIR/deploy/provision/, so update.sh is in place.
    if [[ ! "$SCRIPT_DIR/../update.sh" -ef "$POS_BIN_DIR/deploy/update.sh" ]]; then
        install -D -m 755 "$SCRIPT_DIR/../update.sh" "$POS_BIN_DIR/deploy/update.sh"
    fi
    chmod 755 "$POS_BIN_DIR/deploy/update.sh"
    # Site bootstrap playbook (deploy/ansible): already in place in the
    # kiosk image build (deploy/image/build.sh stages it), copied from
    # the source tree on a plain Debian box.
    if [[ -d "$SCRIPT_DIR/../ansible" && ! "$SCRIPT_DIR/../ansible" -ef "$POS_BIN_DIR/deploy/ansible" ]]; then
        rm -rf "$POS_BIN_DIR/deploy/ansible"
        cp -R "$SCRIPT_DIR/../ansible" "$POS_BIN_DIR/deploy/ansible"
    fi
    echo "  ✓ $POS_BIN_DIR/pili-pos"

    echo "▸ [image:$VARIANT] Building the factory database…"
    INIT_ARGS=(--init-db)
    if [[ "$VARIANT" == "admission" && -z "$SEED_PATH" ]]; then
        echo "ERROR: --variant admission needs --seed <path> (the build chroot has" >&2
        echo "       no source tree - deploy/image/build.sh stages seed_spf_test.sql" >&2
        echo "       via mkosi.profiles/admission/mkosi.extra and passes its path)" >&2
        exit 1
    fi
    [[ -n "$SEED_PATH" ]] && INIT_ARGS+=(--seed "$SEED_PATH")
    PILI_POS_DATA_DIR="$FACTORY_DIR" "$POS_BIN_DIR/pili-pos" "${INIT_ARGS[@]}"
    echo "  ✓ $FACTORY_DIR/pos.db"

    echo "▸ [image:$VARIANT] Writing the pos.json template…"
    if [[ "$VARIANT" == "admission" ]]; then
        # The SPF test setup (UC_TestSetup_20240806): POS xx567 on the
        # BAR, the FPS Finances Experience, its 25 tables.
        POS_ID_TEMPLATE="CPOS0031234567"
        TERMINAL_ID_TEMPLATE="POS-1-BAR"
        TABLE_COUNT_TEMPLATE=25
        RESTAURANT_NAME_TEMPLATE="The FPS Finances Experience"
        RESTAURANT_ADDRESS_TEMPLATE="Boulevard Albert II 33, 1030 Brussels"
        RESTAURANT_VAT_TEMPLATE="BE0000000097"
        RESTAURANT_EST_NO_TEMPLATE="2000000042"
        # Digital ticket through the local mock (pos-fdm-mock serves
        # the Scaleway TEM API too), so email works without internet.
        DIGITAL_TICKET_TEMPLATE='
  "digital_ticket": {
    "project_id": "admission",
    "from_email": "tickets@admission.invalid",
    "from_name": "SPF Admission Test",
    "api_base": "http://127.0.0.1:8081",
    "api_key_file": "/etc/pili-pos/secrets/tem.key",
    "allow_digital_only": true
  },'
        install -d -m 750 "$FACTORY_DIR/secrets"
        printf 'admission-mock-key\n' > "$FACTORY_DIR/secrets/tem.key"
        chmod 640 "$FACTORY_DIR/secrets/tem.key"
    else
        POS_ID_TEMPLATE="CHANGE_ME_14CH"
        TERMINAL_ID_TEMPLATE="POS1"
        TABLE_COUNT_TEMPLATE=20
        RESTAURANT_NAME_TEMPLATE="CHANGE ME"
        # Invalid on purpose: the POS shows the invalid-configuration
        # banner until the installer enters the client's real numbers.
        RESTAURANT_ADDRESS_TEMPLATE="CHANGE ME"
        RESTAURANT_VAT_TEMPLATE="BE0000000000"
        RESTAURANT_EST_NO_TEMPLATE="0000000000"
        # Production: the digital ticket is configured at installation
        # (MANUEL_INSTALLATION_FR.md §4.2, « Ticket numérique par e-mail »); without it the POS is paper only.
        DIGITAL_TICKET_TEMPLATE=""
    fi
    cat > "$FACTORY_DIR/pos.json" <<POSCONF
{
  "pos_id": "$POS_ID_TEMPLATE",
  "terminal_id": "$TERMINAL_ID_TEMPLATE",
  "device_id": "$DEVICE_ID_PLACEHOLDER",
  "restaurant_name": "$RESTAURANT_NAME_TEMPLATE",
  "restaurant_address": "$RESTAURANT_ADDRESS_TEMPLATE",
  "restaurant_vat": "$RESTAURANT_VAT_TEMPLATE",
  "restaurant_est_no": "$RESTAURANT_EST_NO_TEMPLATE",
  "fdm_url": "http://192.168.1.20/graphql",
  "printer_address": "192.168.1.30:9100",
  "training_mode": false,
  "day_end_hour": 5,
  "table_count": $TABLE_COUNT_TEMPLATE,
  "language": "fr",$DIGITAL_TICKET_TEMPLATE
  "backup_dir": "/var/lib/pili-pos/backups"
}
POSCONF
    echo "  ✓ $FACTORY_DIR/pos.json (device_id patched at instance/first-boot time from the NIC MAC)"

    echo "▸ [image:$VARIANT] RustDesk ${RUSTDESK_VERSION} (extracted, no FUSE)…"
    install_rustdesk

    echo "▸ [image:$VARIANT] Kiosk session (systemd units + PAM)…"
    install -m 644 "$SCRIPT_DIR/../pili-pos.service" "$SERVICE_FILE"
    install -D -m 644 "$SCRIPT_DIR/../kiosk/pam.d-pili-pos" /etc/pam.d/pili-pos
    systemctl enable pili-pos.service
    systemctl set-default graphical.target
    echo "  ✓ pili-pos.service enabled (graphical.target)"

    echo "▸ [image:$VARIANT] Masking services never wanted on a kiosk…"
    systemctl mask bluetooth cups avahi-daemon ModemManager ssh \
        unattended-upgrades apt-daily.timer apt-daily-upgrade.timer 2>/dev/null || true
    echo "  ✓ masked (symlinks to /dev/null - chroot-safe, no daemon needed)"

    if [[ "$VARIANT" == "admission" ]]; then
        install_admission_extras
    fi

    materialize_or_symlink_etc

    echo "▸ [image:$VARIANT] First-boot provisioning + installer units…"
    # The unit files themselves ship via deploy/image/mkosi.extra; this
    # phase only enables what is already present in the tree (symlinks
    # only - `systemctl enable` works offline in a chroot, per mkosi's
    # documented behaviour for postinst.chroot scripts).
    for unit in pili-provision.service pili-installer.service pili-bootstrap.service systemd-repart.service; do
        # The admission (SPF test) machine runs the embedded binary and
        # the dossier identity only: no site bootstrap there.
        [[ "$unit" == pili-bootstrap.service && "$VARIANT" == admission ]] && continue
        if [[ -f "/usr/lib/systemd/system/$unit" || -f "/etc/systemd/system/$unit" ]]; then
            systemctl enable "$unit" 2>/dev/null || true
        fi
    done
    echo "  ✓ enabled where present (nothing started)"
}

# ── phase: instance (a live, booted machine - needs dbus + network) ────
phase_instance() {
    STATIC_IP="${STATIC_IP:-}"; GATEWAY="${GATEWAY:-}"; DNS="${DNS:-1.1.1.1}"; IFACE="${IFACE:-eth0}"

    echo "▸ [instance] Network…"
    if [[ -n "$STATIC_IP" && -n "$GATEWAY" ]]; then
        nmcli connection delete "pos-static" 2>/dev/null || true
        nmcli connection add type ethernet con-name "pos-static" ifname "$IFACE" \
            ipv4.addresses "$STATIC_IP" ipv4.gateway "$GATEWAY" ipv4.dns "$DNS" \
            ipv4.method manual connection.autoconnect yes
        nmcli connection up "pos-static"
        echo "  ✓ static IP $STATIC_IP on $IFACE"
    else
        echo "  ✓ DHCP (set STATIC_IP/GATEWAY/IFACE env vars for a static address)"
    fi

    echo "▸ [instance] Device ID (NIC MAC, manual §VI row VII)…"
    if [[ -f /etc/pili-pos/pos.json ]]; then
        if ! MAC="$(detect_device_id)"; then
            echo "ERROR: could not determine device_id - fix the network interface and rerun" >&2
            exit 1
        fi
        sed -i "s/\"device_id\": *\"[^\"]*\"/\"device_id\": \"$MAC\"/" /etc/pili-pos/pos.json
        echo "  ✓ device_id = $MAC"
    else
        echo "  ⚠ /etc/pili-pos/pos.json not found - run --phase image first"
    fi

    echo "▸ [instance] Fiscal identity (what the FDM would refuse)…"
    if "$POS_BIN_DIR/pili-pos" --check-config; then
        echo "  ✓ pos.json is a usable identity"
    else
        echo "  ⚠ fix /etc/pili-pos/pos.json, then rerun: $POS_BIN_DIR/pili-pos --check-config"
    fi

    echo "▸ [instance] Time sync + hardening…"
    systemctl enable --now chrony
    timedatectl set-timezone Europe/Brussels 2>/dev/null || true
    for svc in bluetooth cups avahi-daemon ModemManager ssh; do
        systemctl stop "$svc" 2>/dev/null || true
    done
    systemctl daemon-reload
    echo "  ✓ instance ready - start with: systemctl start pili-pos"
}

case "$PHASE" in
    packages) phase_packages ;;
    image)    phase_image ;;
    instance) phase_instance ;;
    *) echo "ERROR: --phase packages|image|instance is required" >&2; usage; exit 1 ;;
esac
