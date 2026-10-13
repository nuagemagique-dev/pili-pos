# pos-ffi: the Kotlin facade of the engine

Status: written at the end of phase P2. English, internal, not part of the dossier. Source of truth is the code: `android/rust/pos-ffi/src` and the decisions PD52 to PD66 of `docs/android/DECISIONS.md`. The engine itself is described in `docs/android/ENGINE.md`.

## 1. What pos-ffi is

`android/rust/pos-ffi` is a Rust crate (UniFFI 0.32.2, proc-macro mode, no UDL) that wraps `crates/pos-engine` for Kotlin. It depends on `pos-engine` with `default-features = false, features = ["cert"]`: HubRise in, Peppol out, no Tauri. It adds no fiscal logic: every command of the Linux shell has one wrapper here that calls the same engine function (PD52).

- `PosEngine` (an `Object`): `open` (the only synchronous export, a constructor), `shutdown`, `events`, and one `suspend` method per cert command (89, section 11).
- `EventStream` (an `Object`): `next()` returns the next `PosEvent`, or `null` once the engine is gone.
- Free functions: `initLogging`, `posSwVersion`, `installInitDb`, `checkConfigBundle`, `qrMatrix`, `verifyMinisign`, `verifyBundle`, `settlePreview`, `cashDueCents`, `vatSplit`.
- Records, enums and errors mirror the engine types (section 7); Kotlin never sees SQL, JSON of the engine or a Rust type.

`android/rust` is its own Cargo workspace (members `pos-ffi`, `uniffi-bindgen`); its target directory is `<repo>/target/android-rust`. The Kotlin side lives in `android/` (Gradle 8.11.1, Kotlin 2.2.20). In P2 the module `:pos-core` is a Kotlin/JVM library whose main sources are the generated bindings; P3 turns it into an Android library and keeps the JVM tests as host unit tests (PD60).

## 2. Building and generating the bindings

All commands from `android/rust` unless stated. Put rustup's cargo first on `PATH` (`export PATH="$HOME/.cargo/bin:$PATH"`, Homebrew's cargo has no Android std) and unset `CARGO_TARGET_DIR`.

Host library and the Kotlin bindings for the JVM tests (package `uniffi.pos_ffi`, no Android flags, config `jvm-uniffi.toml`):

```sh
cargo build --locked -p pos-ffi
cargo run --locked -p uniffi-bindgen -- generate \
  --library ../../target/android-rust/debug/libpos_ffi.dylib \
  --language kotlin --config jvm-uniffi.toml --out-dir /tmp/pos-ffi-jvm --no-format
```

(`libpos_ffi.so` on Linux.) Gradle does both through the tasks `cargoBuildHost` and `uniffiBindgen` of `:pos-core`; the output is `android/pos-core/build/generated/uniffi` and is never committed.

The Android library (arm64, 16 KB pages, unwinding panics), then the bindings from that exact unstripped `.so`, with the Android flavour config `android-uniffi.toml` (`android = true`, `android_cleaner = true`: `android.system.SystemCleaner`, API 33 and later):

```sh
export ANDROID_HOME=$HOME/Library/Android/sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.1.12297006
cargo ndk -t arm64-v8a build --locked --profile android-release -p pos-ffi
# -> ../../target/android-rust/aarch64-linux-android/android-release/libpos_ffi.so
cargo run --locked -p uniffi-bindgen -- generate \
  --library ../../target/android-rust/aarch64-linux-android/android-release/libpos_ffi.so \
  --language kotlin --config android-uniffi.toml --out-dir /tmp/pos-ffi-android --no-format
```

Kotlin names: methods are camelCase (`list_fiscal_events` becomes `listFiscalEvents`), records are data classes with `var` properties, enums are Kotlin enums (`OrderType.TAKEAWAY`), errors are `PosException`. The generated Kotlin checks, at load time, the UniFFI contract version and one checksum per exported item against the library: a library that does not match the bindings is refused (tested by `mismatchTest`, section 12).

## 3. EngineOptions and defaults (PD53)

```kotlin
EngineOptions(dataDir: String, configBundle: String, deviceId: String?, timers: EngineTimersMs?)
```

| Field | Rule |
|---|---|
| `dataDir` | Absolute path of the app files directory: `pos.db`, `backups/`, `engine.lock`. A relative path is refused (`Internal`, "absolute"). |
| `configBundle` | Absolute path of `pos.json` in the provisioning bundle. The configuration is always `ConfigSource::Bundle` (relative paths inside it resolve against the bundle directory); there is no `File` and no `DevelopmentDefaults` over the FFI, so no silent TEST identity. |
| `deviceId` | The handset identifier (Sunmi serial); replaces `device_id` of `pos.json`. Trimmed, 1 to 600 characters. |
| `timers` | `null`: `EngineTimers::default()` with `printer_status_read` at 2 s (PD37). Otherwise `EngineTimersMs { autoZMs, mailOutboxMs, printerStatusReadMs, idleLockMs, fdmPollMs }`; a value of 0 or less is refused. Tests use short ones. |

Not passed by Kotlin, built in Rust: the software identity `pili-pos-android/<pos-ffi crate version>` (`posSwVersion()` returns it; `1.0.0-dev` during development), `HttpTls::RootsOnly` with the roots of `webpki-root-certs` 1.0.9, and `support_program = None` (the support commands answer "not available").

## 4. Lifecycle (PD55)

1. `initLogging(logDir, filter)` once per process (section 10), then `PosEngine.open(options)` **on `Dispatchers.IO`**: it opens the database, runs the migrations, takes the exclusive lock `<dataDir>/engine.lock` and starts the background tasks (automatic Z, mail outbox, presence worker, idle lock, FDM monitor, HubRise poller). A second engine on the same directory fails with `PosException.Internal("... engine already open ...")`.
2. Every other call is a `suspend` method; call it from any coroutine context.
3. `shutdown()` refuses new calls (`EngineClosed`), cancels the background tasks, waits for them and for the calls in flight for **at most 30 s**, then drops the engine, which releases `engine.lock`. A second `shutdown()` is Ok. On timeout it returns `Internal` and keeps the engine (try again). A `shutdown()` cancelled by its caller still completes in the background, and the next `shutdown()` waits for it, so Ok always means the engine is closed, `engine.lock` is free and every `EventStream` ends. `events()` after shutdown is `EngineClosed`, and `EventStream.next()` returns `null`.
4. Then `destroy()` (UniFFI objects are `AutoCloseable`; the lifecycle method is called `shutdown`, not `close`, for that reason). A `PosEngine` that is garbage collected without `shutdown()` cancels the engine and is logged as an error; do not rely on it.

```kotlin
val engine = withContext(Dispatchers.IO) { PosEngine.open(options) }
try { /* use it */ } finally { withContext(Dispatchers.IO) { engine.shutdown(); engine.destroy() } }
```

## 5. Threading and cancellation

The engine owns a multi-thread tokio runtime (4 workers, thread name `pos-engine`). A synchronous engine function runs in `spawn_blocking`, an asynchronous one in `spawn`, and the call awaits the `JoinHandle`. Consequences:

- Cancelling a coroutine detaches the call; it never aborts a sign in flight. A cancelled `checkout` finishes on the Rust side: afterwards the order is closed with exactly one `N` sale, or (cancelled before the first poll) the order is open with no sale. There is no half state (tested in `EndToEndTest`, both variants).
- Each call holds an `Arc<Engine>`, so `shutdown()` waits for it (the 30 s bound).
- Calls may run in parallel; the engine serialises what must be serialised (signing, SQLite).
- A task cancelled because the runtime is shutting down is `EngineClosed`.

## 6. Errors (PD54)

`PosException` (Kotlin) has four variants; the fields are named `detail`, never `message` (that would clash with `Throwable.message`).

| Engine string | Kotlin | Example |
|---|---|---|
| starts with `FDM` (after trimming) | `PosException.Fdm(detail)` | `FDM_ANSWER_PENDING\|label=N` |
| `code\|k=v\|...` with a code of `errors::ALL` or `errors::hubrise::ALL` (85 codes at 042d911) | `PosException.Coded(code, params)` | `pin_locked\|seconds=30` gives code `pin_locked`, params `{seconds=30}`; `order_no_lines` gives empty params |
| anything else, empty text, every record conversion failure (`contract: ...`) | `PosException.Internal(detail)` | `engine already open: <path>`, `contract: Order: ...` |
| a call after `shutdown()`, or a task cancelled by the runtime stopping | `PosException.EngineClosed` | |
| a panic in a call body or in a spawned task (re-raised) | UniFFI's `InternalException` (not a `PosException`) | the single Fatal channel |

Rules for the callers: `Coded` goes to the screen text of the code (`describeError` of the Vue UI is the model), `Fdm` shows the raw FDM text, `Internal` is for the technician and the log, `InternalException` is Fatal (log, stop the engine, restart the process). No new engine error code exists.

Not every refusal is an exception. The checkout family reports its refusals inside the result, as the Tauri commands do: `checkout` of an order without lines answers `CheckoutResult(success = false, error = "order_no_lines", ticket = null)` and signs nothing; `ReportResult` and the other `*Result` records carry `success` and `error` the same way. The same code is thrown as `Coded("order_no_lines")` by the commands that return `Err(String)` (for example `signPreBill` of an empty table order). Test both shapes (`EndToEndTest`). Verification helpers (`verifyMinisign`, `verifyBundle`) report failures as values too (section 9).

## 7. Records and enums (PD56)

- FFI records and enums mirror the engine types with the same serde attributes, carry `deny_unknown_fields` (except a type marked `// contract: custom` with its own conversion and test) and are converted through `serde_json` (`convert::out` engine to FFI, `convert::inp` FFI to engine). A conversion failure is `Internal("contract: ...")`: it means the two sides drifted, not a user error.
- Unsigned integers become `Long` (`i64`); times (`chrono`) are `String` as the engine prints them; `serde_json::Value` is a JSON text `String` (`modifiersJson`, `extra`); `Vec<u8>` stays bytes.
- The euro `Double` amounts of `FiscalTicket` stay `Double` (display only, the engine and Vue contract). Everything that is money to compute is in cents (`Long`).
- `checkFdmStatus` and `listFiscalEvents` have typed records built from their JSON shape (`FdmStatusReport`, `FiscalEventSummary`).
- Optional fields are nullable (`Long?`, `String?`); a `PaymentPart(method, amountCents = null)` means "the rest".
- Each record has a contract test against the testkit (`android/rust/pos-ffi/tests/ffi`).

## 8. Events (PD57)

`engine.events()` subscribes an `EventStream` to a tokio broadcast channel (capacity 256) fed by the engine's `EventSink`; subscribe **before** the call that causes the event (a broadcast has no replay). Collect in a coroutine until `next()` returns `null`, then `destroy()` the stream.

| Engine topic | `PosEvent` variant | Payload |
|---|---|---|
| `fdm-sign-messages` | `FdmSignMessages(messages)` | `SignMessages { mutation, bookingDate?, warnings, informations }` |
| `fdm-status` | `FdmStatus(health)` | `FdmHealth { connected, answered, severity, code?, mandatory, checkedAt, nextCheckAt? }`; first one right after a session opens (login), then every `fdmPollMs` while the session is open |
| `session-locked` | `SessionLocked(locked)` | `SessionLocked { reason, orderError? }` (idle timeout, `lockSession`) |
| `presence-failed` | `PresenceFailed(failure)` | `PresenceFailed { insz, clockIn, error }` |
| `webshop-order-received` | `WebshopOrderReceived(event)` | `CallbackEvent` |
| `webshop-order-updated` | `WebshopOrderUpdated(event)` | `CallbackEvent` |
| `catalog-updated` | `CatalogUpdated(event)` | `CallbackEvent` |
| `catalog-rejected` | `CatalogRejected(event)` | `CallbackEvent` |
| `catalog-awaiting-adoption` | `CatalogAwaitingAdoption(event)` | `CallbackEvent` |
| none (stream's own) | `Resync(missed)` | `missed: Long` events were dropped because the reader lagged, or a payload could not be converted (`missed = 1`, logged): re-fetch the state |

`CallbackEvent { id, resourceType, eventType, createdAt, previousState?, newState?, extra }` (`previousState`, `newState` and `extra` are JSON texts). `next()` returns `null` after `shutdown()` and stays `null`.

## 9. Helpers

Free functions, all `suspend`:

- `installInitDb(dataDir, seedSqlPath?)`: creates or migrates `<dataDir>/pos.db` and loads the SQL seed when given. It takes the engine lock meanwhile and refuses (`Internal`, "engine already open") while an engine holds it. The engine refuses a seed once the database holds fiscal events.
- `checkConfigBundle(configBundle)`: the problems the engine would find in `pos.json` (empty list: usable).
- `qrMatrix(text)`: the QR modules of a ticket verification URL, rows of booleans.
- `verifyMinisign(publicKey, message, signature)` and `verifyBundle(dir, publicKey)` (PD61): a provisioning bundle directory holds `MANIFEST.json` (`{"format":1,"files":[{"path","size","sha256"}]}`) and `MANIFEST.json.minisig`. `verifyBundle` checks the signature, then every listed file (relative path, no `..`, no absolute path, no symlink, size and SHA-256 equal) and refuses any unlisted regular file. Failures are **values**: `BundleCheck` and `MinisignCheck` carry `valid` and a `problem` string in the `code|k=v` form (`bundle_signature_invalid`, `bundle_manifest_invalid`, `bundle_path_invalid`, `bundle_file_missing`, `bundle_size_mismatch`, `bundle_hash_mismatch`, `bundle_unlisted_file`, `minisign_key_invalid`, `minisign_signature_invalid`); the texts come with the screens (P4.3). The public key is a parameter. File work runs on a dedicated thread.
- `settlePreview(totalCents, parts)` and `cashDueCents(totalCents, parts)` (PD62): how `parts` settle a sale (payment lines, cash rounding, cash to hand over), through `pos_fiscal::payment::settle`; refusals use the checkout codes `payment_invalid`, `payment_exceeds_total`, `payment_incomplete`. Kotlin never multiplies or rounds money.
- `vatSplit(lines)` (PD62): reproduces `ui/src/components/VatSummary.vue` exactly (JavaScript rounding, rows in the order A, B, C, D, only for present codes, X omitted). **VatSummary note:** it can differ by one cent from `VatCode::vat_from_inclusive`, which truncates. This is recorded and not changed: it is display only, the VAT of a ticket comes from the FDM.

## 10. Logging (PD58)

`initLogging(logDir, filter)` installs a process-wide `tracing` subscriber: a daily file `pili-pos.log.<date>` in the app-private directory Kotlin passes (90 files kept) and, on Android, logcat. A panic hook logs every panic to both sinks before the previous hook. A second call is Ok and changes nothing. `src/logging.rs` is the only file with `cfg(target_os = "android")` and the only process-wide state of the crate.

## 11. Command parity

The parity script (`scripts/android/check_command_parity.py`) checks that each of the 88 cert commands has its wrapper, its engine function and a method of the same name on `PosEngine`, with the same parameter names. The table below is generated; `scripts/android/p2_gate.sh --exit` compares it byte for byte with `python3 scripts/android/check_command_parity.py --markdown`. Regenerate it, never edit it by hand.

<!-- parity:begin -->
| Command | Engine function | Call | FFI method (Kotlin) |
| --- | --- | --- | --- |
| add_order_line | order::do_add_order_line | blocking | posEngine.addOrderLine() |
| add_order_line_with_modifiers | modifiers::do_add_order_line_with_modifiers | blocking | posEngine.addOrderLineWithModifiers() |
| backup_database | report::do_backup_database | blocking | posEngine.backupDatabase() |
| check_fdm_status | checkout::do_check_fdm_status | spawn | posEngine.checkFdmStatus() |
| check_kitchen_printer | print::do_check_kitchen_printer | spawn | posEngine.checkKitchenPrinter() |
| check_mail_status | delivery::do_check_mail_status | spawn | posEngine.checkMailStatus() |
| check_payment_terminal | eft::do_check_payment_terminal | spawn | posEngine.checkPaymentTerminal() |
| check_printer | print::do_check_printer | spawn | posEngine.checkPrinter() |
| checkout | checkout::do_checkout_split | spawn | posEngine.checkout() |
| close_unpaid | unpaid::do_close_unpaid | spawn | posEngine.closeUnpaid() |
| copy_ticket | copy::do_copy_ticket_with | spawn | posEngine.copyTicket() |
| create_order | order::do_create_order | blocking | posEngine.createOrder() |
| create_user | auth::do_create_user | blocking | posEngine.createUser() |
| deactivate_user | auth::do_deactivate_user | blocking | posEngine.deactivateUser() |
| eft_abort | eft::do_eft_abort | spawn | posEngine.eftAbort() |
| eft_check | eft::do_eft_check | spawn | posEngine.eftCheck() |
| eft_pay | eft::do_eft_pay | spawn | posEngine.eftPay() |
| end_shift | social::do_end_shift | spawn | posEngine.endShift() |
| export_run_now | export::do_export_run_now | spawn | posEngine.exportRunNow() |
| export_status | export::do_export_status | spawn | posEngine.exportStatus() |
| fdm_check_now | fdm_monitor::do_fdm_check_now | spawn | posEngine.fdmCheckNow() |
| fdm_health | fdm_monitor::do_fdm_health | blocking | posEngine.fdmHealth() |
| generate_user_x_report | report::do_generate_user_x_report | spawn | posEngine.generateUserXReport() |
| generate_x_report | report::do_generate_x_report | spawn | posEngine.generateXReport() |
| generate_z_report | report::do_generate_z_report | spawn | posEngine.generateZReport() |
| get_categories | menu::do_get_categories | blocking | posEngine.getCategories() |
| get_current_user | auth::do_get_current_user | blocking | posEngine.getCurrentUser() |
| get_integrations | integrations::do_get_integrations | blocking | posEngine.getIntegrations() |
| get_item_modifiers | modifiers::do_get_item_modifiers_localized | blocking | posEngine.getItemModifiers() |
| get_menu_items | menu::do_get_menu_items | blocking | posEngine.getMenuItems() |
| get_order | order::do_get_order | blocking | posEngine.getOrder() |
| get_settings | settings::do_get_settings | blocking | posEngine.getSettings() |
| get_table_plan | table_plan::do_get_table_plan | blocking | posEngine.getTablePlan() |
| hubrise_adopt_catalog | hubrise::do_adopt_catalog | blocking | posEngine.hubriseAdoptCatalog() |
| hubrise_adoption_preview | hubrise::do_adoption_preview | blocking | posEngine.hubriseAdoptionPreview() |
| hubrise_complete_oauth | hubrise::do_hubrise_complete_oauth | spawn | posEngine.hubriseCompleteOauth() |
| hubrise_disconnect | hubrise::do_hubrise_disconnect | blocking | posEngine.hubriseDisconnect() |
| hubrise_get_connection_status | hubrise::do_hubrise_get_connection_status | blocking | posEngine.hubriseGetConnectionStatus() |
| hubrise_list_channels | hubrise::do_list_channels | blocking | posEngine.hubriseListChannels() |
| hubrise_list_webshop_orders | hubrise::do_list_webshop_orders | blocking | posEngine.hubriseListWebshopOrders() |
| hubrise_patch_order_status | hubrise::do_patch_order_status | spawn | posEngine.hubrisePatchOrderStatus() |
| hubrise_set_channel | hubrise::do_set_channel | blocking | posEngine.hubriseSetChannel() |
| hubrise_set_confirmed_time | hubrise::do_set_confirmed_time | spawn | posEngine.hubriseSetConfirmedTime() |
| hubrise_start_oauth_flow | hubrise::do_hubrise_start_oauth_flow | blocking | posEngine.hubriseStartOauthFlow() |
| hubrise_status | hubrise::do_hubrise_status | blocking | posEngine.hubriseStatus() |
| hubrise_sync_catalog | hubrise::do_sync_catalog | spawn | posEngine.hubriseSyncCatalog() |
| leave_order | session::do_leave_order | spawn | posEngine.leaveOrder() |
| list_cart_lines | order::do_list_cart_lines | blocking | posEngine.listCartLines() |
| list_copyable_tickets | copy::do_list_copyable_tickets | blocking | posEngine.listCopyableTickets() |
| list_eft_payments | eft::do_list_eft_payments | blocking | posEngine.listEftPayments() |
| list_fiscal_events | copy::do_list_fiscal_events | blocking | posEngine.listFiscalEvents() |
| list_open_orders | order::do_list_open_orders | blocking | posEngine.listOpenOrders() |
| list_open_shifts | social::do_list_open_shifts | blocking | posEngine.listOpenShifts() |
| list_open_takeaways | tables::do_list_open_takeaways | blocking | posEngine.listOpenTakeaways() |
| list_order_lines | order::do_list_order_lines | blocking | posEngine.listOrderLines() |
| list_refundable_events | refund::do_list_refundable_events | blocking | posEngine.listRefundableEvents() |
| list_tables | tables::do_list_configured_tables | blocking | posEngine.listTables() |
| list_ticket_deliveries | delivery::do_list_ticket_deliveries | blocking | posEngine.listTicketDeliveries() |
| list_unsettled_card_payments | eft::do_list_unsettled_card_payments | blocking | posEngine.listUnsettledCardPayments() |
| list_users | auth::do_list_users | blocking | posEngine.listUsers() |
| list_z_reports | report::do_list_z_reports | blocking | posEngine.listZReports() |
| lock_session | session::do_lock_session | spawn | posEngine.lockSession() |
| login | auth::do_login | blocking | posEngine.login() |
| login_with_new_pin | auth::do_login_with_new_pin | blocking | posEngine.loginWithNewPin() |
| logout | auth::do_logout | blocking | posEngine.logout() |
| money_in_out | financial::do_money_in_out | spawn | posEngine.moneyInOut() |
| needs_z_report | report::do_needs_z_report | blocking | posEngine.needsZReport() |
| open_drawer_fiscal | financial::do_open_drawer_fiscal | spawn | posEngine.openDrawerFiscal() |
| print_missing_original | delivery::do_print_missing_original | spawn | posEngine.printMissingOriginal() |
| print_webshop_customer | print::do_print_webshop_customer | spawn | posEngine.printWebshopCustomer() |
| print_webshop_kitchen | print::do_print_webshop_kitchen | spawn | posEngine.printWebshopKitchen() |
| refund_checkout | refund::do_refund_checkout_with | spawn | posEngine.refundCheckout() |
| remote_support_running | support::do_remote_support_running | blocking | posEngine.remoteSupportRunning() |
| remote_support_status | support::do_remote_support_status | blocking | posEngine.remoteSupportStatus() |
| remove_order_line | order::do_remove_order_line | blocking | posEngine.removeOrderLine() |
| set_active_order | session::do_set_active_order | blocking | posEngine.setActiveOrder() |
| set_hubrise_enabled | integrations::do_set_hubrise_enabled | blocking | posEngine.setHubriseEnabled() |
| set_pin | auth::do_set_pin | blocking | posEngine.setPin() |
| set_table_plan | table_plan::do_set_table_plan | blocking | posEngine.setTablePlan() |
| set_training_mode | settings::do_set_training_mode | blocking | posEngine.setTrainingMode() |
| sign_pre_bill | tables::do_sign_pre_bill | spawn | posEngine.signPreBill() |
| sign_table_order | tables::do_sign_table_order | spawn | posEngine.signTableOrder() |
| start_remote_support | support::do_start_remote_support | blocking | posEngine.startRemoteSupport() |
| stop_remote_support | support::do_stop_remote_support | blocking | posEngine.stopRemoteSupport() |
| update_order_line_quantity | order::do_update_order_line_quantity | blocking | posEngine.updateOrderLineQuantity() |
| user_activity | session::do_user_activity | blocking | posEngine.userActivity() |
| webshop_accept_order | webshop::do_accept_webshop_order | spawn | posEngine.webshopAcceptOrder() |
| webshop_close_cancelled_order | webshop::do_close_cancelled_webshop_order | spawn | posEngine.webshopCloseCancelledOrder() |
| webshop_complete_order | webshop::do_complete_webshop_order | blocking | posEngine.webshopCompleteOrder() |
| webshop_handover_order | webshop::do_handover_webshop_order | spawn | posEngine.webshopHandoverOrder() |
| webshop_refund_order | webshop::do_refund_webshop_order | spawn | posEngine.webshopRefundOrder() |
| webshop_reject_order | webshop::do_reject_webshop_order | blocking | posEngine.webshopRejectOrder() |
| work_in | social::do_work_in | spawn | posEngine.workIn() |
| work_out | social::do_work_out | spawn | posEngine.workOut() |

Parity: 94/94.
<!-- parity:end -->

## 12. JVM tests, mocks and gates

`:pos-core` tests (`android/pos-core/src/test/kotlin/dev/nuagemagique/pilipos/core`) run the host library through JNA 5.19.1 (the plain jar, which carries `libjnidispatch` for macOS and Linux):

| Test class | What it covers |
|---|---|
| `LoadTest` | the library loads, contract and checksums match, `posSwVersion` |
| `LifecycleTest` | open, second open refused, shutdown, reopen, relative paths refused, stream ends with the engine, daily log file |
| `ChecksumMismatchTest` | run by the Gradle task `mismatchTest` only: bindings from the normal build against a library built with the pos-ffi feature `abi-mismatch` (`target/android-rust-mismatch`); the load must fail |
| `MocksSmokeTest` | the two mock processes start, answer and stop |
| `EndToEndTest` | sale (cash, FDM reference, `N` event, ticket printed), refund, X then Z, `needsZReport`, `FdmStatus` and `SessionLocked` events, `order_no_lines` in both shapes, calls after shutdown, cancellation, parallel calls |

Mocks (PD65): `pos-fdm-mock` and `pos-printer-mock` are built from the root workspace (`cargo build --locked --manifest-path <repo>/Cargo.toml -p pos-fdm-mock -p pos-printer-mock --target-dir <repo>/target/android-mocks`, root `Cargo.lock` untouched, task `cargoBuildMocks`) and started by the tests as processes on free ports (`Mocks.kt`; `MockProcess.stop()` kills the process tree). The end-to-end tests write a `pos.json` whose `fdm_url` and `printer_address` point at them, seed the database with `installInitDb(dataDir, src-tauri/seed_spf_test.sql)` (manager INSZ `94060242576`, PIN `1111`) and read the printed jobs from the printer mock's HTTP side (`GET /jobs`). From the Android emulator the same mocks run on the host and are reached at `10.0.2.2`.

Gates:

- `cd android && ./gradlew --no-daemon :pos-core:check`: unit, lifecycle, `mismatchTest`, end-to-end, and `apiCheck`.
- `apiCheck` compares the public Kotlin API of `:pos-core`, which includes the generated bindings (package `uniffi.pos_ffi`), with the snapshot `android/pos-core/api/pos-core.api` (binary-compatibility-validator). A change of the FFI changes the snapshot: after a deliberate change run `./gradlew --no-daemon :pos-core:apiDump` and commit the diff, which is then the review of the API change. The dump is deterministic.
- `scripts/android/p2_gate.sh --fast` (card gate), `--exit` (phase exit: strict parity 86/86, `:pos-core:check`, and `ffi-doc`, which compares section 11 with the script), `--android` (arm64 build, 16 KB alignment, bindgen on the `.so`).
