#!/usr/bin/env bash
# Regenerate the SPF submission PDFs from the FR markdown annexes.
# Requires: pandoc + xelatex + the newunicodechar LaTeX package
# (user-tree install: `tlmgr init-usertree && tlmgr --usermode install newunicodechar`).
# Usage: scripts/make_dossier_pdfs.sh [FILE.md ...]   (default: all *_FR.md)
set -euo pipefail

DIR="$(cd "$(dirname "$0")/../docs/SUBMISSION_ARTIFACTS" && pwd)"
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
        -H pdf_header.tex \
        -V geometry:margin=2.2cm \
        -V mainfont="Helvetica Neue" \
        -V monofont="Menlo" \
        -V fontsize=10pt
done
echo "✓ ${#files[@]} PDF(s) in $DIR/pdf_out/"
