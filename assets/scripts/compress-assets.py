#!/usr/bin/env python3
"""Compress publishable image assets for the repo."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

from asset_utils import SUPPORTED_EXTENSIONS, compress_image


def gather_files(inputs: list[str]) -> list[Path]:
    files: list[Path] = []
    for raw_input in inputs:
        path = Path(raw_input)
        if path.is_dir():
            files.extend(
                sorted(
                    candidate
                    for candidate in path.rglob("*")
                    if candidate.is_file() and candidate.suffix.lower() in SUPPORTED_EXTENSIONS
                )
            )
        elif path.is_file():
            files.append(path)
    return files


def build_output_path(source: Path, root: Path | None, output_dir: Path | None) -> Path:
    if output_dir is None:
        return source
    if root is None:
        return output_dir / source.name
    return output_dir / source.relative_to(root)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compress image assets for repo-friendly publishing"
    )
    parser.add_argument("inputs", nargs="+", help="Image files or directories to compress")
    parser.add_argument(
        "--output-dir",
        help="Destination directory for compressed outputs (default: in-place)",
    )
    parser.add_argument(
        "--root",
        help="Root directory used to preserve relative paths when --output-dir is set",
    )
    parser.add_argument(
        "--max-width",
        type=int,
        default=2560,
        help="Maximum output width before downscaling (default: 2560)",
    )
    args = parser.parse_args()

    files = gather_files(args.inputs)
    if not files:
        print("No supported image files found.")
        return 1

    output_dir = Path(args.output_dir).resolve() if args.output_dir else None
    root = Path(args.root).resolve() if args.root else None

    total_before = 0
    total_after = 0
    optimized_count = 0
    for source in files:
        destination = build_output_path(source.resolve(), root, output_dir)
        result = compress_image(source, destination, max_width=args.max_width)
        total_before += int(result["source_size"])
        total_after += int(result["final_size"])
        if result["optimized"]:
            optimized_count += 1

        source_kb = int(result["source_size"]) // 1024
        final_kb = int(result["final_size"]) // 1024
        saved_kb = int(result["saved_bytes"]) // 1024
        print(
            f"{destination}: {source_kb}KB -> {final_kb}KB "
            f"({'optimized' if result['optimized'] else 'copied'}, saved {saved_kb}KB)"
        )

    print(
        f"\nProcessed {len(files)} file(s); optimized {optimized_count}; "
        f"saved {(total_before - total_after) // 1024}KB total."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
