# Lane int: dossier fold (2026-10-18)

Folded `docs/internal/v1.2/` into both v1.0 dossiers (see INT-D25 to INT-D27).

## Captures to retake

- Settings panels: owner alerts, kitchen courses, gift vouchers.
- Order screen: allergen bar and mode, course chips, "Envoyer la suite", voucher button and voucher line.
- Checkout: voucher dialog, held-course warning.
- Reports: "Bons cadeaux" tab.
- Menu editor: allergen editor and course picker.
- Table plan: "Suite : Plats".
- Printed tickets: voucher, voucher balance, allergen sheet, course slips.

## Still to do at the final commit

- Regenerate the RAPPORT_TESTS_BASELINE totals.
- Android EV-06 and EV-08 stay pending (device evidence).
- Rebuild the PDFs (`scripts/make_dossier_pdfs.sh`, and with `PILI_DOSSIER_DIR=docs/android/dossier`).
