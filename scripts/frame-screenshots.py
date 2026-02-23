#!/usr/bin/env python3
"""
Dribbble-style screenshot framing — vibrant mesh gradient background + drop shadow.

Creates beautifully framed versions of screenshots with:
- Rich, vibrant mesh gradient backgrounds (high contrast against dark app)
- Soft drop shadow beneath the screenshot
- Rounded corners on the screenshot
- Auto-crop blank system bar from top
- Scaled down and centered with generous padding

Usage:
  python3 scripts/frame-screenshots.py                    # Frame all README screenshots
  python3 scripts/frame-screenshots.py --single welcome   # Frame just one (by palette key)
  python3 scripts/frame-screenshots.py --variations welcome  # Generate multiple gradient options
"""

from PIL import Image, ImageDraw, ImageFilter
import os
import sys
import argparse


def lerp_color(c1, c2, t):
    """Linear interpolate between two RGB colors."""
    return tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3))


def generate_mesh_gradient(width, height, colors):
    """
    Generate a mesh gradient background using 4 corner colors
    with bilinear interpolation.
    colors = (top_left, top_right, bottom_left, bottom_right)
    """
    img = Image.new('RGB', (width, height))
    pixels = img.load()

    tl, tr, bl, br = colors

    for y in range(height):
        ty = y / max(height - 1, 1)
        for x in range(width):
            tx = x / max(width - 1, 1)
            top = lerp_color(tl, tr, tx)
            bottom = lerp_color(bl, br, tx)
            pixel = lerp_color(top, bottom, ty)
            pixels[x, y] = pixel

    return img


def add_rounded_corners(img, radius):
    """Add rounded corners to an RGBA image."""
    mask = Image.new('L', img.size, 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle([(0, 0), (img.size[0] - 1, img.size[1] - 1)], radius=radius, fill=255)
    result = img.copy()
    result.putalpha(mask)
    return result


def create_drop_shadow(size, offset=(0, 16), blur_radius=50, shadow_color=(0, 0, 0, 70)):
    """Create a soft drop shadow image."""
    expand = blur_radius * 2
    shadow_size = (size[0] + expand * 2, size[1] + expand * 2)
    shadow = Image.new('RGBA', shadow_size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(shadow)

    left = expand + offset[0]
    top = expand + offset[1]
    draw.rounded_rectangle(
        [(left, top), (left + size[0], top + size[1])],
        radius=16,
        fill=shadow_color
    )

    shadow = shadow.filter(ImageFilter.GaussianBlur(radius=blur_radius))
    return shadow, expand


def auto_crop_top(img, threshold=10, min_crop=0):
    """
    Auto-detect and crop blank/dark strip at top of screenshot
    (macOS system bar area above the app window).
    Scans rows from top, crops until finding a row with significant variance.
    """
    pixels = img.load()
    width = img.width

    for y in range(min_crop, min(img.height // 4, 100)):
        # Sample pixels across the row
        row_colors = [pixels[x, y] for x in range(0, width, max(1, width // 20))]
        # Check if row has meaningful color variance
        r_vals = [c[0] for c in row_colors]
        g_vals = [c[1] for c in row_colors]
        b_vals = [c[2] for c in row_colors]
        variance = max(max(r_vals) - min(r_vals), max(g_vals) - min(g_vals), max(b_vals) - min(b_vals))
        if variance > threshold:
            crop_y = max(0, y - 2)  # Keep a tiny margin
            if crop_y > 0:
                return img.crop((0, crop_y, img.width, img.height))
            return img

    return img


def frame_screenshot(input_path, output_path, gradient_colors, scale=0.85, padding_ratio=0.08, crop_top=True):
    """
    Create a Dribbble-style framed screenshot.

    Args:
        input_path: Path to original screenshot
        output_path: Path for framed output
        gradient_colors: Tuple of 4 RGB colors (TL, TR, BL, BR) for mesh gradient
        scale: How much to scale down the screenshot (0.85 = 85% of canvas width)
        padding_ratio: Padding as ratio of canvas height
        crop_top: Auto-crop blank system bar from top
    """
    img = Image.open(input_path).convert("RGBA")

    # Auto-crop blank top strip
    if crop_top:
        img = auto_crop_top(img)

    # Calculate canvas size
    canvas_width = int(img.width / scale)
    scaled_width = int(canvas_width * scale)
    scaled_height = int(img.height * (scaled_width / img.width))
    padding_y = int(canvas_width * padding_ratio)
    canvas_height = scaled_height + (padding_y * 2)

    # Generate mesh gradient background
    gradient = generate_mesh_gradient(canvas_width, canvas_height, gradient_colors)
    canvas = gradient.convert('RGBA')

    # Scale screenshot
    img_scaled = img.resize((scaled_width, scaled_height), Image.Resampling.LANCZOS)

    # Rounded corners
    corner_radius = max(12, int(scaled_width * 0.006))
    img_rounded = add_rounded_corners(img_scaled, corner_radius)

    # Drop shadow
    shadow, shadow_expand = create_drop_shadow(
        (scaled_width, scaled_height),
        offset=(0, 18),
        blur_radius=55,
        shadow_color=(0, 0, 0, 80)
    )

    # Center positions
    x_offset = (canvas_width - scaled_width) // 2
    y_offset = (canvas_height - scaled_height) // 2

    # Composite shadow
    shadow_x = x_offset - shadow_expand
    shadow_y = y_offset - shadow_expand
    canvas.paste(shadow, (shadow_x, shadow_y), shadow)

    # Composite screenshot
    canvas.paste(img_rounded, (x_offset, y_offset), img_rounded)

    # Save
    canvas.save(output_path, 'PNG', optimize=True)
    size_kb = os.path.getsize(output_path) // 1024
    print(f"  {os.path.basename(output_path)} ({canvas_width}x{canvas_height}, {size_kb}KB)")


# ============================================================================
# Gradient Palettes — vibrant, rich, high-contrast against dark app UI
# ============================================================================

PALETTES = {
    # Warm sunset vibes — brand-aligned orange → coral → magenta
    'welcome': (
        (180, 80, 50),     # Warm orange
        (200, 60, 120),    # Coral-magenta
        (120, 50, 160),    # Purple
        (60, 80, 180),     # Blue-indigo
    ),
    # Deep ocean → electric blue
    'graph': (
        (30, 100, 200),    # Electric blue
        (80, 50, 180),     # Indigo
        (20, 140, 180),    # Teal
        (100, 60, 200),    # Violet
    ),
    # Warm rose → coral (avoids purple dominance)
    'ideation': (
        (200, 70, 80),     # Rose-coral
        (180, 50, 110),    # Dusty rose
        (160, 80, 60),     # Terracotta
        (140, 60, 130),    # Muted plum accent
    ),
    # Teal → emerald gradient
    'merge': (
        (20, 140, 160),    # Teal
        (60, 100, 180),    # Steel blue
        (30, 160, 120),    # Emerald
        (80, 120, 200),    # Periwinkle
    ),
    # Warm burgundy → steel (avoids purple dominance)
    'ai-review': (
        (160, 50, 70),     # Burgundy
        (120, 60, 100),    # Dusty mauve
        (180, 70, 60),     # Warm brick
        (100, 70, 130),    # Muted slate-purple
    ),
    # Warm amber → orange
    'merge-conflicts': (
        (200, 100, 40),    # Amber
        (180, 60, 80),     # Rust-rose
        (160, 120, 30),    # Gold
        (200, 80, 60),     # Burnt orange
    ),
    # Cool green → teal
    'approved': (
        (30, 160, 120),    # Emerald
        (40, 120, 180),    # Teal-blue
        (50, 180, 100),    # Green
        (60, 140, 160),    # Teal
    ),
    # Deep teal → cyan (merged/completed state)
    'merged': (
        (20, 160, 150),    # Deep teal
        (50, 130, 200),    # Cerulean
        (30, 180, 130),    # Aquamarine
        (70, 150, 190),    # Steel teal
    ),
}

# Variation palettes for testing (--variations mode)
VARIATION_PALETTES = {
    'A-sunset': (
        (180, 80, 50),     # Warm orange
        (200, 60, 120),    # Coral-magenta
        (120, 50, 160),    # Purple
        (60, 80, 180),     # Blue-indigo
    ),
    'B-ocean': (
        (30, 100, 200),    # Electric blue
        (80, 50, 180),     # Indigo
        (20, 140, 180),    # Teal
        (100, 60, 200),    # Violet
    ),
    'C-aurora': (
        (40, 180, 140),    # Mint
        (80, 60, 200),     # Violet
        (30, 140, 200),    # Sky blue
        (160, 50, 160),    # Magenta
    ),
    'D-ember': (
        (220, 90, 40),     # Bright orange
        (200, 50, 80),     # Crimson
        (180, 120, 30),    # Gold
        (160, 40, 120),    # Magenta
    ),
    'E-royal': (
        (100, 40, 200),    # Royal purple
        (180, 50, 140),    # Hot pink
        (40, 60, 180),     # Deep blue
        (120, 40, 180),    # Violet
    ),
    'F-nebula': (
        (60, 20, 140),     # Deep violet
        (180, 40, 100),    # Fuchsia
        (20, 80, 160),     # Royal blue
        (140, 30, 160),    # Purple
    ),
}

# Screenshots config: (source_filename, output_prefix, palette_key)
SCREENSHOTS = [
    ('welcome-2026-02-22.png', 'framed-welcome-2026-02-22.png', 'welcome'),
    ('graph-2026-02-22.png', 'framed-graph-2026-02-22.png', 'graph'),
    ('ideation-2026-02-21.png', 'framed-ideation-2026-02-21.png', 'ideation'),
    ('merge-2026-02-21.png', 'framed-merge-2026-02-21.png', 'merge'),
    ('ai-review-2026-02-22.png', 'framed-ai-review-2026-02-22.png', 'ai-review'),
    ('merge-conflicts-2026-02-22.png', 'framed-merge-conflicts-2026-02-22.png', 'merge-conflicts'),
    ('approved-2026-02-22.png', 'framed-approved-2026-02-22.png', 'approved'),
    ('merged-2026-02-23.png', 'framed-merged-2026-02-23.png', 'merged'),
]


def main():
    parser = argparse.ArgumentParser(description='Frame screenshots with Dribbble-style gradients')
    parser.add_argument('--single', help='Frame just one screenshot by palette key (e.g., welcome)')
    parser.add_argument('--variations', help='Generate gradient variations for one screenshot')
    args = parser.parse_args()

    assets_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'assets')
    assets_dir = os.path.abspath(assets_dir)

    if args.variations:
        # Generate multiple gradient options for comparison
        key = args.variations
        src_file = next((s[0] for s in SCREENSHOTS if s[2] == key), None)
        if not src_file:
            print(f"Unknown key: {key}. Available: {[s[2] for s in SCREENSHOTS]}")
            return

        src_path = os.path.join(assets_dir, src_file)
        print(f"Generating {len(VARIATION_PALETTES)} gradient variations for {src_file}...\n")

        for var_name, palette in VARIATION_PALETTES.items():
            dst_name = f"var-{var_name}-{src_file}"
            dst_path = os.path.join(assets_dir, dst_name)
            frame_screenshot(src_path, dst_path, palette)

        print(f"\nDone! Check assets/var-*-{src_file} files.")
        return

    if args.single:
        matches = [(s, d, k) for s, d, k in SCREENSHOTS if k == args.single]
        if not matches:
            print(f"Unknown key: {args.single}. Available: {[s[2] for s in SCREENSHOTS]}")
            return
        targets = matches
    else:
        targets = SCREENSHOTS

    print("Framing screenshots with vibrant gradient backgrounds...\n")

    for src_name, dst_name, palette_key in targets:
        src_path = os.path.join(assets_dir, src_name)
        dst_path = os.path.join(assets_dir, dst_name)

        if not os.path.exists(src_path):
            print(f"  SKIP {src_name} (not found)")
            continue

        frame_screenshot(src_path, dst_path, PALETTES[palette_key])

    print("\nDone!")


if __name__ == '__main__':
    main()
