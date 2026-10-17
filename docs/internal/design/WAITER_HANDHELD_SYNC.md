# Waiter handhelds synchronised with the main till (design note)

Status: design only, no code. Target: a later certified update after v1.0.0.
Audience: the planner of the next lane. Language: English (internal).

## 1. Goal

Waiters take orders on handhelds at the table; the orders land on the same tables as the main till, the
kitchen receives them at once, and the cashier bills from the main till. Several devices may touch the same
table. This note is the transport and consistency layer that `KITCHEN_DISPLAY.md` (separate display device)
and `QR_TABLE_ORDERING.md` (guest orders) also lean on.

## 2. What the spec requires

| Source | Requirement |
| --- | --- |
| `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` section 6.2 (around lines 534 to 540) | "Si un SCE comprend plusieurs caisses enregistreuses sous le meme posId, un identifiant de terminal unique doit etre utilise pour chaque caisse." The terminal concept is independent of hardware. |
| Same file, field `terminalId` (around line 1100) | Mandatory, 1 to 600 characters, unique per till under one posId. |
| Same file, field `posFiscalTicketNo` (around line 1088) | Value 1 to 999999999, may be generated globally, per event type or per terminal, as long as the combination with the event type or the terminal id is unique. |
| Same file, section on the `posId` (around lines 1079 to 1083) | One posId per installation (14 characters); the certification posId is reserved. |
| `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` art. 43 and 44 | Registered data cannot be changed after the fact; the ticket sequence and the FDM sequence are consecutive and without gaps per FDM. |
| Certification manual, FDM section | An FDM serves one POS installation; the sequence number of the FDM (fiscal event counters) is global to the FDM, so two devices must never sign at the same time. |

The decisive consequence: one FDM, one signing sequence. Whatever the number of handhelds, **all signing
happens on one machine** (the main till, which owns the FDM link and the database).

## 3. Today in the code

- `crates/pos-engine/src/commands/signing.rs` serialises every signing behind one async mutex (`SignGuard`,
  `acquire`) and the pending-sign table makes a crashed signing recoverable (`pending_signs`).
- `commands/tables.rs`: `do_sign_table_order` signs the P ORDER when the waiter leaves a table; the table's
  lines are the open order's `order_lines`; a drained order hand-over uses the same guard so no line slips in.
- `pos_fiscal::archive` stores `pos_id` and `terminal_id` on every event; `terminal_id` today is one value from
  settings. `posFiscalTicketNo` comes from the global counter (`counter.rs`).
- Android: the POS app is a single-device app (engine embedded through UniFFI, its own database, its own FDM
  link or its own FDM through the Sunmi USB path, per the Android phase documents). There is no network
  server in the engine; the Linux shell listens only on `127.0.0.1` for its health check.
- Users: `users` with roles, PIN login; `Access` levels per command in `pos-engine` (`access_control.rs`).
- The legacy tsb-service in the wider repository has a `/pos/*` module for a different handheld product; it is
  not part of this engine and is not reused.

## 4. Options

| Option | Idea | Verdict |
| --- | --- | --- |
| A. Thin clients | Handhelds are screens only; every command goes to the main till over the LAN. | Recommended. One database, one signing queue, no merge problem. |
| B. Replicated databases | Each handheld has its own engine and database, syncs by log shipping. | Rejected: signing and counters cannot be replicated safely; offline signing would need its own FDM. |
| C. Cloud broker | Handhelds and till talk through a cloud service. | Rejected for v1: adds a dependency and a data-protection surface for no fiscal benefit. |

So: option A, the main till runs a **local server mode** and handhelds are thin clients.

## 5. Proposal (option A)

### 5.1 Server mode on the main till

- A new crate `pos-lan` (or a module of `pos-engine`) exposes the existing engine commands over an
  authenticated API on the LAN: HTTPS with a certificate generated at pairing (pinned by the handheld), or a
  WebSocket with the same TLS. It is **off by default** and enabled by a manager in settings.
- It exposes a **whitelist of commands** (the waiter set: list tables, open or extend an order, add/remove/
  correct lines, move lines between tables, send to kitchen, sign the table order, request the pre-bill,
  sold-out list, read the menu) and never the manager or fiscal set (Z, reports, settings, user management,
  archive export). The server reuses `Access` levels: a handheld session is a waiter session.
- A push channel (server to handheld) carries table changes, sold-out changes and "ready" notifications, so
  handhelds never poll. The Android UI already consumes a `PosEvent` stream; the same events are serialised.

### 5.2 Pairing and identity

- A handheld pairs once: the manager shows a short-lived pairing code on the main till (QR with the till's
  address, certificate fingerprint and a single-use secret); the handheld stores a device key and a name.
- Each paired device gets a `terminalId` ("handheld-1", "terrasse"), used in the optional per-device
  attribution of lines (who ordered what) and in the audit trail. Because signing is on the main till, the
  `terminalId` sent to the FDM stays the till's, unless the owner declares the handheld as a terminal in the
  dossier. Recommendation: keep one fiscal `terminalId` (the main till) and record the handheld as a **device
  attribute** on `order_lines`, so the certified perimeter does not change. Open question Q1 below.
- Login on the handheld is the usual user PIN; the session token is bound to the device and the user, short
  lived, revoked when the device is unpaired.

### 5.3 Consistency

- All writes go to the main till's engine and are serialised by its existing mutex; the handheld has no
  authoritative state. Each write carries a client `request_id` so a retry after a network hiccup is
  idempotent (a replay returns the first result).
- Concurrent edits on one table: lines are appended (no overwrite), so two waiters adding items merge
  naturally. Removing a line that another device just sent is a conflict: the server answers with the current
  line set and the UI refreshes. A table being **signed** (drain in progress) rejects new lines with the
  existing "retry" error.
- Offline handheld: the handheld may queue lines locally while the Wi-Fi is down and flush them when back; it
  must show them as "not sent". It never signs and never prints a kitchen slip by itself (the slip comes from
  the till after the P ORDER). Order of arrival defines the order on the ticket.
- Main till down: handhelds are read-only with an explicit banner (no hidden local sales). A fallback of a
  handheld becoming the till is out of scope (the FDM is attached to one machine).

### 5.4 Printing and the kitchen

Handheld orders are signed on the till, so `bar_slip` and the kitchen display (see `KITCHEN_DISPLAY.md`) work
unchanged. A handheld may also trigger a provisional bill print on the till's ticket printer, or a Bluetooth
print on the handheld if the owner wants it (a non-fiscal pro-forma only; never a VAT ticket).

### 5.5 Security

TLS with pinned certificate, device keys revocable, per-command whitelist, rate limits, no listener when the
setting is off, binding to the LAN interface only, audit lines for pairing and unpairing, and a settings
warning if the till is on an open or guest Wi-Fi. No card data crosses this API (EFT stays on the till).

## 6. Data model

Next free migration after 42 (a v1.2 number): `paired_devices(id, name, key_hash, created_at, last_seen,
revoked_at NULL)`, `device_sessions`, `order_lines.device_id NULL`, `request_log(request_id, result_json,
created_at)` for idempotency (pruned after a day). Settings: server mode on/off, port, certificate. None of it
touches fiscal tables.

## 7. Fiscal events and payloads

No new event. The `terminalId` field is the only one that could change, and the recommendation is that it
does not (Q1). If the owner declares each handheld as a terminal, `terminalId` becomes the device name for P
events signed on behalf of that handheld, and `posFiscalTicketNo` must stay unique per (type, terminal) pair;
today it is global and therefore already satisfies the rule. The dossier must then list the terminals.

## 8. UI

- Vue (main till): settings screen "Handhelds" (enable, pairing code, list, revoke), a small device badge on
  lines in the order view, a banner when a handheld is connected.
- Android: two build flavours of the same app, or one app with a mode switch: **till** (embedded engine, as
  today) and **handheld** (thin client: restricted waiter UI, no engine, no FDM link, no Reports tab).
  The handheld flavour reuses the Compose screens through a repository interface with two implementations
  (engine and remote). Strings fr, en, zh; parity rows added to `docs/android/parity`.

## 9. Tests

Engine: whitelist enforced (a manager command from a device is refused), idempotent replay, two devices
adding to one table, remove-versus-add conflict, signing while a handheld sends a line, revoked device, pairing
expiry. Network: reconnect with queued lines, certificate mismatch refusal. UI: handheld flows with the remote
repository faked. A soak test with 3 handhelds and 200 orders (reuse the existing soak harness).

## 10. Effort (worker-days)

| Layer | Estimate |
| --- | --- |
| Server mode, TLS, pairing, whitelist, push channel | 12 |
| Idempotency, conflicts, offline queue | 6 |
| Database and settings | 2 |
| Android thin-client flavour (repository split, screens) | 12 |
| Vue (settings, badges) | 3 |
| Tests and soak | 8 |
| Security review | 3 |
| Dossier (perimeter, terminals), runbook | 2.5 |
| Total | about 48.5 |

## 11. Risks

- Fiscal perimeter creep: a network API on a certified till is a new attack surface the SPF may question; keep
  it off by default, whitelisted, and described in the dossier.
- Wi-Fi quality in a restaurant is poor; the offline queue and banners are not optional.
- Two truths: any handheld cache that looks authoritative will drift; the design keeps it read-through.
- Android single-device assumptions (embedded engine, one database) run through the app code; the repository
  split is the bulk of the Android work.
- Pairing and certificates are operational burdens for a non-technical owner; the pairing code must be as
  simple as a QR scan.

## 12. Open questions

- Q1. Does the owner (and the SPF) want each handheld declared as its own terminal, or only one fiscal terminal
  with a device attribute? The second keeps the certified perimeter unchanged.
- Q2. How many handhelds, and is a tablet in the kitchen (see `KITCHEN_DISPLAY.md`) the same API client?
- Q3. Is Bluetooth printing on the handheld wanted, or does everything print on the till?
- Q4. Must a handheld work with the main till down (it cannot sign; confirm the read-only fallback is acceptable)?
