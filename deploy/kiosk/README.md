# Kiosk Lockdown

How the POS terminal (Debian 12 minimal, Cage compositor) is locked
down. Applied either by `deploy/install.sh` (thin wrapper, plain
already-booted Debian 12 box) or baked into the mkosi kiosk image
(`deploy/image/`, `mkosi.postinst.chroot`) — both call the same
`deploy/provision/provision.sh`, see that script's header.

## Files

| File | Destination | Purpose |
|------|-------------|---------|
| `pam.d-moyun-pos` | `/etc/pam.d/moyun-pos` | PAM stack for `moyun-pos.service` (`PAMName=moyun-pos`): opens the logind session Cage needs on tty1 |

## What keeps the operator inside the POS

- **Cage** runs `moyun-pos` fullscreen with no window decorations and
  no shell or launcher. Cage has no configuration file and no
  keybindings besides VT switching; the unit passes `-s` so the
  distributor can reach the maintenance console.
- **tty1** is owned by the kiosk unit (`Conflicts=getty@tty1.service`):
  no login prompt behind the POS.
- **Maintenance console** — `Ctrl+Alt+F2` with a USB keyboard gives a
  getty login. Only `root` (random password generated at install,
  kept by the distributor) and the SPF test account on the admission
  machine can log in; `pos` has no password and a `nologin` shell.
- **No SSH server**, no listening port. Remote support is an on-demand
  RustDesk session (pinned AppImage, verified then extracted with
  `--appimage-extract` into `/opt/moyun-pos/rustdesk/` — launched via
  `/opt/moyun-pos/rustdesk/AppRun`, never a service, never started at
  boot) — see `../REMOTE-OPS.md`.
- **BIOS password** and boot order locked to the internal disk
  (`MANUEL_INSTALLATION_FR.md` §3.1). Software boot lockdown also
  applies: the ESP + root partitions are read-only (below) and the
  root disk is unlocked only via this terminal's own TPM2 chip or the
  distributor's recovery key (`SECURITE_BD_FR.md` §3.6) — an attacker
  who defeats the BIOS password still cannot read `pos.db` off the
  bare disk.

USB keyboards are deliberately *not* blocked: the maintenance procedure
and the SPF agent's checks at admission rely on plugging one in.

## Kiosk image (`deploy/image/`, mkosi)

Done, as of this branch:

1. **Read-only root filesystem** — `Format=disk` build (systemd-boot +
   UKI, ESP 512M + a 4G ext4 root marked `ReadOnly=yes`); `/var` is a
   separate partition.
2. **LUKS2 full-disk encryption of `/var` with TPM2 auto-unlock** —
   first-boot `moyun-provision.service`
   (`mkosi.extra/usr/local/lib/moyun-pos/moyun-provision`): temporary
   key → `luksFormat` → enroll a TPM2 token (`--tpm2-pcrs=7`) + a
   recovery key → wipe the temporary key. See `SECURITE_BD_FR.md` §3.6
   for what this does and does not protect against (Secure Boot stays
   **disabled** — no UKI signing pipeline exists yet — so the PCR 7
   binding is weaker than it would be with Secure Boot on; the
   recovery key, recorded once in the distributor's installation
   notebook, is the fallback).
3. **USB installer/updater** — the same image, booted from a USB key,
   exposes "Install" and "Update" systemd-boot entries
   (`moyun.installer=install|update`) built from the exact same
   kernel+initrd via UKI section-splicing (`deploy/image/build.sh`).

## Still to do

- Firewall: allow outbound LAN traffic to the FDM and printer only,
  plus outbound 443 while a RustDesk session is running.
- A real Secure Boot signing pipeline (own MOK, `sbsign` the UKI,
  enroll the key in each terminal's firmware) so the TPM2 PCR 7
  binding gives its full guarantee.
- A real boot test of `moyun-provision.service` on physical hardware
  with a real TPM2 chip (only QEMU+swtpm has exercised this so far,
  and even that boot-smoke path has not been observed to complete the
  full first-boot wizard end-to-end — see the Phase 4 final report).
