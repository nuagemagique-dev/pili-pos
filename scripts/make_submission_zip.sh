#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos - SCE 2.0 submission ZIP
#
# 1. Regenerates the dossier PDFs (scripts/make_dossier_pdfs.sh) and
#    the screenshot PDF (scripts/make_captures_pdf.sh).
# 2. Checks the dossier with scripts/check_dossier.py --final: code
#    references, brand-neutral wording, placeholders, screenshots, PDF
#    freshness, MANIFEST. Any finding stops here.
# 3. Zips exactly what docs/SUBMISSION_ARTIFACTS/MANIFEST.txt lists:
#    the PDFs of pdf_out/ at the root of the ZIP, the listed folders
#    as they are, never a .md, a script or a .DS_Store.
#
# Output: target/submission/moyun-pos-dossier-v<version>.zip
# Usage: scripts/make_submission_zip.sh [--skip-pdfs]
#   --skip-pdfs  reuse the PDFs already built (the check still refuses
#                a PDF older than its markdown).
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOSSIER="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS"
cd "$PROJECT_ROOT"

if [[ "${1:-}" != "--skip-pdfs" ]]; then
    echo "── 1/3 PDFs ─────────────────────────────────────────────"
    scripts/make_dossier_pdfs.sh
    scripts/make_captures_pdf.sh
fi

echo "── 2/3 Dossier check (--final) ──────────────────────────"
python3 scripts/check_dossier.py --final

echo "── 3/3 ZIP ──────────────────────────────────────────────"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
OUT_DIR="$PROJECT_ROOT/target/submission"
ZIP="$OUT_DIR/moyun-pos-dossier-v${VERSION}.zip"
mkdir -p "$OUT_DIR"
rm -f "$ZIP"

python3 - "$DOSSIER" "$ZIP" <<'PY'
import sys
import zipfile
from pathlib import Path

dossier, zip_path = Path(sys.argv[1]), Path(sys.argv[2])


def excluded(path: Path) -> bool:
    return path.suffix == ".md" or path.name == ".DS_Store" or path.suffix in (".sh", ".py")


def archive_name(rel: Path) -> str:
    # The annex PDFs sit at the root of the ZIP.
    return rel.name if rel.parts[0] == "pdf_out" else rel.as_posix()


entries = []
for line in (dossier / "MANIFEST.txt").read_text(encoding="utf-8").splitlines():
    entry = line.strip()
    if not entry or entry.startswith("#"):
        continue
    path = dossier / entry
    if path.is_dir():
        entries += sorted(p for p in path.rglob("*") if p.is_file() and not excluded(p))
    elif path.is_file() and not excluded(path):
        entries.append(path)
    else:
        sys.exit(f"MANIFEST entry not found or not allowed: {entry}")

names = set()
with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
    for path in entries:
        name = archive_name(path.relative_to(dossier))
        if name in names:
            sys.exit(f"two MANIFEST entries end up as {name}")
        names.add(name)
        zf.write(path, name)
print(f"{len(names)} file(s)")
PY

echo ""
echo "✓ $ZIP"
echo "  SHA-256: $(shasum -a 256 "$ZIP" | awk '{print $1}')"
