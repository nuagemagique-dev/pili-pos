#!/usr/bin/env bash
# Regenerate the §VI ticket transcriptions from the real print
# pipeline (crates/pos-print/tests/ticket_examples.rs). Output goes to
# docs/SUBMISSION_ARTIFACTS/evidence/tickets/, then embedded into
# TICKETS_EXEMPLES_FR.md (scripts/embed_ticket_examples.py).
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

OUT="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS/evidence/tickets"
TICKET_EXAMPLES_DIR="$OUT" cargo test -p pos-print --test ticket_examples

python3 "$PROJECT_ROOT/scripts/embed_ticket_examples.py"

echo "✓ transcriptions in $OUT"
ls -1 "$OUT"
