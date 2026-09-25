#!/usr/bin/env python3
"""Embed the generated ticket transcriptions into TICKETS_EXEMPLES_FR.md.

Each numbered section of the document holds one fenced block; its
content is replaced by the matching file of evidence/tickets/, so the
document always shows what the print pipeline produces.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOC = ROOT / "docs/SUBMISSION_ARTIFACTS/TICKETS_EXEMPLES_FR.md"
TICKETS = ROOT / "docs/SUBMISSION_ARTIFACTS/evidence/tickets"

# Section heading prefix → transcription file.
SECTIONS = {
    "## 1. ": "ticket_N_vente.txt",
    "## 2. ": "ticket_P_pro_forma.txt",
    "## 3. ": "ticket_N_remboursement.txt",
    "## 4. ": "ticket_C_copie.txt",
    "## 5. ": "ticket_T_training.txt",
    "## 6. ": "rapport_X.txt",
    "## 7. ": "rapport_Z.txt",
}


def main() -> int:
    text = DOC.read_text()
    for heading, name in SECTIONS.items():
        start = text.find("\n" + heading)
        if start < 0:
            print(f"missing section {heading!r}", file=sys.stderr)
            return 1
        fence = re.compile(r"```\n.*?\n```", re.S)
        match = fence.search(text, start)
        if not match:
            print(f"no code block in section {heading!r}", file=sys.stderr)
            return 1
        body = (TICKETS / name).read_text().rstrip("\n")
        text = text[: match.start()] + "```\n" + body + "\n```" + text[match.end() :]
    DOC.write_text(text)
    print(f"embedded {len(SECTIONS)} transcriptions into {DOC.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
