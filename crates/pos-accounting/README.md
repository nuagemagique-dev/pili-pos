# pos-accounting

`pili-accounting`: accountant export from a copy of the till's `pos.db`. A stand-alone
command line tool, **outside the certified binary** (decision V12-D5): no shipped crate
(`pos-engine`, `src-tauri`, `android/rust/pos-ffi`) depends on it, and it depends on none
of the POS crates at run time. `tests/isolation.rs` enforces both.

```
pili-accounting --db <pos.db | SPF export folder> (--month YYYY-MM | --from D --to D)
                [--out DIR] [--accounts accounts.json]
                [--profile generic|winbooks|yuki] [--decimal comma|point] [--lang fr]
```

## What it reads

Only the signed archive: `fiscal_events` rows that have an FDM answer (labels N, I, F, R),
`z_reports` and `booking_periods`. Never `orders`, `order_lines`, `payments` or any other
unsigned state. Amounts are the signed request's lines and payments and the FDM's
`vatCalc`; nothing is recomputed from the menu. The channel comes from the signed cost
center (`TABLE` dine-in, none takeaway, other online). Training (T), copies (C), pro forma
(P), social (S) and X reports are never read.

The database is opened with `mode=ro&immutable=1`: nothing is written, no journal or WAL
file is created. `--db` may be a folder written by the till's export (its
`MANIFEST.sha256` is then checked; it must list `pos.db`, in the `sha256sum` text or
`--binary` form). A non-empty `pos.db-wal` beside the file is refused: it
means a copy of a live database that misses what the WAL still holds. A database without
migration 4 (append-only archive) is refused.

## Reconciliation

Only booking periods closed by a real Z are exported (open ones are named in
`LISEZMOI.txt`). Each period is checked against its Z report before anything is written:
ticket count, turnover, VAT per label (base, VAT, total), payments per payment type and
amount type. Any gap prints `RECONCILIATION_FAILED <day, Z number, field>`, exits with
code 2 and writes nothing.

## Output

`<out>/export-comptable-<from>-<to>/` (never overwrites; written under a `.partial` name
and renamed): `jours.csv`, `tva.csv`, `paiements.csv`, `remboursements.csv`,
`factures.csv`, `bons_cadeaux.csv`, `mouvements_caisse.csv`, `rapports_z.csv`,
`journal.csv`, `LISEZMOI.txt` (French) and `MANIFEST.sha256`. CSV files are UTF-8 with a
BOM, `;` separated, CRLF terminated, decimal comma by default, with an integer `_centimes`
column next to every amount. No national number, no employee name; the customer name and
VAT number of an invoice appear only in `factures.csv`.

The journal has one balanced entry per booking period (document number = Z number).
Default accounts follow the Belgian chart of accounts (see `src/journal.rs`) and are
overridden by `--accounts` (a JSON file; unknown keys are refused). The `generic` profile
is the reference layout. `winbooks` and `yuki` follow public import descriptions and are
best effort: confirm them with a test import at the accountant.

## Exit codes

0 done; 1 error (`db_unreadable`, `not_a_pili_db`, `manifest_mismatch`, `period_invalid`,
`period_open`, `empty_range`, `accounts_invalid`, `out_exists`, `event_unreadable`,
`journal_unbalanced`, `io_error`); 2 reconciliation failed.

## Tests

`cargo test -p pos-accounting`. `tests/reconcile_report_day.rs` builds real archives with
`pos-engine-testkit` (the Reports 1.3 day, a mixed day on the admission mock FDM);
`tests/synthetic.rs` covers golden files (`tests/golden/`, regenerate with
`UPDATE_GOLDEN=1` and review the diff by hand), tampering, read-only proof, the source
guards and the M150 invoice; `tests/cli.rs` the exit codes; `tests/isolation.rs` V12-D5.

The meal voucher method code is `cheques_repas` (CSV and accounts file). An accounts file
that still uses the former key `payments.titres_repas` is read as `cheques_repas`.
