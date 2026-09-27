#!/usr/bin/env python3
"""check-not-blank.py - fail if a screendump is a uniform/blank frame.

Used by .github/workflows/image.yml's `boot-smoke` job right after
scripts/qemu/screendump.sh: a screenshot of a black, unrendered, or
single-color frame (the kiosk never actually reached a rendered UI -
KMS/Cage crash, pili-pos never started, VGA still at the firmware
splash) has zero, or near-zero, brightness spread; a real rendered UI
frame does not.

A spread alone is weak: a black console with one cursor passes it.
--expect-color RRGGBB --min-fraction F also requires that color (the
app's own background, ui/src/styles/tokens.css --bg-primary) on at
least F of the frame, which a text console never has.

Usage: check-not-blank.py <image.png> [--min-spread N]
                          [--expect-color RRGGBB --min-fraction F]
Exit 0 if the frame looks like a real UI frame, 1 if not, 2 on a
usage/dependency error.
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
    parser.add_argument("--expect-color", help="RRGGBB that must cover part of the frame")
    parser.add_argument(
        "--min-fraction",
        type=float,
        default=0.3,
        help="share of the frame --expect-color must cover (default: 0.3)",
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

    if args.expect_color:
        want = tuple(int(args.expect_color[i : i + 2], 16) for i in (0, 2, 4))
        rgb = Image.open(args.image).convert("RGB")
        total = rgb.size[0] * rgb.size[1]
        hits = sum(n for n, color in rgb.getcolors(maxcolors=total) if color == want)
        share = hits / total
        print(f"#{args.expect_color}: {share:.1%} of the frame (need {args.min_fraction:.0%})")
        if share < args.min_fraction:
            print(
                f"ERROR: the app's background #{args.expect_color} covers only {share:.1%}"
                " of the frame: the kiosk UI is not on screen",
                file=sys.stderr,
            )
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
