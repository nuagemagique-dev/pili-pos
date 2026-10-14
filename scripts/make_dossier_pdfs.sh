#!/usr/bin/env bash
# Regenerate the SPF submission PDFs from the FR markdown annexes.
# Requires: pandoc + xelatex + the newunicodechar LaTeX package
# (user-tree install: `tlmgr init-usertree && tlmgr --usermode install newunicodechar`).
# Usage: scripts/make_dossier_pdfs.sh [FILE.md ...]   (default: all *_FR.md)
# Another dossier: PILI_DOSSIER_DIR=docs/android/dossier scripts/make_dossier_pdfs.sh (the LaTeX header of the
# Linux dossier is used when the dossier has none).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$(cd "$ROOT_DIR/${PILI_DOSSIER_DIR:-docs/SUBMISSION_ARTIFACTS}" && pwd)"
HEADER="$DIR/pdf_header.tex"
[[ -f "$HEADER" ]] || HEADER="$ROOT_DIR/docs/SUBMISSION_ARTIFACTS/pdf_header.tex"
cd "$DIR"
mkdir -p pdf_out

files=("$@")
if [[ ${#files[@]} -eq 0 ]]; then
    files=(*_FR.md)
fi

for md in "${files[@]}"; do
    base="$(basename "$md" .md)"
    echo "→ $base"
    pandoc "$base.md" -o "pdf_out/$base.pdf" \
        --pdf-engine=xelatex \
        -H "$HEADER" \
        -V geometry:margin=2.2cm \
        -V mainfont="Helvetica Neue" \
        -V monofont="Menlo" \
        -V fontsize=10pt
done
echo "✓ ${#files[@]} PDF(s) in $DIR/pdf_out/"
