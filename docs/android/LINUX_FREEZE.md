# Linux soft freeze: linux-v1.1.0-rc1

The soft freeze of the certified Linux build (plan P1b, PD44) after the
user's two decisions of 2026-10-08 (PD47, PD71). From this tag on, the
Linux build inputs that `scripts/android/check_linux_frozen.sh` guards do
not change while the Android port lands; a change there is a Linux v1.2
change and needs the user.

## Tag

| Item | Value |
|------|-------|
| tag | `linux-v1.1.0-rc1` (local annotated tag, set by the orchestrator after review, never pushed by the chain) |
| tag target | the final commit of branch `android/freeze-rc1`, the commit that adds this file |
| built commit | `8dec93a98d4e307b1a4fb466799833bf29905d3e` (`fix(ui): a refused catalog resync says so instead of reporting a sync`) |
| branch base | `android/int` at `623c90b` (`chore(android): merge P3-T7`) |
| version | 1.1.0 (`Cargo.toml` `[workspace.package]`, `src-tauri/tauri.conf.json`, `ui/package.json`) |

The tag target adds only `docs/android/LINUX_FREEZE.md` and
`docs/android/DECISIONS.md` on top of the built commit. Neither is a
build input: the freeze paths are identical (`git diff 8dec93a <tag> --
Cargo.toml Cargo.lock rust-toolchain.toml src-tauri ui deploy/build-release.sh
scripts/build_and_hash.sh crates` is empty) and the binary embeds no git
metadata, so the tag target builds the same binary. Only the `Git commit`
and `Git tag` lines of a `build_info_*.txt` report name the commit.

## Reproducible build

`scripts/check_repro.sh` ran on 2026-10-08 from a clean local clone of
`8dec93a` (`~/.cache/pili-freeze/clone`, outside the worktrees: Docker in
the build container cannot follow a worktree's `.git` file). It runs
`deploy/build-release.sh --docker` twice, deleting the build outputs
between the runs, in a linux/amd64 container under emulation on the
macOS host (Docker Desktop, kernel `Linux 7.0.14-linuxkit x86_64`). Both
runs passed all five steps of `scripts/build_and_hash.sh`, the mutation
scan of step 3/5 included (PD47).

| | run 1 | run 2 |
|---|---|---|
| report | `build_info_20261008-101151.txt` | `build_info_20261008-103126.txt` |
| SHA-1 | `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e` | `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e` |
| SHA-256 | `01ac4abf50c63fb44692bb2f9618dc12cec50fe089091bac44e7aa43886dfc57` | `01ac4abf50c63fb44692bb2f9618dc12cec50fe089091bac44e7aa43886dfc57` |
| MD5 | `daa2d60fcad633ef18c55ab4e66e6c06` | `daa2d60fcad633ef18c55ab4e66e6c06` |
| size | 38962336 bytes | 38962336 bytes |
| working tree | clean | clean |

Result: `✓ Reproducible: SHA-1 9d230ef1bba90a4edde0157bcb107e4bb4d23d6e`
(exit 0). The reports said `Git tag: (untagged)`: the tag is set after
the build.

Checks of both reports: HubRise integration present (installation
switch, off by default), 0 `pos_peppol` symbols, no `fiscal_events`
DELETE/UPDATE in the source compiled into the binary.

## Build inputs

| Item | Value |
|------|-------|
| `Cargo.lock` sha256 | `129aa3afd07b5869217026697d7deb6d87ae23b8f7307d251a273c0294cb7cdf` |
| rustc (in the container, both reports) | `rustc 1.99.0 (b940084d7 2026-09-28)` |
| `rust-toolchain.toml` | `channel = "1.99.0"`, components rustfmt, clippy |
| build image | `rust:1.99.0-trixie@sha256:9d5e02aa6c7e9c112ed7a4c438900b41f519a792959cd5445c25de42bfb8388b` (`deploy/build-release.sh`) |
| Node.js | `v24.21.0`, tarball sha256 `fd8e59d5a511510f6a298afb548f18c7d2b1be404d8b4a27d94fbe49f56cb2d6` |
| build command | `cargo build --release --no-default-features --features cert` |

The baseline Linux SHA-1 (P0.1, main `96728e1`, version 1.0) was
`83313be28027d2e8d9e84cbddff4e584799c2fbc`; the 1.1.0 binary differs, as
expected (engine extraction, version 1.1.0, the P1b rules and the PD71 fix).

## Decisions by the user (2026-10-08)

1. **Mutation scan scope (PD47).** Step 3/5 of `scripts/build_and_hash.sh`
   scans the source compiled into the cert binary, as v1.0 did in
   practice: every `*.rs` and `*.sql` file of `crates` and `src-tauri/src`,
   minus the Cargo integration-test directories (a `tests` directory
   directly under a crate root, so `crates/*/tests`) and the
   `#[cfg(test)]` blocks. No allowlist of lines. The dossier sentence
   (`GARANTIE_MAJ_LANGUES_FR.md` section 1.2) and the manual check of
   `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md` say the same. Commit `228c562`.
2. **Rejected catalog outcome (PD71), fixed before the freeze.**
   `pos_catalog::ApplyOutcome::Rejected` is a struct variant,
   `Rejected { issues }`, serialized as
   `{"outcome": "rejected", "issues": [...]}`, so `hubrise_sync_catalog`
   and `hubrise_adopt_catalog` answer the rejection instead of a
   serialization error. Commits `2e1e3ad` (engine and FFI) and `8dec93a`
   (UI).

## Commits of the freeze branch

| Commit | Subject |
|--------|---------|
| `228c562` | fix(cert): mutation scan covers the source compiled into the cert binary |
| `2e1e3ad` | fix(catalog): a rejected catalog outcome serializes with its issues |
| `8dec93a` | fix(ui): a refused catalog resync says so instead of reporting a sync |
| (tag target) | docs(android): Linux soft freeze record and the user's decisions on PD47 and PD71 |

## Checking the freeze

```bash
scripts/android/check_linux_frozen.sh            # against linux-v1.1.0-rc1 once tagged
scripts/check_repro.sh                           # from a clean clone of the tag, never a worktree
```

The hard freeze `linux-v1.1.0` (signed by the user) is still to come.

## P4 exit check (2026-10-08)

At the end of P4 (`android/int` at `96c99f1`, `chore(android): merge P4-T12 restart-check fixes`):

- `scripts/android/check_linux_frozen.sh` prints `frozen against linux-v1.1.0-rc1 (65b567f): 16 paths, clean`, and `git diff --quiet 30b32a0 -- <freeze paths, android/rust, android/smoke, android/docker, android/pos-core sources, .github>` is empty: P4 changed no Linux build input.
- `scripts/android/p4_gate.sh --fast --linux-repro --base 30b32a0` (integrator, same commit) built the tag twice in Docker from a clean clone and reproduced SHA-1 `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e`.
- No tag was set. The plan requires no freeze step at the end of P4, and the plan's update of 2026-10-08 renames the freeze: the local tag `linux-v1.1.0-rc1` was misnamed, and the freeze is redone as `linux-v1.0.0-rc1` after the nightly export feature is merged. That feature is not merged yet and a re-freeze needs the user (PD121). Until then the chain keeps checking `linux-v1.1.0-rc1`.

## X1 exit check (2026-10-09, export lane)

At the end of X1 (`android/lane-export` at `a364ad7`, `docs(android): record the X1 wave 6 integration`; fork point `ba57adf`):

- The Linux freeze was reopened for this lane only (user, 2026-10-08; X1D2, X1D3). `check_linux_frozen.sh` against `linux-v1.1.0-rc1` fails here by design: the export lane changed `Cargo.toml`, `Cargo.lock`, `crates/` (new `pos-export`, `pos-export-mock`, engine glue), `src-tauri/` (two wrappers) and `ui/` (the export panel). `x1_gate.sh` waives exactly the six freeze checks and checks the scope instead (`x1-scope`, `x1-lock-additive`).
- `x1_gate.sh --fast --linux-repro` (integrator, integrate-6, same commit) built the Linux cert binary twice in Docker from a clean clone: SHA-1 `d4fb09a2ae2cf2501fe4d6847eb72e954d0b95ed`, two identical builds. It needed `rust-embed` with `deterministic-timestamps` on pos-export (`dc29f43`, X1D20); keep it.
- No tag was set (X1D27). The version is still 1.1.0. The re-freeze as `linux-v1.0.0-rc1` (and the rename to 1.0.0 if wanted) is the user's, after `android/lane-export` is merged into `android/int` and the Linux build is reproduced there again. Until then the foundation gates keep `linux-v1.1.0-rc1`, and their freeze checks fail on `android/int` once the export lane is merged.

## P6 exit check (2026-10-09, platform lane)

At the end of P6 (`android/lane-plat` at `4cfe0b3`, `docs(android): record the P6 wave 7 merge and the open gate items`, base `ba57adf`):

- `scripts/android/check_linux_frozen.sh --tag linux-v1.1.0-rc1` prints `frozen against linux-v1.1.0-rc1 (65b567f): 16 paths, clean`, and `git diff --quiet ba57adf HEAD -- Cargo.toml Cargo.lock rust-toolchain.toml crates src-tauri ui deploy android/rust android/pos-core android/smoke` is empty: P6 changed no Linux build input and no engine or FFI source. The lane's only build-tooling changes are `android/docker/`, `scripts/android/check_repro.sh` and `.github/workflows/android.yml` (P6-T10, the `:app` APK), which are not Linux build inputs.
- `p4_gate.sh --fast --linux-repro` built the tag twice in Docker from a clean clone on 2026-10-09 and reproduced SHA-1 `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e` (`~/.cache/pili-freeze/p4-linux-repro.log`, integrator wave 7). The check builds the tag, not the lane head, so the result holds for the exit head.
- No tag was set (PD228). The plan requires no freeze step at the end of P6. The re-freeze `linux-v1.0.0-rc1` waits for the user. The nightly export lane (`android/lane-export`, X1) changes Linux build inputs and proposes the candidate SHA-1 `d4fb09a2ae2cf2501fe4d6847eb72e954d0b95ed` for it. When that re-freeze happens, `p4_gate.sh` (hard-coded `linux-v1.1.0-rc1`) and the platform string `plat_about_engine_ref` (`pili-pos 1.1.0, linux-v1.1.0-rc1 (65b567f)`, checked by `plat-engine-ref`) must follow.
