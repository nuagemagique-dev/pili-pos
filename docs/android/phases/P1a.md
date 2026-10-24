# P1a: Engine extraction

Phase P1a of the Android port of moyun-pos (pili-pos) to the Sunmi D3 Pro (plan `~/.claude/plans/i-d-like-you-to-vast-koala.md`, section "P1a: Engine extraction", invariants and orchestration protocol). The Tauri-free fiscal core moves out of `src-tauri` into a new crate `crates/pos-engine` as a pure, test-guarded move, so the Linux shell and the later Android FFI call the same engine source.

- **Base:** `android/int` at `aeee4e8b6624dcf48d1c6a79a44d615ef6106f2e` (`docs(android): S4 exit report`). main (`96728e1`) and `cert/v1.0` (`d6210ca`) are not touched in this run.
- **Rule of the phase:** no behaviour change. Same FDM requests, tickets, database, Tauri command names, argument names, JSON shapes and event topics. Every commit green.
- **Inputs read by the planner:** the plan, `docs/android/phases/P0-exit.md`, `docs/android/phases/S4-exit.md`, `docs/android/DECISIONS.md` (sections 4, 5, 11 and the "Decided while the user was away" list), `docs/android/BASELINE.md`, the code at `aeee4e8`.
- **Measured starting point:** `src-tauri/src/commands` has 27 files; 79 `#[tauri::command]` wrappers (77 without `peppol`); 8 wrappers hold logic (the plan's 7 plus `list_tables`); 47 integration test files plus 9 harness files under `src-tauri/tests`; 2 globals (`STATUS_CACHE` in `commands/checkout.rs`, `SIGN_MESSAGE_SINK` in `crates/pos-fiscal/src/fdm_client.rs`); `POS_SW_VERSION` is a compile-time constant (`crates/pos-fiscal/src/event.rs:12`); 3 production HTTP client sites (`state.rs:98`, `delivery.rs:77`, `fdm_client.rs:427`); 168 dossier path references in 25 files; test inventory 1846 lines (ws 1355, cert 491, sha256 `586834f0...`); all dossier checkers green.

## Cards and waves

| Card | Wave | Depends on | Lane | Plan step | Scope | Cargo heavy |
|------|------|------------|------|-----------|-------|-------------|
| P1a-T1 | 1 | none | A (engine) | 1, 2 | engine functions for the 8 wrappers with logic; 79 wrappers moved to `src-tauri/src/ipc`; `check_ipc_wrappers.py` | yes |
| P1a-T2 | 1 | none | tooling | 9 + gates | inventory across moves (`test_moves.tsv`, multi-package cert pass, `--min-ws`), `p1a_gate.sh`, `check_linux_frozen.sh`, `exclude = ["android"]`, test.yml `paths-ignore` | yes |
| P1a-T3 | 1 | none | docs | 10 (tool) | `rewrite_dossier_paths.py` + tests; planner decisions appended to DECISIONS.md | no |
| P1a-T4 | 2 | T1, T2, T3 | A | 3 | `EngineConfig`, `EnginePaths`, `ConfigSource`, `Engine::open` in src-tauri; Tauri manages the Engine | yes |
| P1a-T5 | 3 | T4 | A | 4 (+ install) | `git mv` commands/, state.rs, engine.rs to `crates/pos-engine`; features mirrored; `pos_engine::install`; temporary re-exports | yes |
| P1a-T6 | 4 | T5 | A | 5 | tests to `crates/pos-engine/tests`, harness to `crates/pos-engine-testkit`; scripts and generators follow | yes |
| P1a-T7 | 5 | T6 | A | 6 | `EventSink`/`EngineEvent`, background loops into the engine, `TauriEventSink` | yes |
| P1a-T8 | 6 | T7 | A | 7 | globals removed (status cache, sign-message sink), `engine.lock` | yes |
| P1a-T9 | 7 | T8 | A | 8a | `SoftwareIdentity`, `HttpTls`; FDM journal parity (Docker) | yes |
| P1a-T10 | 8 | T9 | A | 8b | device id override, support program from the host, `PosConfig::resolve_relative_to`, `ConfigSource::Bundle`, install API | yes |
| P1a-T11 | 9 | T10 | docs | 10 (prose) | dossier prose, CLAUDE.md/README.md, `docs/android/ENGINE.md` | no |

The engine lane is strictly sequential (each step builds on the previous one), so after wave 1 every wave holds one card. Eleven cards, nine waves. No card needs a human (needs-human: no on all cards).

**Size note.** P1a-T1, P1a-T5 and P1a-T6 exceed the 600 LOC guideline in raw diff lines, but the excess is mechanical: moved wrappers (checked by `check_ipc_wrappers.py --signatures`) and `git mv` renames (checked by rename detection). Their hand-written logic stays well under the guideline.

**Wave 1 merge order (PD15):** P1a-T2, then P1a-T3, then P1a-T1.

## Planner decisions (decided while the user was away)

P1a-T3 copies these into `docs/android/DECISIONS.md` under "Decided while the user was away (review)".

- **PD1 Target layout.** `crates/pos-engine` holds `commands/` (engine functions only), `state.rs`, `engine.rs` (EngineConfig, EnginePaths, ConfigSource, Engine), `events.rs`, `background.rs`, `install.rs`. `src-tauri` keeps `main.rs`, `cli.rs` (argument parsing), `paths.rs`, `events.rs` (TauriEventSink) and `ipc/` (79 one-call wrappers, 77 in the cert build). Integration tests live in `crates/pos-engine/tests`, their harness in `crates/pos-engine-testkit` (`publish = false`, outside the cert build graph). Test target names and test function names do not change.
- **PD2 Test inventory across moves.** The baseline `docs/android/baseline/test_inventory.txt` is never edited. Pure moves are declared as prefix rewrites in `docs/android/baseline/test_moves.tsv`, applied by `scripts/android/test_inventory.sh --diff` (a stale rule or a name collision fails). The `cert` section covers every workspace member with a `cert` feature (pili-pos, then pos-engine). Gate: 0 removed and `ws >= 1355`.
- **PD3 Version during P1a.** The workspace version stays `1.0.0` through P1a, so the normalised FDM journals (which keep `posSwVersion` `pili-pos/1.0.0`) must stay byte-identical (sha256 `68a09ee3...`) with identical `eventCounter`/`totalCounter` sequences. The bump to `1.1.0` is a P1b exit step together with the soft freeze `linux-v1.1.0-rc1`, which needs the user's go-ahead.
- **PD4 An eighth wrapper with logic.** Besides the plan's seven, `list_tables` reads `table_count` from the configuration; it gets `do_list_configured_tables`.
- **PD5 Managed state.** Tauri manages the `Engine` object, which derefs to `AppState`; wrappers keep calling `do_*(state.inner(), ...)` through the alias `ipc::Managed`.
- **PD6 Events.** A typed `EngineEvent` enum with the six exact topics (`fdm-sign-messages`, `webshop-order-received`, `webshop-order-updated`, `catalog-updated`, `catalog-rejected`, `catalog-awaiting-adoption`); the Tauri sink emits the same typed payloads as today. Its AppHandle is an instance field filled in `setup`, not a static.
- **PD7 Where the runtime seams live.** `HttpTls` (DECISIONS §4) and `SoftwareIdentity` are defined in `pos-fiscal` and re-exported by `pos-engine`. The version string is a required EngineConfig input: never read from `pos.json`, never a compile-time constant in `pos-fiscal` or `pos-engine`. Linux passes `pili-pos/` plus its Cargo version; Android will pass `pili-pos-android/X.Y.Z` (DECISIONS §2).
- **PD8 Bundle configuration.** `ConfigSource` gets a third variant `Bundle(path)` (relative paths resolved against the bundle directory) for the Android provisioning bundle. Linux only uses `File` and `DevelopmentDefaults`, so its behaviour is unchanged.
- **PD9 No new crates.** The engine lock uses `std::fs::File::try_lock` (stable since Rust 1.89). No registry crate is added, removed or bumped in `Cargo.lock` during P1a.
- **PD10 Dossier paths.** The 168 path references in 25 files (docs/SUBMISSION_ARTIFACTS, CLAUDE.md, deploy/REMOTE-OPS.md) are rewritten by `scripts/android/rewrite_dossier_paths.py` in the same commit as each move, so `check_dossier.py` stays green on every commit. `docs/android/**` and `docs/internal/**` keep their historical paths.
- **PD11 Gate cost.** The full gate runs on each card's last commit, the fast gate on every earlier commit. The FDM journal capture (Docker) runs in P1a-T9 and at the phase exit only. `check_repro.sh` is not run in P1a (it is the P1b exit check).
- **PD12 DLE EOT timeout (S4 exit).** Not changed in P1a: a pure move admits no behaviour change. It stays a question for the user and a candidate behaviour-change card for the P1b planner, before the soft freeze.
- **PD13 Remaining environment reads in the engine.** The `HUBRISE_*` development fallback (`commands/hubrise.rs`) and the support session HOME (`RUNTIME_DIRECTORY`, else `temp_dir()`, `commands/support.rs`, the only production `temp_dir()` use) stay; the second is reached only with a support program, which Android never passes. Recorded for P2.
- **PD14 Freeze check.** `scripts/android/check_linux_frozen.sh` is written in P1a but not run: no freeze tag exists, and no freeze tag is created in P1a.
- **PD15 Wave 1 merge order.** P1a-T2, then P1a-T3, then P1a-T1, so the gate script exists on `android/int` when P1a-T1 is checked there.
- **PD16 Test backups.** The test harness gets its own temporary data directory. Until P1a, a Z report test without `backup_dir` wrote its backup into the developer's real data directory (`paths::backups_dir()`); recorded as a test-only fix, no fiscal behaviour involved.
- **PD17 Phase exit without the user.** P1a has no freeze step. With the user away, the planner writes `P1a-exit.md` and the chain continues to P1b on `android/int`; merging into main waits for the user.

## Target layout (end of P1a)

```
crates/pos-engine/            Tauri-free engine (in the cert build graph)
  Cargo.toml                  features: default = [hubrise, peppol], hubrise, peppol, cert = [hubrise]
  src/lib.rs                  pub mod commands, engine, events, background, install, state
  src/commands/*.rs           the former src-tauri/src/commands, engine functions only (do_*)
  src/state.rs                AppState (+ paths, events, identity, http_tls, status cache)
  src/engine.rs               EngineConfig, EnginePaths, ConfigSource, EngineTimers, Engine::open (+ engine.lock)
  src/events.rs               EventSink, EngineEvent (six topics), NullSink
  src/background.rs           auto-Z, mail outbox, HubRise poller and outbox; start_background, shutdown
  src/install.rs              init_db, apply_seed, check_config, check_config_source
  tests/*.rs                  the 47 former src-tauri/tests files, same names
crates/pos-engine-testkit/    publish = false, the former src-tauri/tests/common (TestApp, fixtures, compare.rs)
crates/pos-fiscal/            + SoftwareIdentity, HttpTls; FdmClient with a per-client sign-message sink
src-tauri/                    thin shell: main.rs, cli.rs (parse), paths.rs, events.rs (TauriEventSink), ipc/*.rs
```

EngineConfig at the end of P1a: `paths`, `config: ConfigSource::{File, Bundle, DevelopmentDefaults}`, `identity: SoftwareIdentity`, `device_id_override: Option<String>`, `http_tls: HttpTls`, `support_program: Option<PathBuf>`, `events: Arc<dyn EventSink>`, `timers: EngineTimers { auto_z, mail_outbox }` (P1b adds `idle_lock` and `fdm_poll`). Linux values: `EnginePaths::under(paths::data_dir())`, File/DevelopmentDefaults as today, `pili-pos/1.0.0`, None, `Platform`, the RustDesk path when installed, `TauriEventSink`, 60 s and `OUTBOX_INTERVAL`.

## Shared environment (all cards)

- Worker worktree `$WT` on its task branch from `android/int`. `INT=/Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/int` is read only for workers.
- `export PATH="$HOME/.cargo/bin:$PATH"` (Homebrew cargo shadows rustup); `unset CARGO_TARGET_DIR` so cargo builds in `$WT/target`.
- Warm start: `cp -cR "$INT/target" "$WT/target"` (APFS clone, no extra disk) and `cp -cR "$INT/ui/node_modules" "$INT/ui/dist" "$WT/ui/"` (the cert build embeds `ui/dist`).
- Host: Apple M1, 16 GB RAM, about 59 GB free disk, shared. One heavy cargo command per agent; `CARGO_BUILD_JOBS=4` in wave 1 (two cargo cards). Never run `scripts/check_repro.sh`. Use `grep -rnE`, not `rg`, in scripts. Docker only in P1a-T9 and the phase exit (journal capture, local mocks only).
- Workers leave `$WT/target` for the reviewer; the integrator deletes a card's worktree target after the merge.

## Rules (reviewer checks on every card)

- Allowed paths per card (regex in the card).
- Never edited: `ui/`, `rust-toolchain.toml`, `deploy/build-release.sh`, `scripts/build_and_hash.sh`, `scripts/check_repro.sh`, `docs/android/baseline/test_inventory.txt`.
- Tests never drop: no test deleted, renamed or newly ignored; pure moves declared in `test_moves.tsv` only by the cards that move tests (T5, T6). `test_inventory.sh --diff`: 0 removed, `ws >= 1355`.
- `Cargo.lock`: only workspace-member entries and dependency lists change; no `source` or `checksum` line changes.
- No `cfg(target_os)`, no tauri, no new globals in the engine sources. Every wrapper is a single call (`check_ipc_wrappers.py`). The IPC contract is unchanged (`--signatures` diff against `android/int`).
- No secrets or build outputs committed; no live module2/HubRise/SPF call (never `scripts/live_*.sh` or ignored live tests); no push, no GitHub Actions, no `declare.sh`, no tags.
- No em-dash anywhere; Conventional Commits, merge commits included; dossier French and brand-neutral.

### Gates

- **Full gate** (last commit of each card from wave 2 on): `scripts/android/p1a_gate.sh --base android/int`.
- **Fast gate** (every commit): `scripts/android/p1a_gate.sh --fast --base android/int`. Reviewer loop:

```bash
bash -c 'b=$(git rev-parse --abbrev-ref HEAD); for c in $(git rev-list --reverse android/int..HEAD); do git checkout -q $c && scripts/android/p1a_gate.sh --fast --base android/int || { echo FAIL at $c; git checkout -q $b; exit 1; }; done; git checkout -q $b'
```

- **Hygiene** (inside the gate script; spelled out for wave 1 cards):

```bash
! git diff android/int...HEAD | grep '^+' | grep -q $'\xe2\x80\x94'
! git diff --name-only android/int...HEAD | grep -Eiq '\.(so|dylib|apk|aab|aar|dex|a|rlib|jks|keystore|p12|pfx|pem|key)$|(^|/)\.env|local\.properties$'
! git log --format=%s android/int..HEAD | grep -vqE '^(feat|fix|docs|style|refactor|perf|test|chore)(\([a-z0-9-]+\))?: '
git diff --quiet android/int...HEAD -- ui rust-toolchain.toml deploy/build-release.sh scripts/build_and_hash.sh scripts/check_repro.sh docs/android/baseline/test_inventory.txt
! git diff android/int...HEAD -- Cargo.lock | grep -qE '^[+-](source|checksum) '
test -z "$(git diff --name-only android/int...HEAD | grep -vE '<ALLOWED>')"
```

- **Freeze invariant:** `scripts/android/check_linux_frozen.sh` exists from P1a-T2 but no freeze tag exists in P1a, so the gate prints SKIP. The integrator never creates `linux-v*` tags in P1a.

## Integration procedure

1. Merge order inside a wave as listed (wave 1: T2, T3, T1). Merge with `git merge --no-ff -m "chore(android): merge P1a-Tn <short title>"` into `android/int` (the subject must contain `merge P1a-Tn`: the phase gate keys its exit checks on `merge P1a-T11`).
2. Rerun the phase gate on `android/int` after each wave.
3. Delete the merged card's worktree target dir.
4. REWORK at most twice per card; a third failure, a freeze violation or two escalations stop the run (plan protocol). With the user away, the planner decides and records in DECISIONS.md.

## Phase gate (integrator, on android/int)

```bash
cd /Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/int
test -z "$(git status --porcelain)"
test -f ui/dist/index.html || (cd ui && npm ci && npm run build)
PATH="$HOME/.cargo/bin:$PATH" scripts/android/p1a_gate.sh --base aeee4e8b6624dcf48d1c6a79a44d615ef6106f2e
# freeze invariant: SKIP while no tag exists
# main, cert/v1.0 untouched, no linux-v* tag
# after merge P1a-T11 only: rewrite_dossier_paths.py --strict, layout checks,
#   FDM journal capture: normalised sha256 68a09ee3..., eventCounter/totalCounter identical to run1
```

The exact command list is in the orchestrator's phase gate.

## Card P1a-T1: engine functions for every wrapper, wrappers to src-tauri/src/ipc (wave 1)

- **Depends on:** none. **Lane:** A. **Needs human:** no. **Plan steps:** 1 and 2.
- **Allowed:** `^(src-tauri/src/|scripts/android/(check_ipc_wrappers|test_check_ipc_wrappers)\.py$|docs/SUBMISSION_ARTIFACTS/)`

**Work.**
1. `refactor(engine): give every command wrapper an engine function`: bodies moved verbatim into pub engine functions in the same files: `do_get_current_user`, `do_get_categories`, `do_get_menu_items`, `do_check_printer`, `do_check_kitchen_printer`, `do_hubrise_get_connection_status`, `do_hubrise_disconnect`, `do_list_configured_tables` (reads `table_count`, calls the unchanged `do_list_tables`).
2. `refactor(shell): move the Tauri command wrappers to src-tauri/src/ipc`: `ipc/mod.rs` (`pub type Managed = crate::state::AppState;`) and one file per module with wrappers, same cfg gates; each wrapper keeps name, parameter names and types, return type, async-ness and doc; body is one call `crate::commands::<m>::<fn>(state.inner(), args)` (+ `.await`). Allowed argument forms: `p`, `&p`, `p.as_ref()`, `p.as_deref()`, `p.f`, `&p.f`, `p.f.as_ref()`. No tauri left in `commands/` or `state.rs`; engine functions and signature types pub; `main.rs` lists `ipc::<m>::<cmd>`, same order and membership.
3. `chore(android): add the IPC wrapper checker`: `check_ipc_wrappers.py` (default check, `--map`, `--signatures DIR`) and its unittest.
4. Dossier: the two refs to wrappers (`financial.rs::open_drawer_fiscal`, `invoice.rs::issue_b2b_invoice`) point at the engine functions they call (`do_open_drawer_fiscal`, `do_issue_b2b_invoice`).

**Acceptance.** 79 wrappers in `src-tauri/src/ipc`, 0 in `commands/`; checker `single-call: 79/79`, handler lists 79/77; `--signatures` identical to `android/int`'s `src-tauri/src/commands`; inventory 0 added, 0 removed; no test, crate or Cargo file changed; every commit passes fmt, clippy and workspace tests.

**Gates.** Explicit cargo commands (fmt, clippy ws, test ws, cert clippy and test for pili-pos), `test_inventory.sh --diff`, the checker and its tests, the signature diff, the tauri grep and counts, dossier checks, hygiene, per-commit loop with explicit commands.

## Card P1a-T2: gate tooling, inventory across moves, freeze check, workspace exclude (wave 1)

- **Depends on:** none. **Lane:** tooling. **Needs human:** no. **Plan step:** 9 plus the phase gates.
- **Allowed:** `^(scripts/android/(test_inventory\.sh|test_test_inventory\.sh|check_linux_frozen\.sh|test_check_linux_frozen\.sh|p1a_gate\.sh|p1a_globals_allow\.txt)|docs/android/baseline/test_moves\.tsv|Cargo\.toml|\.github/workflows/test\.yml)$`

**Work.**
1. `test_inventory.sh`: cert section over every member with a `cert` feature; `--moves FILE` (default `docs/android/baseline/test_moves.tsv`, prefix rewrites, stale rule and collision fail); `--current FILE`; `--min-ws N` (1355). Self-test `test_test_inventory.sh` (no cargo). Empty `test_moves.tsv` with header.
2. `check_linux_frozen.sh [--tag TAG]` (default `linux-v1.1.0`, else `-rc1`, else exit 2): frozen paths plus every path crate of the cert graph; tracked diff and untracked files; self-test `test_check_linux_frozen.sh` in a temporary clone.
3. `p1a_gate.sh [--fast] [--base REF]`: fast and full check sets (see the card brief), `GATE | name | PASS|FAIL|SKIP | detail` lines; `p1a_globals_allow.txt` with `STATUS_CACHE`, `SIGN_MESSAGE_SINK`.
4. Root `Cargo.toml` `exclude = ["android"]`; test.yml `paths-ignore: ['android/**', 'docs/android/**']` on push and pull_request.

**Acceptance.** A fresh write-mode inventory is byte-identical to the baseline; `--diff` 0/0 with `moves: 0 rules`; the self-tests pass; `check_linux_frozen.sh` exits 2 in the real repo; `p1a_gate.sh --base android/int` passes with SKIP only for absent tools and the freeze; `Cargo.lock` unchanged.

## Card P1a-T3: dossier path rewrite tool and the planner decisions (wave 1, docs lane)

- **Depends on:** none. **Lane:** docs. **Needs human:** no. **Plan step:** 10 (tool). No cargo.
- **Allowed:** `^(scripts/android/(rewrite_dossier_paths|test_rewrite_dossier_paths)\.py|docs/android/DECISIONS\.md)$`

**Work.** `rewrite_dossier_paths.py` with ordered rules (wrapper refs to `src-tauri/src/ipc`, cli install functions to `crates/pos-engine/src/install.rs`, `commands`, `state.rs`, testkit, tests), scope `docs/SUBMISSION_ARTIFACTS/**/*.md`, `README.md`, `CLAUDE.md`, `deploy/**/*.md`; modes `--plan`, `--apply` (only rules whose target exists), `--check`, `--strict`, `--unmapped`; unittest. Append PD1 to PD17 to DECISIONS.md.

**Acceptance.** `--plan` totals 168 references in 25 files; `--unmapped` empty; `--apply` changes nothing on this tree; `--check` 0, `--strict` 1; tests pass; DECISIONS.md carries the 17 bullets.

## Card P1a-T4: EngineConfig and Engine::open in src-tauri (wave 2)

- **Depends on:** T1, T2, T3. **Lane:** A. **Needs human:** no. **Plan step:** 3.
- **Allowed:** `^(src-tauri/(src|tests)/|docs/SUBMISSION_ARTIFACTS/)`

**Work.** `src-tauri/src/engine.rs`: `EnginePaths::under`, `ConfigSource::{File, DevelopmentDefaults}`, `config_source_from(env_path, production_path)`, `EngineConfig { paths, config }`, `Engine { state: Arc<AppState> }` with `open` (same steps, order and log text as `AppState::new`), `Deref<Target = AppState>`, `state()`. `AppState::new`/`load_config` removed, `AppState.paths` added, `backup_database_impl` takes the default backups dir (no `crate::paths` in engine code). main.rs opens the Engine, manages it, `ipc::Managed = Engine`. Harness gets a per-TestApp temp data dir (PD16). Engine unit tests.

**Acceptance.** No `crate::paths`/`std::env` in `commands/`, `state.rs`, `engine.rs` except the two PD13 reads; `state::tests::*` names unchanged; wrappers still 79/79 single calls; full gate green.

## Card P1a-T5: move commands, state and engine into crates/pos-engine (wave 3)

- **Depends on:** T4. **Lane:** A. **Needs human:** no. **Plan step:** 4 (plus the install module).
- **Allowed:** `^(crates/pos-engine/|src-tauri/|Cargo\.(toml|lock)$|docs/android/baseline/test_moves\.tsv$|docs/SUBMISSION_ARTIFACTS/|CLAUDE\.md$|deploy/[^/]+\.md$|scripts/(check_dossier|test_check_dossier)\.py$|\.github/workflows/test\.yml$)`

**Work.** `git mv` commands/, state.rs, engine.rs; `crates/pos-engine/Cargo.toml` with mirrored features; `lib.rs`; `install.rs` (init_db, apply_seed, check_config and their 6 tests from cli.rs); workspace member; src-tauri depends on pos-engine with feature forwarding; temporary `pub use pos_engine::{commands, state}` in `src-tauri/src/lib.rs`; moves rules for `commands::`, `state::` and the 6 cli tests; test.yml cert steps for pos-engine; check_dossier absolute-path literal scan extended to `crates/pos-engine/src`; `rewrite_dossier_paths.py --apply`.

**Acceptance.** 29 renames detected; no tauri in pos-engine (sources and `cargo tree`); pos-engine in the pili-pos cert graph; inventory 0 removed with the new rules; dossier `--check` 0; full gate green.

## Card P1a-T6: tests to crates/pos-engine/tests, harness to crates/pos-engine-testkit (wave 4)

- **Depends on:** T5. **Lane:** A. **Needs human:** no. **Plan step:** 5.
- **Allowed:** `^(crates/pos-engine/|crates/pos-engine-testkit/|src-tauri/|Cargo\.(toml|lock)$|docs/android/baseline/test_moves\.tsv$|docs/SUBMISSION_ARTIFACTS/|CLAUDE\.md$|deploy/[^/]+\.md$|scripts/(capture_uc_evidence\.sh|live_fdm\.sh|live_hubrise\.sh|live_combined\.sh|live_hubrise_login\.sh|embed_uc_evidence\.py|uc_matrix\.py|test_uc_matrix\.py|check_dossier\.py|test_check_dossier\.py|generate_test_report\.py)$|\.github/workflows/test\.yml$)`

**Work.** `git mv` 47 test files and 9 harness files; testkit crate (`publish = false`); test files use `pos_engine_testkit as common` and `pos_engine::`; include paths fixed; temporary re-exports removed; moves rule `pili-pos::test:` to `pos-engine::test:`; scripts (`-p pos-engine`, never run live ones), generators (embed text, uc_matrix description); `rewrite_dossier_paths.py --apply`; MATRICE regenerated.

**Acceptance.** `src-tauri/tests` gone; testkit outside the cert graph; inventory 0 removed; UC evidence tests pass into a temp dir; `embed_uc_evidence.py` idempotent; `uc_matrix.py --write` changes only the date line; full gate green.

## Card P1a-T7: EventSink and background loops into the engine (wave 5)

- **Depends on:** T6. **Lane:** A. **Needs human:** no. **Plan step:** 6.
- **Allowed:** `^(crates/pos-engine/|crates/pos-engine-testkit/|src-tauri/|Cargo\.lock$|docs/SUBMISSION_ARTIFACTS/)`

**Work.** `events.rs` (EngineEvent, six topics, EventSink, NullSink, outcome mapping); `EngineConfig.events`, `EngineTimers`; `background.rs` (`start_background(&tokio Handle)`, `shutdown()`, auto-Z, mail outbox, HubRise poller and outbox, sign-message forwarding through the still-global sink); `src-tauri/src/events.rs` TauriEventSink; main.rs keeps window, kiosk, health listener; dossier prose about where the loops start.

**Acceptance.** No fiscal loop in main.rs; topic strings only in the engine; payload types unchanged; loop and shutdown tests; full gate green.

## Card P1a-T8: no globals, engine lock file (wave 6)

- **Depends on:** T7. **Lane:** A. **Needs human:** no. **Plan step:** 7.
- **Allowed:** `^(crates/pos-engine/|crates/pos-engine-testkit/|crates/pos-fiscal/|src-tauri/|Cargo\.lock$|docs/SUBMISSION_ARTIFACTS/|scripts/android/p1a_globals_allow\.txt$)`

**Work.** Status cache into AppState; per-client sign-message sink in `FdmClient` (`on_sign_messages` removed), engine `fdm_client` helper; `engine.lock` with `File::try_lock`; allowlist emptied; dossier sentence on `on_sign_messages` and the data directory listing.

**Acceptance.** Zero global declarations in `crates/pos-engine/src` and `crates/pos-fiscal/src`; second open on one data dir fails, reopen after drop works; status cache and sign-message tests keep their names; full gate green.

## Card P1a-T9: SoftwareIdentity and HttpTls (wave 7)

- **Depends on:** T8. **Lane:** A. **Needs human:** no. **Plan step:** 8 (first half).
- **Allowed:** `^(crates/pos-(fiscal|mail|print|core|engine|engine-testkit)/|src-tauri/|Cargo\.lock$|docs/SUBMISSION_ARTIFACTS/)`

**Work.** `SoftwareIdentity` (1 to 36 chars) replaces `POS_SW_VERSION` in `build_common_fields` and tickets; `EngineConfig.identity`; Linux passes `pili-pos/` + `CARGO_PKG_VERSION` from src-tauri. `HttpTls::{Platform, RootsOnly}` with `client_builder()` at the three production sites; empty roots refused at open; Linux passes `Platform`. FDM journal capture (Docker, local mocks).

**Acceptance.** No `POS_SW_VERSION`, no `CARGO_PKG_VERSION` in engine or pos-fiscal sources; no bare reqwest builder in the engine; normalised journal sha256 `68a09ee3...`, counters identical, `posSwVersion` `pili-pos/1.0.0`; full gate green.

## Card P1a-T10: device id override, support program, bundle configuration (wave 8)

- **Depends on:** T9. **Lane:** A. **Needs human:** no. **Plan step:** 8 (second half).
- **Allowed:** `^(crates/pos-(core|engine|engine-testkit)/|src-tauri/|Cargo\.lock$|docs/SUBMISSION_ARTIFACTS/)`

**Work.** `device_id_override`; `support_program` from the host (RustDesk path and existence check move to the shell); `PosConfig::resolve_relative_to`; `ConfigSource::Bundle`; `install::check_config_source`; no absolute Linux path literal and no env read in the engine besides PD13.

**Acceptance.** Tests for override, bundle resolution, support None; Linux behaviour unchanged; full gate green; src-tauri LOC reported.

## Card P1a-T11: dossier and repo docs after the extraction (wave 9, docs lane)

- **Depends on:** T10. **Lane:** docs. **Needs human:** no. **Plan step:** 10 (prose).
- **Allowed:** `^(docs/SUBMISSION_ARTIFACTS/.*\.md|docs/android/ENGINE\.md|README\.md|CLAUDE\.md|deploy/[^/]+\.md)$`

**Work.** `rewrite_dossier_paths.py --strict` clean; dossier prose on the new structure (no fiscal statement, figure, result or evidence changed; no v1.1 legal framing); CLAUDE.md and README.md structure and commands; `docs/android/ENGINE.md` (EngineConfig, lifecycle, events, command map from `check_ipc_wrappers.py --map`, PD13 env reads, what P2 passes on Android).

**Acceptance.** Docs only; dossier checkers green; `embed_uc_evidence.py` idempotent; no em-dash; brand-neutral French.

## Phase exit

- All eleven cards APPROVE and merged; the phase gate green on `android/int`, including the exit-only checks: `rewrite_dossier_paths.py --strict`, layout (`src-tauri/src/commands`, `src-tauri/src/state.rs`, `src-tauri/tests` gone; `crates/pos-engine`, `crates/pos-engine-testkit` present), no `POS_SW_VERSION`/`on_sign_messages`, the FDM journal capture (normalised sha256 `68a09ee31caa69a39864de47bb71313f949034b7da86d93fbd3af3b01c073473`, `eventCounter`/`totalCounter` identical to `fdm_journal_hubrise_e2e_run1.json`).
- main at `96728e1`, `cert/v1.0` at `d6210ca`, no `linux-v*` tag.
- The planner writes `docs/android/phases/P1a-exit.md`: gate evidence, inventory figures (ws, cert, added, renamed by moves), journal hashes, src-tauri LOC against the plan's < 900 target, open items (PD12 DLE EOT, PD13 env reads, version bump and rc1 freeze for P1b), questions for the user. Per PD17 the chain continues with P1b; merging to main waits for the user.
