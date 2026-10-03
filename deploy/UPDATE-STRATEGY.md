# Update Strategy - pili-pos

## Constraints

- **Every modification of the POS requires an update certification
  request to SPF Finances** (certification manual §II.E), tested on the
  SPF test machine. A terminal may only run a binary whose SHA-1 was
  declared for that version.
- The terminal is a sealed appliance: no SSH server, no listening port,
  no unattended package activity, read-only root filesystem (target).
- Remote access is an on-demand RustDesk session only
  (`REMOTE-OPS.md`); there is no permanent remote channel.

A self-updating terminal (nightly pull from a release server) is
therefore ruled out: it would install binaries without an operator
checking that the SPF accepted them, and it needs a permanent outbound
channel.

## Release flow

1. Tag `vX.Y.Z` (GPG-signed) on the certified commit.
2. `deploy/build-release.sh --docker` builds the cert binary in a
   linux/amd64 Debian 13 container with the toolchain pinned by
   `rust-toolchain.toml`, and writes the `build_info_*.txt` report
   (SHA-1 / SHA-256).
3. File the update certification request with the changelog and the
   new SHA-1; wait for SPF approval.
4. Deploy to terminals.

## Publishing and declaring (GitHub Releases)

- The `vX.Y.Z` tag runs `.github/workflows/release.yml`. It does the same
  reproducible build and image as `image.yml`, then creates a **prerelease**
  on the releases repository. The prerelease holds:
  - `pili-pos-vX.Y.Z-x86_64.tar.gz`;
  - `pili-pos-kiosk-vX.Y.Z.img.xz`;
  - `build_info-vX.Y.Z.txt`;
  - `SHA256SUMS`.
- Once SPF has accepted the SHA-1, the owner runs
  `scripts/release/declare.sh vX.Y.Z --sha1 <SHA-1>`.
  - It signs `declared.json` (`version`, `sha1`, `sha256`, `asset_url`)
    with an offline minisign key.
  - It publishes the manifest on the fixed `declared` release and marks the
    version as latest.
- New terminals read that manifest **once**, at the first-boot site
  bootstrap (`deploy/usb/README.md`, `deploy/ansible/roles/pos_binary`).
  They install only a signed, SHA-1-matching, not-older binary, through
  `update.sh`. Otherwise they keep the binary embedded in the image.
- This is not a self-update. It runs once, with the installer present.
  Later updates stay manual: `pili-bootstrap --rerun` on the maintenance
  console uses the same checks.

## Deploying to a terminal

`deploy/update.sh <binary> --sha1 <declared SHA-1>` on the maintenance
console. It refuses a binary whose SHA-1 differs from the declared
value, backs up the binary and the database, applies schema migrations
offline (`pili-pos --init-db`), and restarts the kiosk.

The binary reaches the terminal either:

- **on site** - USB stick, maintenance console (`Ctrl+Alt+F2`); or
- **remotely** - RustDesk file transfer during an on-demand session
  opened by the manager on site.

Schedule updates outside service hours, after the day's Z report.

## Rollback

`update.sh` keeps the previous binary as `/opt/pili-pos/pili-pos.bak`
(also a certified version). Rolling back swaps the binary only; the
database is never restored from a backup, since that would drop fiscal
events recorded since.

## Later: whole-image updates

Once the kiosk image is built with a read-only root (mkosi), updates
move to A/B image partitions (`systemd-sysupdate`): the new image is
written to the inactive slot and verified before reboot, so an update
never leaves a half-installed system. The SHA-1 check on the
`pili-pos` binary stays the acceptance gate.
