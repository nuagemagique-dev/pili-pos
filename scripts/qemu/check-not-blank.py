#!/usr/bin/env python3
"""check-not-blank.py - fail if a screendump is a uniform/blank frame.

Used by .github/workflows/image.yml's `boot-smoke` job right after
scripts/qemu/screendump.sh: a screenshot of a black, unrendered, or
single-color frame (the kiosk never actually reached a rendered UI -
KMS/Cage crash, moyun-pos never started, VGA still at the firmware
splash) has zero, or near-zero, brightness spread; a real rendered UI
frame does not.

Usage: check-not-blank.py <image.png> [--min-spread N]
Exit 0 if the frame has enough brightness spread to be a real UI
frame, 1 if it looks blank/uniform, 2 on a usage/dependency error.
"""

import argparse
import sys

try:
    from PIL import Image
except ImportError:
    print("ERROR: Pillow not installed (pip/apt install python3-pil)", file=sys.stderr)
    sys.exit(2)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    parser.add_argument(
        "--min-spread",
        type=int,
        default=8,
        help="minimum (max-min) grayscale value across the frame (default: 8)",
    )
    args = parser.parse_args()

    try:
        img = Image.open(args.image).convert("L")
    except OSError as exc:
        print(f"ERROR: could not open {args.image}: {exc}", file=sys.stderr)
        return 2

    lo, hi = img.getextrema()
    spread = hi - lo
    print(f"{args.image}: {img.size[0]}x{img.size[1]}, min={lo}, max={hi}, spread={spread}")

    if spread < args.min_spread:
        print(
            f"ERROR: frame looks blank/uniform (spread {spread} < {args.min_spread})",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
