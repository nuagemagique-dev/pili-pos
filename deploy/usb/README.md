# Plug-and-play installation key

One USB key per site. Plug it into a new terminal and it installs the kiosk
image, configures the site and installs the declared POS version. Only two
manual steps are left.

## Make the key (Mac or Linux)

```bash
cp -R deploy/usb/site.example deploy/usb/sites/<site>   # git-ignored
$EDITOR deploy/usb/sites/<site>/site.yml                 # identity, IPs, FDM URL
cp fdm-token deploy/usb/sites/<site>/secrets/fdm/token   # see secrets/README.md
deploy/usb/make-key.sh --list
deploy/usb/make-key.sh v1.0.0 --site deploy/usb/sites/<site> /dev/disk4
```

You can pass a local `.img.xz` in place of `v1.0.0`. The image is built by
`deploy/image/build.sh production`.

## On the terminal

1. **Firmware, once per model.** Secure Boot **off**, TPM 2.0 **on**, USB
   first in the boot order.
2. **Plug the key in and power on.** The menu boots "Install" after 5 s.
   - On a blank disk, the install starts after 15 s. Any key cancels it.
   - A disk that already holds fiscal data (`pili-data`) always asks for
     `OUI`.
3. **Automatic reboot onto the internal disk.** The key can stay plugged in.
4. **First boot.** The LUKS2/TPM2 setup prints the root password and the
   recovery key once.
   - **Manual step:** write them in the installer's notebook, then press
     Enter.
5. **Site bootstrap.** `pili-bootstrap.service` runs Ansible
   (`deploy/ansible`):
   1. LAN address;
   2. `pos.json` and secrets (the secrets are then deleted from the
      terminal's ESP);
   3. the declared binary, pulled anonymously from the public package
      `ghcr.io/brunomoyse/moyun-pos`. It is installed only if the owner's
      signature and the SHA-1 match. Offline or in any doubt, the binary
      embedded in the image is kept;
   4. a health check of the FDM and printer.
   - A summary stays on screen for 60 s, then the kiosk starts.
6. **Manual step:** in the app, check the FDM status, print a test ticket
   and connect HubRise if the site uses it.

## Afterwards

- **Wipe the secrets** from the key once the site's terminals are
  installed: `deploy/usb/make-key.sh --wipe-secrets /dev/disk4`.
- **Bootstrap log and report:** `/var/log/pili-bootstrap.log` and
  `/var/lib/pili-pos/bootstrap-report.txt`.
- **Re-run** from the maintenance console (Ctrl+Alt+F2, root):
  - `pili-bootstrap --rerun`: applies the site again and the current
    declared version;
  - `pili-bootstrap --rerun --offline`: site only;
  - `pili-bootstrap --rerun --site /path`: another site directory.

## Releases

- **Where things live.**
  - The code and the GitHub Releases stay **private** in `moyun-pos`.
  - Terminals only read the **public** package `ghcr.io/brunomoyse/moyun-pos`,
    with no token.
- **On a `vX.Y.Z` tag**, `.github/workflows/release.yml`:
  - opens a **prerelease** with the tarball, the kiosk image and
    `build_info` (the SHA-1 to declare);
  - pushes the tarball to the package as tag `vX.Y.Z`.
- **No terminal installs a pending version.**
- **After SPF approval**, run on the Mac:

  ```bash
  scripts/release/declare.sh vX.Y.Z --sha1 <SHA-1 accepted by SPF>
  ```

  It signs `declared.json` with the offline minisign key, then:
  - pushes the tarball + `declared.json` + signature as the package's
    `declared` tag;
  - turns the prerelease into the latest release.

  From then on, new terminals install that version.
- **One-time setup:**
  1. `scripts/release/declare.sh --init-key`, then commit
     `deploy/release/declared.pub`;
  2. `brew install oras minisign`;
  3. `gh auth refresh -s write:packages`;
  4. after the first tag's push, make the package **public** (package
     settings > Change visibility).

## Testing

`deploy/ansible/tests/run.sh` tests the playbook in a Debian 13 container
(Docker). It uses a fake binary, a mock of ghcr.io's anonymous pull API and a throwaway
key, and covers 8 cases: update, re-run, tampered manifest, wrong SHA-1,
downgrade, offline, unreachable server, and no site.
