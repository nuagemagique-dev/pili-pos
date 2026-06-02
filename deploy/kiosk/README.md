# Kiosk Lockdown

Config files for hardening the POS terminal in production
(Penetek A3216 M82, Debian 12 minimal, Cage compositor).

## Files

| File | Destination | Purpose |
|------|-------------|---------|
| `cage.ini` | `/etc/cage/config` | Disable all keybindings (Alt+Tab, Ctrl+Alt+Backspace, Super) so the operator can't escape the POS window |
| `99-block-hid.rules` | `/etc/udev/rules.d/` | Block USB HID keyboards and mice so a physically plugged device can't access a shell |

## Installation

```bash
sudo cp cage.ini /etc/cage/config
sudo cp 99-block-hid.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## Additional recommendations

1. **No shell accessible without SSH**: remove `getty` from
   systemd targets, or password-protect it with a strong passphrase.
2. **SSH key-only**: disable password auth in `/etc/ssh/sshd_config`.
3. **Firewall**: allow only SSH (22) + HubRise outbound (443) +
   FDM LAN (8080) + printer LAN (9100). Block everything else.
4. **Auto-start**: the existing `moyun-pos.service` systemd unit
   handles this — ensure it's `WantedBy=graphical.target`.
5. **LUKS FDE**: encrypt the root partition with LUKS + TPM2
   auto-unlock (see hardening #7 in the audit).
