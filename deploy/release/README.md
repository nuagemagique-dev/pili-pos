# Release signing key

`declared.pub` is the public half of the owner's minisign key. It is created by
`scripts/release/declare.sh --init-key`.

- `deploy/image/build.sh` embeds it in the production image at
  `/usr/share/pili-pos/declared.pub`.
- On first boot, the site bootstrap (`deploy/ansible/roles/pos_binary`) checks
  `declared.json` against it before installing a downloaded binary.
- If the file is missing, the image never fetches anything. It keeps the
  binary it was built with.

The secret key never enters the repository or CI. It lives in
`~/.minisign/pili-pos-declared.key` on the owner's machine, protected by a
password, with an offline backup.
