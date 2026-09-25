#!/usr/bin/env bash
#
# build_pdfs.sh - convertit tous les .md FR du dossier en PDF
#
# Pré-requis :
#   - pandoc (`brew install pandoc`)
#   - BasicTeX (`brew install --cask basictex`)
#   - paquet LaTeX xecjk (`sudo /Library/TeX/texbin/tlmgr install xecjk`)
#
# Sortie : un PDF par .md dans docs/SUBMISSION_ARTIFACTS/pdf_out/
#
set -euo pipefail

export PATH="/Library/TeX/texbin:$PATH"

cd "$(dirname "$0")/.."

OUT="pdf_out"
mkdir -p "$OUT"

# Polices choisies pour couverture Unicode sur macOS :
#   - Lucida Grande     : flèches →, check ✓, glyphes admin
#   - Menlo             : box-drawing ─ │ ┌ ┘ (schémas ASCII)
#   - Heiti SC          : caractères chinois simplifiés 中文
MAINFONT="Arial Unicode MS"
MONOFONT="Menlo"
CJKFONT="Heiti SC"

FAIL=0
for md in *.md; do
    pdf="$OUT/${md%.md}.pdf"
    echo "→ $md"
    if pandoc "$md" -o "$pdf" \
        --pdf-engine=xelatex \
        -V lang=fr \
        -V geometry:a4paper \
        -V geometry:margin=2.5cm \
        -V mainfont="$MAINFONT" \
        -V monofont="$MONOFONT" \
        -V CJKmainfont="$CJKFONT" \
        -V colorlinks=true \
        -V linkcolor=blue \
        -V toccolor=blue \
        --toc \
        --toc-depth=2 \
        2>&1 | grep -E "(Error|Missing character)" | head -5; then
        true
    fi

    if [ ! -f "$pdf" ]; then
        echo "  ❌ ÉCHEC : $pdf non produit"
        FAIL=$((FAIL+1))
    fi
done

echo ""
if [ "$FAIL" -eq 0 ]; then
    echo "✓ Tous les PDF sont produits dans $OUT/"
    ls -la "$OUT/"
else
    echo "❌ $FAIL fichier(s) en échec"
    exit 1
fi
