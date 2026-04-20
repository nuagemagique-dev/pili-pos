# Update Strategy — tsb-pos

## Problem

The POS runs on a single Debian terminal in the restaurant. To
deploy a bug fix or feature update, the current process is manual:
SSH → stop service → copy binary → restart. This doesn't scale
and requires the deployer to be online at the restaurant's
close-of-day.

## Recommended Approach: systemd pull + restart

The simplest MVP-viable strategy:

```
┌─────────────────────┐
│   GitHub Releases    │ ← CI builds a new tsb-pos binary on tag push
│   (or S3 bucket)     │
└──────────┬──────────┘
           │ https (Cloudflare)
           ▼
┌─────────────────────┐
│   POS Terminal       │
│   cron / systemd     │ ← Nightly check for new version
│   timer              │
│   ┌───────────────┐  │
│   │ update.sh     │  │ ← Downloads binary, verifies checksum,
│   │               │  │    swaps, restarts service
│   └───────────────┘  │
└─────────────────────┘
```

### update.sh (already exists in deploy/)

The existing `deploy/update.sh` can be extended to:

1. Fetch the latest release version from a manifest URL
   (e.g., `https://releases.tsb.nuagemagique.dev/pos/latest.json`)
2. Compare with the local version (`tsb-pos --version`)
3. If newer: download binary + sha256 checksum
4. Verify checksum
5. Stop service: `systemctl stop tsb-pos`
6. Swap binary: `mv tsb-pos.new /opt/tsb-pos/tsb-pos`
7. Restart: `systemctl start tsb-pos`
8. Log the update to `/var/log/tsb-pos-updates.log`

### Trigger

**Option A — Nightly cron (simplest)**:

```
# /etc/cron.d/tsb-pos-update
0 4 * * * root /opt/tsb-pos/deploy/update.sh >> /var/log/tsb-pos-updates.log 2>&1
```

Runs at 04:00 when the restaurant is closed. If no update is
available, the script exits silently.

**Option B — Dashboard-triggered**:

tsb-dashboard has a "Deploy" button that SSHs into the terminal
(same pattern as the current CI workflow for tsb-service) and runs
`update.sh`. Requires the terminal to be reachable via Cloudflare
Tunnel or a VPN.

**Option C — Tauri plugin (future)**:

`tauri-plugin-updater` supports signed auto-updates with a
built-in UI prompt. Requires:
- A release server (S3 + CloudFront + JSON manifest)
- Code-signing key pair
- Rust integration in `main.rs`

This is the most polished UX but is infrastructure-heavy. Defer
to a second iteration.

## Rollback

If an update breaks the POS:

1. SSH in
2. `systemctl stop tsb-pos`
3. `cp /opt/tsb-pos/tsb-pos.bak /opt/tsb-pos/tsb-pos`
   (update.sh should always keep the previous binary as `.bak`)
4. `systemctl start tsb-pos`

## Version Embedding

Add `--version` flag to `tsb-pos` by setting the version in
`tauri.conf.json` (already present: `"version": "0.1.0"`) and
exposing it via a Tauri command or CLI flag.

## Security

- Binaries MUST be signed (minisign, ed25519) and verified by
  update.sh before swap. Unsigned binaries = rejected.
- HTTPS only for the download (Cloudflare terminates TLS).
- The update script runs as root (needs to stop/start systemd
  services) — keep it minimal and auditable.
