#!/usr/bin/env bash
# Regenerate the §VI ticket transcriptions from the real print
# pipeline (crates/pos-print/tests/ticket_examples.rs). Output goes to
# docs/SUBMISSION_ARTIFACTS/evidence/tickets/; TICKETS_EXEMPLES_FR.md
# embeds these files — re-embed after regenerating if layouts changed.
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

OUT="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS/evidence/tickets"
TICKET_EXAMPLES_DIR="$OUT" cargo test -p pos-print --test ticket_examples

echo "✓ transcriptions in $OUT"
ls -1 "$OUT"
