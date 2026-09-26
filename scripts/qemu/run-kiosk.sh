#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos - boot a kiosk image (deploy/image/build.sh output) in QEMU
#
# q35 + UEFI (OVMF) + swtpm (TPM2, needed by moyun-provision.service's
# systemd-cryptenroll --tpm2-device=auto), virtio-vga at 1920x1080, a
# touch-capable pointer, and QEMU usermode networking so the guest can
# reach:
#   - the mock FDM on the HOST at 10.0.2.2:8081 (crates/pos-fdm-mock;
#     point pos.json::fdm_url at http://10.0.2.2:8081/graphql)
#   - a fake printer: this script itself listens with `nc -l 9100` on
#     the host and dumps whatever the guest sends to a log file (point
#     pos.json::printer_address at 10.0.2.2:9100)
#
# Runs under KVM on Linux (if /dev/kvm exists) or TCG on macOS -
# noticeably slower there, but the only option on Apple Silicon /
# without nested virtualization.
#
# The image file is never modified: a working copy (+8G of free space,
# for systemd-repart to carve the "moyun-data" partition out of on
# first boot) is made under .run/<name>/ and thrown away with --clean.
#
# Usage:
#   run-kiosk.sh <image.raw|image.img.xz> [--name NAME] [--pos-json FILE]
#                [--ram MB] [--clean]
#
# --pos-json FILE: injected via -fw_cfg name=opt/moyun/pos.json - read
#   by moyun-provision.service on FIRST boot only (i.e. only while no
#   pos.json exists yet at /var/lib/moyun-etc/moyun-pos/pos.json); a
#   later run against the same working copy sees the fw_cfg entry
#   present but ignores it, matching a real terminal where nobody can
#   inject config after the fact. Mainly useful for
#   scripts/qemu/screendump.sh runbooks that need a specific identity
#   already configured before the first screenshot.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RUN_ROOT="$SCRIPT_DIR/.run"

IMAGE=""
NAME=""
POS_JSON=""
RAM_MB=4096
CLEAN=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --name)     NAME="$2"; shift 2 ;;
        --pos-json) POS_JSON="$2"; shift 2 ;;
        --ram)      RAM_MB="$2"; shift 2 ;;
        --clean)    CLEAN=1; shift ;;
        -h|--help)
            sed -n '2,40p' "$0"; exit 0 ;;
        *)
            if [[ -z "$IMAGE" ]]; then IMAGE="$1"; shift; else
                echo "Unknown argument: $1" >&2; exit 1
            fi ;;
    esac
done

if [[ -z "$IMAGE" || ! -f "$IMAGE" ]]; then
    echo "Usage: $0 <image.raw|image.img.xz> [--name NAME] [--pos-json FILE] [--ram MB] [--clean]" >&2
    exit 1
fi
[[ -n "$NAME" ]] || NAME="$(basename "$IMAGE" | sed -E 's/\.(img\.xz|raw)$//')"

WORK_DIR="$RUN_ROOT/$NAME"
if [[ "$CLEAN" -eq 1 ]]; then
    rm -rf "$WORK_DIR"
fi
mkdir -p "$WORK_DIR"

DISK="$WORK_DIR/disk.raw"
if [[ ! -f "$DISK" ]]; then
    echo "▸ Preparing disk (source + 8G free space for moyun-data)…"
    case "$IMAGE" in
        *.img.xz) xz -dc "$IMAGE" > "$DISK" ;;
        *.raw)    cp "$IMAGE" "$DISK" ;;
        *) echo "ERROR: expected a .raw or .img.xz image" >&2; exit 1 ;;
    esac
    truncate -s +8G "$DISK"
fi

# ── Firmware (OVMF) ─────────────────────────────────────────────
find_first() {
    local p
    for p in "$@"; do
        [[ -f "$p" ]] && { echo "$p"; return 0; }
    done
    return 1
}

OVMF_CODE="$(find_first \
    "$REPO_ROOT/deploy/image/mkosi.tools/usr/share/OVMF/OVMF_CODE_4M.fd" \
    "$REPO_ROOT/deploy/image/mkosi.tools/usr/share/OVMF/OVMF_CODE.fd" \
    /usr/share/OVMF/OVMF_CODE_4M.fd \
    /usr/share/OVMF/OVMF_CODE.fd \
    /usr/share/edk2/x64/OVMF_CODE.fd \
    /opt/homebrew/share/qemu/edk2-x86_64-code.fd \
    /usr/local/share/qemu/edk2-x86_64-code.fd \
)" || { echo "ERROR: OVMF CODE firmware not found (install 'ovmf' or 'qemu')" >&2; exit 1; }

OVMF_VARS_SRC="$(find_first \
    "$REPO_ROOT/deploy/image/mkosi.tools/usr/share/OVMF/OVMF_VARS_4M.fd" \
    "$REPO_ROOT/deploy/image/mkosi.tools/usr/share/OVMF/OVMF_VARS.fd" \
    /usr/share/OVMF/OVMF_VARS_4M.fd \
    /usr/share/OVMF/OVMF_VARS.fd \
    /usr/share/edk2/x64/OVMF_VARS.fd \
    /opt/homebrew/share/qemu/edk2-i386-vars.fd \
    /usr/local/share/qemu/edk2-i386-vars.fd \
)" || { echo "ERROR: OVMF VARS firmware not found" >&2; exit 1; }

OVMF_VARS="$WORK_DIR/OVMF_VARS.fd"
[[ -f "$OVMF_VARS" ]] || cp "$OVMF_VARS_SRC" "$OVMF_VARS"

# ── Acceleration ─────────────────────────────────────────────────
if [[ "$(uname -s)" == "Linux" && -e /dev/kvm ]]; then
    ACCEL="kvm"
    CPU="host"
else
    ACCEL="tcg"
    CPU="max"
    echo "▸ No KVM - using TCG software emulation (slower)."
fi

# ── TPM2 (swtpm) ─────────────────────────────────────────────────
command -v swtpm >/dev/null 2>&1 || { echo "ERROR: swtpm not found" >&2; exit 1; }
TPM_STATE_DIR="$WORK_DIR/tpm"
mkdir -p "$TPM_STATE_DIR"
TPM_SOCK="$WORK_DIR/swtpm.sock"
rm -f "$TPM_SOCK"
swtpm socket --tpmstate "dir=$TPM_STATE_DIR" \
    --ctrl "type=unixio,path=$TPM_SOCK" \
    --tpm2 -d
for _ in $(seq 1 50); do
    [[ -S "$TPM_SOCK" ]] && break
    sleep 0.1
done
[[ -S "$TPM_SOCK" ]] || { echo "ERROR: swtpm did not create its control socket" >&2; exit 1; }

# ── Touch/pointer device: prefer virtio-multitouch, else usb-tablet ─
TOUCH_ARGS=(-device usb-ehci,id=usb-bus -device usb-tablet,bus=usb-bus.0)
if qemu-system-x86_64 -device help 2>/dev/null | grep -q 'virtio-multitouch-pci'; then
    TOUCH_ARGS=(-device virtio-multitouch-pci)
fi

# ── Printer sink: `nc -l 9100` on the host, guest reaches it at
#    10.0.2.2:9100 through QEMU usermode NAT. ────────────────────────
PRINTER_LOG="$WORK_DIR/printer-9100.log"
PRINTER_PID=""
if command -v nc >/dev/null 2>&1; then
    ( while true; do nc -l 9100 >> "$PRINTER_LOG" 2>/dev/null || true; done ) &
    PRINTER_PID=$!
    echo "▸ Printer sink: nc -l 9100 -> $PRINTER_LOG (pid $PRINTER_PID)"
else
    echo "▸ 'nc' not found - no printer sink; ESC/POS output will be refused by the guest."
fi

QMP_SOCK="$WORK_DIR/qmp.sock"
rm -f "$QMP_SOCK"

FW_CFG_ARGS=()
if [[ -n "$POS_JSON" ]]; then
    [[ -f "$POS_JSON" ]] || { echo "ERROR: --pos-json file not found: $POS_JSON" >&2; exit 1; }
    FW_CFG_ARGS=(-fw_cfg "name=opt/moyun/pos.json,file=$POS_JSON")
fi

cleanup() {
    [[ -n "$PRINTER_PID" ]] && kill "$PRINTER_PID" 2>/dev/null || true
}
trap cleanup EXIT

# The admission image copies its journal to ttyS0
# (journald.conf.d/90-moyun-serial.conf); scripts/qemu/boot-check.sh reads it.
SERIAL_LOG="$WORK_DIR/serial.log"
: > "$SERIAL_LOG"

echo "▸ QMP socket: $QMP_SOCK (scripts/qemu/screendump.sh $NAME)"
echo "▸ Serial console (journal): $SERIAL_LOG"
echo "▸ Booting '$NAME' ($ACCEL)…"

exec qemu-system-x86_64 \
    -name "moyun-pos-$NAME" \
    -machine "q35,accel=$ACCEL" \
    -cpu "$CPU" \
    -smp 4 \
    -m "$RAM_MB" \
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
    -drive "if=pflash,format=raw,file=$OVMF_VARS" \
    -drive "if=virtio,format=raw,file=$DISK" \
    -device virtio-vga,xres=1920,yres=1080 \
    "${TOUCH_ARGS[@]}" \
    -netdev user,id=net0 \
    -device virtio-net-pci,netdev=net0 \
    -chardev "socket,id=chrtpm,path=$TPM_SOCK" \
    -tpmdev emulator,id=tpm0,chardev=chrtpm \
    -device tpm-tis,tpmdev=tpm0 \
    -qmp "unix:$QMP_SOCK,server,nowait" \
    -serial "file:$SERIAL_LOG" \
    "${FW_CFG_ARGS[@]}" \
    -display none -vnc :0
