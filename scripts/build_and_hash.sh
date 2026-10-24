#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - certification build + integrity report
#
# Implements the five steps declared in
# docs/SUBMISSION_ARTIFACTS/GARANTIE_MAJ_LANGUES_FR.md §1.2:
#   1. Compile the certification build (HubRise in, switched per
#      installation; invoicing in every build).
#   2. Verify the binary has the HubRise integration (and report the invoicing symbols).
#   3. Verify the source compiled into the binary contains no
#      DELETE/UPDATE on fiscal_events.
#   4. Compute SHA-1, SHA-256 and MD5 of the binary.
#   5. Emit a build_info_YYYYMMDD-HHMMSS.txt report.
#
# The SHA-1 value goes on the SPF Finances request form (§I.D).
# Usage: scripts/build_and_hash.sh
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

BINARY="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}/release/pili-pos"
STAMP="$(date +%Y%m%d-%H%M%S)"
REPORT="$PROJECT_ROOT/build_info_${STAMP}.txt"

echo "── 1/5 Certification build ──────────────────────────────"
# A plain `cargo build` does not run tauri.conf.json's beforeBuildCommand:
# build the UI bundle first so it gets embedded (`custom-protocol`,
# pulled in by the `cert` feature). The bundle has the HubRise screens;
# the installation's switch decides whether they show.
# VP_GIT_HOOKS=0: `npm ci` runs `vp config` (prepare), which would point the
# checkout's git hooks at ui/.vite-hooks; a build never needs them.
(cd ui && VP_GIT_HOOKS=0 npm ci && npm run build)
(cd src-tauri && cargo build --release --no-default-features --features cert)
[[ -f "$BINARY" ]] || { echo "✗ binary not found: $BINARY"; exit 1; }

echo "── 2/5 Integration check ────────────────────────────────"
# One binary for every installation: the HubRise integration is in it
# (off unless switched on), invoicing (event I, Peppol queue) is in every
# build (V11-D1).
# Count, not `grep -q`: with pipefail, grep quitting early kills
# `strings` with SIGPIPE and the pipeline reads as a failure.
SWITCH_REFS=$(strings "$BINARY" | grep -c "integration_switch_log" || true)
if [[ "$SWITCH_REFS" -eq 0 ]]; then
    echo "✗ the HubRise integration switch is missing from the cert binary"
    exit 1
fi
PEPPOL_SYMS=$(nm "$BINARY" 2>/dev/null | grep -c pos_peppol || true)
echo "✓ HubRise integration present (switched per installation), invoicing present ($PEPPOL_SYMS pos_peppol symbols)"
# devUrl (localhost:1420) is part of the config Tauri always embeds, so
# its presence proves nothing. Without custom-protocol the bundle is not
# embedded at all: check that every built asset's path is in the binary.
BIN_STRINGS="$(mktemp)"
strings "$BINARY" > "$BIN_STRINGS"
ASSETS=(ui/dist/assets/*)
[[ -e "${ASSETS[0]}" ]] || { echo "✗ ui/dist/assets is empty"; rm -f "$BIN_STRINGS"; exit 1; }
for asset in "${ASSETS[@]}"; do
    if ! grep -qF "/assets/$(basename "$asset")" "$BIN_STRINGS"; then
        echo "✗ /assets/$(basename "$asset") is not embedded (custom-protocol missing?)"
        rm -f "$BIN_STRINGS"
        exit 1
    fi
done
rm -f "$BIN_STRINGS"
echo "✓ UI bundle embedded (${#ASSETS[@]} assets)"

echo "── 3/5 Fiscal-archive mutation scan ─────────────────────"
# The fiscal archive is append-only by law. No source file compiled
# into the cert binary may contain a DELETE or UPDATE statement
# targeting fiscal_events. Scanned: every *.rs and *.sql file under
# crates and src-tauri/src. Two kinds of test code are left out, since
# neither is compiled into the binary and both hold the tests proving
# the database refuses these statements:
#   - the Cargo integration-test directories, i.e. a `tests` directory
#     directly under a crate root (a directory with a Cargo.toml, so
#     crates/*/tests), built only by `cargo test`;
#   - `#[cfg(test)]` blocks.
# Everything else is scanned, test helpers under src/ that are not
# behind #[cfg(test)] included.
TEST_DIR_PRUNE=()
while IFS= read -r manifest; do
    crate_dir="$(dirname "$manifest")"
    if [[ -d "$crate_dir/tests" ]]; then
        TEST_DIR_PRUNE+=(-path "$crate_dir/tests" -prune -o)
    fi
done < <(find crates -name Cargo.toml -not -path '*/target/*' | sort)
MUTATIONS="$(find crates src-tauri/src ${TEST_DIR_PRUNE[@]+"${TEST_DIR_PRUNE[@]}"} \( -name '*.rs' -o -name '*.sql' \) -print | sort | xargs awk '
    FNR == 1 { armed = 0; skip = 0; depth = 0 }
    /^[[:space:]]*#\[cfg\(test\)\]/ { armed = 1; next }
    armed && !skip && /\{/ { skip = 1; depth = 0 }
    skip {
        depth += gsub(/\{/, "{") - gsub(/\}/, "}")
        if (depth <= 0) { skip = 0; armed = 0 }
        next
    }
    { armed = 0 }
    tolower($0) ~ /(delete[[:space:]]+from|update)[[:space:]]+fiscal_events/ { print FILENAME ":" FNR ": " $0 }
')"
if [[ -n "$MUTATIONS" ]]; then
    echo "$MUTATIONS"
    echo "✗ forbidden mutation of fiscal_events found in source"
    exit 1
fi
echo "✓ no DELETE/UPDATE on fiscal_events in source"

echo "── 4/5 Hashes ───────────────────────────────────────────"
SHA1=$(shasum -a 1 "$BINARY" | awk '{print $1}')
SHA256=$(shasum -a 256 "$BINARY" | awk '{print $1}')
MD5=$(md5sum "$BINARY" 2>/dev/null | awk '{print $1}' || md5 -q "$BINARY")
echo "SHA-1  : $SHA1"

echo "── 5/5 Report ───────────────────────────────────────────"
GIT_SHA=$(git rev-parse HEAD)
GIT_TAG=$(git describe --tags --exact-match 2>/dev/null || echo "(untagged)")
GIT_DIRTY=$([[ -n "$(git status --porcelain)" ]] && echo "DIRTY" || echo "clean")

cat > "$REPORT" <<REPORT_EOF
pili-pos certification build report
Generated : ${STAMP}
Host      : $(uname -srm)

Build command : cargo build --release --no-default-features --features cert
Toolchain     : $(rustc --version)
Git commit    : ${GIT_SHA}
Git tag       : ${GIT_TAG}
Working tree  : ${GIT_DIRTY}

Binary        : ${BINARY}
Size          : $(wc -c < "$BINARY" | tr -d ' ') bytes
SHA-1         : ${SHA1}
SHA-256       : ${SHA256}
MD5           : ${MD5}

Checks
  HubRise integration ............ present (installation switch, off by default)
  pos_peppol symbols ............. ${PEPPOL_SYMS}
  fiscal_events DELETE/UPDATE .... none in source
REPORT_EOF

cat "$REPORT"
echo ""
echo "✓ report written to $REPORT"
if [[ "$GIT_DIRTY" == "DIRTY" ]]; then
    echo "⚠ working tree is dirty - do NOT declare this hash to the SPF;"
    echo "  commit, tag (vX.Y.Z, GPG-signed) and rebuild first."
fi
