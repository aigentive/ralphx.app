#!/usr/bin/env python3
"""Shared helpers for the repo asset pipeline."""

from __future__ import annotations

from pathlib import Path
import shutil
import subprocess
import tempfile


REPO_ROOT = Path(__file__).resolve().parents[2]
ASSETS_DIR = REPO_ROOT / "assets"
RAW_ASSETS_DIR = ASSETS_DIR / "raw"
PUBLIC_ASSETS_DIR = ASSETS_DIR / "public"
RAW_VARIATIONS_DIR = RAW_ASSETS_DIR / "variations"
RAW_UNPUBLISHED_DIR = RAW_ASSETS_DIR / "unpublished"

PNG_COMMAND = [
    "-strip",
    "-define",
    "png:compression-filter=5",
    "-define",
    "png:compression-level=9",
    "-define",
    "png:compression-strategy=1",
]

SUPPORTED_EXTENSIONS = {".png", ".jpg", ".jpeg", ".webp"}


def ensure_asset_dirs() -> None:
    RAW_ASSETS_DIR.mkdir(parents=True, exist_ok=True)
    PUBLIC_ASSETS_DIR.mkdir(parents=True, exist_ok=True)
    RAW_VARIATIONS_DIR.mkdir(parents=True, exist_ok=True)
    RAW_UNPUBLISHED_DIR.mkdir(parents=True, exist_ok=True)


def find_magick() -> str:
    magick = shutil.which("magick")
    if not magick:
        raise RuntimeError("ImageMagick `magick` is required for asset publishing")
    return magick


def compress_image(
    input_path: Path,
    output_path: Path,
    *,
    max_width: int = 2560,
    allow_larger_output: bool = False,
) -> dict[str, object]:
    """Compress a publishable image, keeping the original if optimization grows it."""
    ensure_asset_dirs()
    input_path = input_path.resolve()
    output_path = output_path.resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    source_size = input_path.stat().st_size
    with tempfile.NamedTemporaryFile(suffix=input_path.suffix, delete=False) as tmp:
        tmp_path = Path(tmp.name)

    cmd = [
        find_magick(),
        str(input_path),
        "-resize",
        f"{max_width}x>",
        *PNG_COMMAND,
        str(tmp_path),
    ]

    try:
        subprocess.run(cmd, check=True)
        optimized_size = tmp_path.stat().st_size
        keep_optimized = allow_larger_output or optimized_size < source_size

        if keep_optimized:
            if output_path.exists():
                output_path.unlink()
            shutil.move(str(tmp_path), str(output_path))
            final_size = optimized_size
            optimized = True
        else:
            if output_path != input_path:
                shutil.copy2(input_path, output_path)
            final_size = source_size
            optimized = False
        output_path.chmod(0o644)
    finally:
        tmp_path.unlink(missing_ok=True)

    return {
        "input_path": input_path,
        "output_path": output_path,
        "source_size": source_size,
        "final_size": final_size,
        "saved_bytes": source_size - final_size,
        "optimized": optimized,
    }
