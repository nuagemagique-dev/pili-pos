#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos - certification dossier screenshot PDF
#
# Assembles docs/SUBMISSION_ARTIFACTS/screenshots/*.png and *.jpg,
# in filename order (01_..., 09a_..., 20_..., 50_...), into
# docs/SUBMISSION_ARTIFACTS/CAPTURES_ECRAN_FR.pdf with ImageMagick 7.
#
# See docs/SUBMISSION_ARTIFACTS/CAPTURES_ECRAN_RUNBOOK_FR.md for how
# the screenshots are produced (mostly `cd ui && npm run captures`).
# Usage: scripts/make_captures_pdf.sh
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOSSIER="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS"
SCREENSHOTS_DIR="$DOSSIER/screenshots"
OUT="$DOSSIER/CAPTURES_ECRAN_FR.pdf"

if ! command -v magick >/dev/null 2>&1; then
    echo "✗ magick (ImageMagick 7) not found - install it first" >&2
    exit 1
fi

shopt -s nullglob
files=("$SCREENSHOTS_DIR"/*.png "$SCREENSHOTS_DIR"/*.jpg)
shopt -u nullglob

if [[ ${#files[@]} -eq 0 ]]; then
    echo "✗ no screenshot found in $SCREENSHOTS_DIR" >&2
    echo "  run 'cd ui && npm run captures' first (see CAPTURES_ECRAN_RUNBOOK_FR.md)" >&2
    exit 1
fi

# Every path shares the same directory prefix, so a plain sort of the
# full paths sorts by basename: 01_... before 09a_..., 20_... and 50_...
IFS=$'\n' sorted=($(printf '%s\n' "${files[@]}" | sort))
unset IFS

echo "── Compiling ${#sorted[@]} screenshot(s) ────────────────"
magick "${sorted[@]}" "$OUT"
echo "✓ $OUT"
