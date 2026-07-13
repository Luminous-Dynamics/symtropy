#!/usr/bin/env python3
"""Validate Old Waterworks screenshot captures.

The gate is intentionally dependency-free: it parses non-interlaced 8-bit RGB
or RGBA PNGs directly, then reports magenta-like fallback pixels. This catches
the common Bevy/wgpu "missing material" failure without requiring Pillow.
"""

from __future__ import annotations

import argparse
import glob
import struct
import sys
import zlib
from pathlib import Path


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def paeth(left: int, up: int, upper_left: int) -> int:
    estimate = left + up - upper_left
    dist_left = abs(estimate - left)
    dist_up = abs(estimate - up)
    dist_upper_left = abs(estimate - upper_left)
    if dist_left <= dist_up and dist_left <= dist_upper_left:
        return left
    if dist_up <= dist_upper_left:
        return up
    return upper_left


def read_png_pixels(path: Path) -> tuple[int, int, list[tuple[int, int, int, int]]]:
    data = path.read_bytes()
    if not data.startswith(PNG_SIGNATURE):
        raise ValueError("not a PNG file")

    offset = len(PNG_SIGNATURE)
    width = height = bit_depth = color_type = interlace = None
    compressed = bytearray()

    while offset < len(data):
        if offset + 8 > len(data):
            raise ValueError("truncated PNG chunk header")
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        chunk_type = data[offset + 4 : offset + 8]
        chunk_start = offset + 8
        chunk_end = chunk_start + length
        if chunk_end + 4 > len(data):
            raise ValueError(f"truncated PNG chunk {chunk_type!r}")
        chunk = data[chunk_start:chunk_end]
        offset = chunk_end + 4

        if chunk_type == b"IHDR":
            width, height, bit_depth, color_type, _compression, _filter, interlace = struct.unpack(
                ">IIBBBBB", chunk
            )
        elif chunk_type == b"IDAT":
            compressed.extend(chunk)
        elif chunk_type == b"IEND":
            break

    if width is None or height is None or bit_depth is None or color_type is None:
        raise ValueError("missing IHDR")
    if bit_depth != 8:
        raise ValueError(f"unsupported PNG bit depth {bit_depth}; expected 8")
    if color_type not in (2, 6):
        raise ValueError(f"unsupported PNG color type {color_type}; expected RGB/RGBA")
    if interlace != 0:
        raise ValueError("interlaced PNGs are not supported")

    channels = 3 if color_type == 2 else 4
    stride = width * channels
    raw = zlib.decompress(bytes(compressed))
    expected = height * (1 + stride)
    if len(raw) != expected:
        raise ValueError(f"unexpected decompressed size {len(raw)}; expected {expected}")

    pixels: list[tuple[int, int, int, int]] = []
    previous = bytearray(stride)
    cursor = 0
    for _row in range(height):
        filter_type = raw[cursor]
        cursor += 1
        scanline = bytearray(raw[cursor : cursor + stride])
        cursor += stride

        for i, value in enumerate(scanline):
            left = scanline[i - channels] if i >= channels else 0
            up = previous[i]
            upper_left = previous[i - channels] if i >= channels else 0
            if filter_type == 0:
                recon = value
            elif filter_type == 1:
                recon = value + left
            elif filter_type == 2:
                recon = value + up
            elif filter_type == 3:
                recon = value + ((left + up) // 2)
            elif filter_type == 4:
                recon = value + paeth(left, up, upper_left)
            else:
                raise ValueError(f"unknown PNG filter type {filter_type}")
            scanline[i] = recon & 0xFF

        for x in range(width):
            base = x * channels
            red = scanline[base]
            green = scanline[base + 1]
            blue = scanline[base + 2]
            alpha = scanline[base + 3] if channels == 4 else 255
            pixels.append((red, green, blue, alpha))
        previous = scanline

    return width, height, pixels


def is_magenta_like(pixel: tuple[int, int, int, int]) -> bool:
    red, green, blue, alpha = pixel
    return alpha > 16 and red >= 180 and blue >= 180 and green <= 100


def is_visible(pixel: tuple[int, int, int, int]) -> bool:
    red, green, blue, alpha = pixel
    return alpha > 16 and max(red, green, blue) >= 24


def palette_bucket(pixel: tuple[int, int, int, int]) -> str | None:
    red, green, blue, alpha = pixel
    if alpha <= 16:
        return None

    # These deliberately match broad material families, not exact RGB values.
    # The goal is to catch semantic regressions without tying the gate to one seed.
    if 24 <= red <= 95 and 24 <= green <= 100 and 24 <= blue <= 105 and max(red, green, blue) - min(red, green, blue) <= 35:
        return "dark_concrete"
    if red <= 35 and 20 <= green <= 70 and 25 <= blue <= 90 and blue >= green >= red:
        return "dirty_water"
    if red >= 80 and 20 <= green <= 95 and blue <= 70 and red >= green * 1.35:
        return "rust_pipe"
    if red >= 145 and 90 <= green <= 185 and blue <= 75 and red >= green:
        return "warning_amber"
    if red <= 85 and green >= 95 and blue >= 110 and abs(green - blue) <= 95:
        return "field_deck_cyan"
    if green >= 95 and 55 <= red <= 170 and blue <= 140 and green >= blue * 1.15:
        return "evidence_green"
    return None


def analyze(
    path: Path,
    max_magenta_fraction: float,
    min_visible_fraction: float,
    min_palette_fraction: float,
    require_semantic_palette: bool,
) -> bool:
    width, height, pixels = read_png_pixels(path)
    total = len(pixels)
    magenta = sum(1 for pixel in pixels if is_magenta_like(pixel))
    visible = sum(1 for pixel in pixels if is_visible(pixel))
    palette_counts = {
        "dark_concrete": 0,
        "dirty_water": 0,
        "rust_pipe": 0,
        "warning_amber": 0,
        "field_deck_cyan": 0,
        "evidence_green": 0,
    }
    for pixel in pixels:
        bucket = palette_bucket(pixel)
        if bucket is not None:
            palette_counts[bucket] += 1

    magenta_fraction = magenta / total if total else 1.0
    visible_fraction = visible / total if total else 0.0
    palette_fractions = {
        name: count / total if total else 0.0 for name, count in palette_counts.items()
    }

    print(
        f"{path}: {width}x{height}, "
        f"magenta={magenta_fraction:.6f}, visible={visible_fraction:.6f}, "
        + ", ".join(f"{name}={value:.6f}" for name, value in palette_fractions.items())
    )

    ok = True
    if magenta_fraction > max_magenta_fraction:
        print(
            f"ERROR: {path} exceeds magenta fallback threshold "
            f"{max_magenta_fraction:.6f}",
            file=sys.stderr,
        )
        ok = False
    if visible_fraction < min_visible_fraction:
        print(
            f"ERROR: {path} appears too dark/empty; visible fraction below "
            f"{min_visible_fraction:.6f}",
            file=sys.stderr,
        )
        ok = False
    if require_semantic_palette:
        missing = [
            name
            for name, fraction in palette_fractions.items()
            if fraction < min_palette_fraction
        ]
        if missing:
            print(
                f"ERROR: {path} is missing Old Waterworks palette coverage "
                f"above {min_palette_fraction:.6f}: {', '.join(missing)}",
                file=sys.stderr,
            )
            ok = False
    return ok


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="+", help="PNG files or glob patterns")
    parser.add_argument("--max-magenta-fraction", type=float, default=0.001)
    parser.add_argument("--min-visible-fraction", type=float, default=0.05)
    parser.add_argument("--min-palette-fraction", type=float, default=0.00002)
    parser.add_argument(
        "--skip-semantic-palette",
        action="store_true",
        help="Only check magenta/visibility, not Old Waterworks material families.",
    )
    args = parser.parse_args()

    paths: list[Path] = []
    for pattern in args.paths:
        matches = [Path(match) for match in glob.glob(pattern)]
        if matches:
            paths.extend(matches)
        else:
            paths.append(Path(pattern))

    if not paths:
        print("ERROR: no PNG paths provided", file=sys.stderr)
        return 2

    ok = True
    for path in sorted(paths):
        if not path.exists():
            print(f"ERROR: missing capture {path}", file=sys.stderr)
            ok = False
            continue
        try:
            ok = (
                analyze(
                    path,
                    args.max_magenta_fraction,
                    args.min_visible_fraction,
                    args.min_palette_fraction,
                    not args.skip_semantic_palette,
                )
                and ok
            )
        except Exception as exc:  # noqa: BLE001 - CLI should report corrupt captures.
            print(f"ERROR: failed to analyze {path}: {exc}", file=sys.stderr)
            ok = False

    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
