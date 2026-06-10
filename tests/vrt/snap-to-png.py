#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
#
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Render insta `.snap` terminal snapshots to PNG for Visual Regression
# Tracker (VRT is image-only; TUI snapshots are a monospace character grid).
# Output is byte-deterministic for a given (Pillow version, font file, font
# size) — pin all three in CI so a baseline only changes when the snapshot
# text changes, never because of a renderer/font drift.
#
# Usage:
#   snap-to-png.py --snap-dir <dir> --out-dir <dir> --font <ttf> [--font-size N]
#
# Each `<snap-dir>/<rel>/<name>.snap` renders to `<out-dir>/<rel>/<name>.png`.
# The insta YAML frontmatter (between the first two `---` lines) is stripped;
# only the captured terminal body is drawn.

import argparse
import os
import sys

from PIL import Image, ImageDraw, ImageFont

# Fixed render parameters — part of the determinism contract. Changing any of
# these intentionally refreshes every TUI baseline.
PADDING = 8
BG = (13, 17, 23)
FG = (220, 223, 228)


def snapshot_body(text):
    """Return the captured terminal body, dropping insta's YAML frontmatter.

    insta writes `---\\n<frontmatter>\\n---\\n<body>`. A snapshot with no
    frontmatter (no leading `---`) is returned unchanged.
    """
    lines = text.split("\n")
    if not lines or lines[0].strip() != "---":
        return text
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            body = "\n".join(lines[i + 1 :])
            return body[1:] if body.startswith("\n") else body
    return text


def render(body, font, cell_w, cell_h):
    rows = body.split("\n")
    while rows and rows[-1] == "":
        rows.pop()
    cols = max((len(r) for r in rows), default=1)
    width = cols * cell_w + 2 * PADDING
    height = max(len(rows), 1) * cell_h + 2 * PADDING
    img = Image.new("RGB", (width, height), BG)
    draw = ImageDraw.Draw(img)
    for y, row in enumerate(rows):
        # Place each cell at a fixed column so box-drawing glyphs stay aligned
        # regardless of any per-glyph advance-width quirks.
        for x, ch in enumerate(row):
            if ch != " ":
                draw.text((PADDING + x * cell_w, PADDING + y * cell_h), ch,
                          font=font, fill=FG)
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--snap-dir", required=True)
    ap.add_argument("--out-dir", required=True)
    ap.add_argument("--font", required=True)
    ap.add_argument("--font-size", type=int, default=16)
    args = ap.parse_args()

    if not os.path.isdir(args.snap_dir):
        print(f"[snap-to-png] snap dir not found: {args.snap_dir}", file=sys.stderr)
        return 1

    font = ImageFont.truetype(args.font, args.font_size)
    # Monospace cell metrics from the font itself; integer so positions are exact.
    cell_w = int(round(font.getlength("M")))
    ascent, descent = font.getmetrics()
    cell_h = ascent + descent

    prune = {"target", ".git", ".cargo", "node_modules", ".derived-data"}
    count = 0
    for root, dirs, files in os.walk(args.snap_dir):
        dirs[:] = [d for d in dirs if d not in prune]
        for name in sorted(files):
            if not name.endswith(".snap"):
                continue
            src = os.path.join(root, name)
            with open(src, encoding="utf-8") as fh:
                body = snapshot_body(fh.read())
            img = render(body, font, cell_w, cell_h)
            rel = os.path.relpath(src, args.snap_dir)
            dst = os.path.join(args.out_dir, rel[: -len(".snap")] + ".png")
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            img.save(dst, "PNG", optimize=False)
            count += 1

    print(f"[snap-to-png] rendered {count} snapshot(s) to {args.out_dir}")
    return 0 if count else 1


if __name__ == "__main__":
    sys.exit(main())
