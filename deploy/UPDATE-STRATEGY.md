# Update Strategy - moyun-pos

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

## Deploying to a terminal

`deploy/update.sh <binary> --sha1 <declared SHA-1>` on the maintenance
console. It refuses a binary whose SHA-1 differs from the declared
value, backs up the binary and the database, applies schema migrations
offline (`moyun-pos --init-db`), and restarts the kiosk.

The binary reaches the terminal either:

- **on site** - USB stick, maintenance console (`Ctrl+Alt+F2`); or
- **remotely** - RustDesk file transfer during an on-demand session
  opened by the manager on site.

Schedule updates outside service hours, after the day's Z report.

## Rollback

`update.sh` keeps the previous binary as `/opt/moyun-pos/moyun-pos.bak`
(also a certified version). Rolling back swaps the binary only; the
database is never restored from a backup, since that would drop fiscal
events recorded since.

## Later: whole-image updates

Once the kiosk image is built with a read-only root (mkosi), updates
move to A/B image partitions (`systemd-sysupdate`): the new image is
written to the inactive slot and verified before reboot, so an update
never leaves a half-installed system. The SHA-1 check on the
`moyun-pos` binary stays the acceptance gate.
