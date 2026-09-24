# Kiosk Lockdown

How the POS terminal (Debian 12 minimal, Cage compositor) is locked
down. `deploy/install.sh` applies all of it.

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
  RustDesk session (`/opt/moyun-pos/bin/rustdesk`, pinned AppImage,
  never started at boot) — see `../REMOTE-OPS.md`.
- **BIOS password** and boot order locked to the internal disk
  (`MANUEL_INSTALLATION_FR.md` §3.1).

USB keyboards are deliberately *not* blocked: the maintenance procedure
and the SPF agent's checks at admission rely on plugging one in.

## Still to do in the kiosk image

1. Read-only root filesystem, `/var/lib/moyun-pos` on a separate rw
   mount (`GARANTIE_MAJ_LANGUES_FR.md` §1.3).
2. LUKS full-disk encryption with TPM2 auto-unlock.
3. Firewall: allow outbound LAN traffic to the FDM and printer only,
   plus outbound 443 while a RustDesk session is running.
