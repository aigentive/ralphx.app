#!/usr/bin/env python3
"""
Pipeline diagram generator — dark-theme vertical flow for README "How It Works" section.

Generates assets/pipeline-diagram.png matching the app's design system:
  - Subdued dark mesh gradient background
  - Flat surface cards with subtle border and drop shadow
  - Accent orange (#ff6b35) arrows with glow + arrowheads
  - SF Pro typography via SFNS.ttf variable font

Usage:
  python3 scripts/generate-pipeline-diagram.py
"""

from PIL import Image, ImageDraw, ImageFilter, ImageFont
import os


# ── Colors ────────────────────────────────────────────────────────────────────
BG_SURFACE = (27, 29, 33)
TEXT_PRIMARY = (226, 228, 232)
TEXT_SECONDARY = (142, 149, 163)
BORDER_SUBTLE = (41, 44, 50)
ACCENT = (255, 107, 53)

BG_GRADIENT = (
    (16, 18, 26),
    (22, 16, 30),
    (18, 24, 32),
    (14, 20, 28),
)

# ── Layout ────────────────────────────────────────────────────────────────────
CANVAS_W = 2800
CARD_W = 1800
CARD_X = (CANVAS_W - CARD_W) // 2
CARD_PAD_X = 48
CARD_PAD_Y = 48
CARD_RADIUS = 20
BADGE_SIZE = 56
BADGE_GAP = 20
ARROW_GAP = 120
ARROW_MARGIN = 12
PAD_TOP = 150
PAD_BOTTOM = 160
TRIGGER_ARROW_GAP = 90


# ── Pipeline stages ──────────────────────────────────────────────────────────
TRIGGER_TEXT = "You describe what you want"

STAGES = [
    ("Ideation Studio", [
        "Natural language \u2192 task proposals with dependencies",
        "Solo, Research Team, or Debate Team mode",
    ]),
    ("Kanban Board", [
        "Drag task to Planned \u2192 execution begins",
        "Up to 10 tasks running concurrently",
    ]),
    ("Worker Agent", [
        "Writes code in isolated git worktree",
        "Scoped tools: file read/write, shell",
        "Cannot approve its own code",
    ]),
    ("Reviewer Agent", [
        "Reviews diffs, files structured issues",
        "File read + validation commands + verdict",
        "Max 3 auto-fix cycles before escalation",
    ]),
    ("Merger Agent", [
        "Merges to main, runs full validation suite",
        "Type check \u2192 lint \u2192 clippy \u2192 tests",
        "Reports conflicts, never forces",
    ]),
    ("Supervisor Watchdog", [
        "Detects loops, stalls, resource waste",
        "Stops stuck agents via state machine",
        "Escalates to you when it matters",
    ]),
]


# ── Gradient (from frame-screenshots.py) ──────────────────────────────────────

def lerp_color(c1, c2, t):
    """Linear interpolate between two RGB colors."""
    return tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3))


def generate_mesh_gradient(width, height, colors):
    """Generate a mesh gradient background (4-corner bilinear interpolation)."""
    img = Image.new('RGB', (width, height))
    pixels = img.load()
    tl, tr, bl, br = colors
    for y in range(height):
        ty = y / max(height - 1, 1)
        for x in range(width):
            tx = x / max(width - 1, 1)
            top = lerp_color(tl, tr, tx)
            bottom = lerp_color(bl, br, tx)
            pixels[x, y] = lerp_color(top, bottom, ty)
    return img


# ── Font loading ──────────────────────────────────────────────────────────────

def load_fonts():
    """Load SF Pro variable font at required weights."""
    path = '/System/Library/Fonts/SFNS.ttf'

    title = ImageFont.truetype(path, 44)
    title.set_variation_by_name('Semibold')

    desc = ImageFont.truetype(path, 30)
    desc.set_variation_by_name('Regular')

    trigger = ImageFont.truetype(path, 48)
    trigger.set_variation_by_name('Semibold')

    badge = ImageFont.truetype(path, 28)
    badge.set_variation_by_name('Bold')

    return title, desc, trigger, badge


# ── Measurement ───────────────────────────────────────────────────────────────

def text_h(draw, text, font):
    bbox = draw.textbbox((0, 0), text, font=font)
    return bbox[3] - bbox[1]


def text_w(draw, text, font):
    bbox = draw.textbbox((0, 0), text, font=font)
    return bbox[2] - bbox[0]


TITLE_DESC_GAP = 20
LINE_GAP = 14


def card_height(draw, name, lines, title_font, desc_font):
    th = text_h(draw, name, title_font)
    dh = text_h(draw, "Xg", desc_font)
    content = th + TITLE_DESC_GAP + dh * len(lines) + LINE_GAP * max(0, len(lines) - 1)
    return content + CARD_PAD_Y * 2


# ── Drawing ───────────────────────────────────────────────────────────────────

def draw_card_shadow(canvas, x, y, w, h):
    blur_r = 30
    expand = blur_r * 3
    shadow = Image.new('RGBA', (w + expand * 2, h + expand * 2), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    sd.rounded_rectangle(
        [(expand, expand + 12), (expand + w, expand + h + 12)],
        radius=CARD_RADIUS,
        fill=(0, 0, 0, 50),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(radius=blur_r))
    canvas.paste(shadow, (x - expand, y - expand), shadow)


def draw_card(canvas, draw, y, name, lines, step, fonts):
    title_font, desc_font, _, badge_font = fonts
    h = card_height(draw, name, lines, title_font, desc_font)

    # Shadow
    draw_card_shadow(canvas, CARD_X, y, CARD_W, h)

    # Background + border
    draw.rounded_rectangle(
        [(CARD_X, y), (CARD_X + CARD_W, y + h)],
        radius=CARD_RADIUS, fill=BG_SURFACE, outline=BORDER_SUBTLE, width=2,
    )

    # Title — draw first to measure actual rendered position
    bx = CARD_X + CARD_PAD_X
    tx = bx + BADGE_SIZE + BADGE_GAP
    ty = y + CARD_PAD_Y
    draw.text((tx, ty), name, font=title_font, fill=TEXT_PRIMARY)

    # Find the title's true visual vertical center
    title_bbox = draw.textbbox((tx, ty), name, font=title_font)
    title_cy = (title_bbox[1] + title_bbox[3]) // 2

    # Badge — centered on the same horizontal line as title
    by = title_cy - BADGE_SIZE // 2
    draw.ellipse([(bx, by), (bx + BADGE_SIZE, by + BADGE_SIZE)], fill=ACCENT)

    num = str(step)
    draw.text(
        (bx + BADGE_SIZE // 2, title_cy),
        num, font=badge_font, fill=(255, 255, 255), anchor="mm",
    )

    # Description lines
    title_height = text_h(draw, name, title_font)
    dy = ty + title_height + TITLE_DESC_GAP
    dh = text_h(draw, "Xg", desc_font)
    for line in lines:
        draw.text((tx, dy), line, font=desc_font, fill=TEXT_SECONDARY)
        dy += dh + LINE_GAP

    return h


def draw_arrow(canvas, draw, y_start, y_end):
    cx = CANVAS_W // 2

    # Glow — narrow strip composited via alpha
    strip_h = y_end - y_start + 40
    margin = 20
    strip = Image.new('RGBA', (60, strip_h + margin * 2), (0, 0, 0, 0))
    sd = ImageDraw.Draw(strip)
    sd.line(
        [(30, margin), (30, strip_h + margin)],
        fill=(*ACCENT, 38), width=8,
    )
    strip = strip.filter(ImageFilter.GaussianBlur(radius=4))
    canvas.paste(strip, (cx - 30, y_start - margin), strip)

    # Crisp line
    draw.line([(cx, y_start), (cx, y_end)], fill=ACCENT, width=3)

    # Arrowhead
    s = 14
    draw.polygon(
        [(cx - s, y_end - s), (cx + s, y_end - s), (cx, y_end + 2)],
        fill=ACCENT,
    )


def draw_trigger_glow(canvas, draw, y, text, font):
    """Draw a warm glow behind the trigger text."""
    tw = text_w(draw, text, font)
    th = text_h(draw, text, font)
    margin = 40
    glow = Image.new('RGBA', (tw + margin * 2, th + margin * 2), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.text((margin, margin), text, font=font, fill=(*ACCENT, 30))
    glow = glow.filter(ImageFilter.GaussianBlur(radius=20))
    glow_x = (CANVAS_W - tw) // 2 - margin
    canvas.paste(glow, (glow_x, y - margin), glow)


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    fonts = load_fonts()
    title_font, desc_font, trigger_font, badge_font = fonts

    # Scratch context for pre-measurement
    scratch = ImageDraw.Draw(Image.new('RGB', (1, 1)))

    trigger_h = text_h(scratch, TRIGGER_TEXT, trigger_font)

    heights = []
    for name, lines in STAGES:
        heights.append(card_height(scratch, name, lines, title_font, desc_font))

    canvas_h = (
        PAD_TOP
        + trigger_h
        + TRIGGER_ARROW_GAP
        + sum(heights)
        + ARROW_GAP * (len(STAGES) - 1)
        + PAD_BOTTOM
    )

    print(f"Canvas: {CANVAS_W}x{canvas_h}")

    # Background
    bg = generate_mesh_gradient(CANVAS_W, canvas_h, BG_GRADIENT)
    canvas = bg.convert('RGBA')
    draw = ImageDraw.Draw(canvas)

    # Trigger text
    y = PAD_TOP
    tw = text_w(draw, TRIGGER_TEXT, trigger_font)
    tx = (CANVAS_W - tw) // 2

    draw_trigger_glow(canvas, draw, y, TRIGGER_TEXT, trigger_font)
    draw.text((tx, y), TRIGGER_TEXT, font=trigger_font, fill=TEXT_PRIMARY)

    y += trigger_h + 20

    # Arrow from trigger to first card
    arrow_end = y + TRIGGER_ARROW_GAP - 20
    draw_arrow(canvas, draw, y, arrow_end)
    y = arrow_end + ARROW_MARGIN + 4

    # Cards with inter-card arrows
    for i, ((name, lines), ch) in enumerate(zip(STAGES, heights)):
        draw_card(canvas, draw, y, name, lines, i + 1, fonts)
        y += ch

        if i < len(STAGES) - 1:
            a_start = y + ARROW_MARGIN
            a_end = y + ARROW_GAP - ARROW_MARGIN
            draw_arrow(canvas, draw, a_start, a_end)
            y += ARROW_GAP

    # Save
    assets_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'assets')
    assets_dir = os.path.abspath(assets_dir)
    out = os.path.join(assets_dir, 'pipeline-diagram.png')

    canvas.convert('RGB').save(out, 'PNG', optimize=True)

    size_kb = os.path.getsize(out) // 1024
    print(f"Generated: {out}")
    print(f"File size: {size_kb}KB")


if __name__ == '__main__':
    main()
