#!/usr/bin/env bash
#
# build_and_hash.sh - construction reproductible + calcul SHA-1
#
# Génère le binaire de certification et calcule son empreinte SHA-1
# à consigner dans le formulaire de demande de certification SPF
# Finances (§I.D).
#
# Usage:
#   ./build_and_hash.sh
#
# Sortie:
#   build_info_YYYYMMDD-HHMMSS.txt
#   contenant le hash SHA-1, la version, le commit git, la toolchain.
#
set -euo pipefail

cd "$(dirname "$0")/../.."

TIMESTAMP="$(date -u +%Y%m%d-%H%M%S)"
OUT="docs/SUBMISSION_ARTIFACTS/build_info_${TIMESTAMP}.txt"

echo "=== Construction de la build de certification ===" | tee "$OUT"
echo "Date : $(date -u --iso-8601=seconds)" | tee -a "$OUT"
echo "Hôte : $(hostname)" | tee -a "$OUT"
echo "" | tee -a "$OUT"

echo "--- Environnement ---" | tee -a "$OUT"
echo "Toolchain Rust :" | tee -a "$OUT"
rustc --version | tee -a "$OUT"
cargo --version | tee -a "$OUT"
echo "" | tee -a "$OUT"

echo "--- État git ---" | tee -a "$OUT"
echo "Branche : $(git rev-parse --abbrev-ref HEAD)" | tee -a "$OUT"
echo "Commit  : $(git rev-parse HEAD)" | tee -a "$OUT"
echo "Tag     : $(git describe --tags --always)" | tee -a "$OUT"
if [ -n "$(git status --porcelain)" ]; then
    echo "⚠️  ATTENTION : working tree non propre - build NON reproductible" | tee -a "$OUT"
    echo "Fichiers modifiés :" | tee -a "$OUT"
    git status --short | tee -a "$OUT"
else
    echo "✓ Working tree propre" | tee -a "$OUT"
fi
echo "" | tee -a "$OUT"

echo "--- Build de certification ---" | tee -a "$OUT"
cargo build --release --no-default-features --features cert 2>&1 | tail -5 | tee -a "$OUT"
echo "" | tee -a "$OUT"

BINARY="src-tauri/target/release/moyun-pos"
if [ ! -x "$BINARY" ]; then
    echo "❌ Binaire introuvable à $BINARY" | tee -a "$OUT"
    exit 1
fi

echo "--- Vérifications anti-régression ---" | tee -a "$OUT"
HUBRISE_COUNT="$(nm "$BINARY" 2>/dev/null | grep -ic hubrise || true)"
echo "Symboles hubrise dans le binaire : $HUBRISE_COUNT (attendu : 0)" | tee -a "$OUT"
if [ "$HUBRISE_COUNT" -ne 0 ]; then
    echo "❌ HubRise présent dans la build cert - refaire la build" | tee -a "$OUT"
    exit 1
fi

GREP_DELETE="$(grep -r 'DELETE FROM fiscal_events\|UPDATE fiscal_events' src-tauri/ crates/ 2>/dev/null | wc -l || true)"
echo "DELETE/UPDATE sur fiscal_events : $GREP_DELETE (attendu : 0)" | tee -a "$OUT"
if [ "$GREP_DELETE" -ne 0 ]; then
    echo "❌ Mutation interdite trouvée - voir DECLARATION_FONCTIONS_INTERDITES_FR.md" | tee -a "$OUT"
    exit 1
fi
echo "" | tee -a "$OUT"

echo "--- Empreintes du binaire certifié ---" | tee -a "$OUT"
echo "Chemin  : $BINARY" | tee -a "$OUT"
echo "Taille  : $(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY") octets" | tee -a "$OUT"
echo "SHA-1   : $(shasum -a 1 "$BINARY" | awk '{print $1}')" | tee -a "$OUT"
echo "SHA-256 : $(shasum -a 256 "$BINARY" | awk '{print $1}')" | tee -a "$OUT"
echo "MD5     : $(md5sum "$BINARY" 2>/dev/null | awk '{print $1}' || md5 -q "$BINARY")" | tee -a "$OUT"
echo "" | tee -a "$OUT"

echo "=== TERMINÉ ===" | tee -a "$OUT"
echo "" | tee -a "$OUT"
echo "Valeur SHA-1 à reporter dans le formulaire SPF Finances §I.D :"
shasum -a 1 "$BINARY" | awk '{print $1}'
echo ""
echo "Rapport complet : $OUT"
