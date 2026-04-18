# High-Contrast Theme

> **Purpose:** Accessibility-first theme for users with low vision, color-blindness, or who enable macOS *Increase contrast*. Not a cosmetic variant — it is a full remap of the palette plus shape/size reinforcements so every UI state is distinguishable without relying on hue.
>
> **Targets:** WCAG 2.1 **AAA** — 7:1 normal text, 4.5:1 large text / UI components.
>
> **Related:**
> - `specs/design/theme-architecture.md` — how themes are loaded/switched
> - `specs/design/accessibility.md` — principles every theme must satisfy
> - `specs/design/styleguide.md` — tokens + components (Default theme values)

---

## 1. Design intent

| Intent | How |
|---|---|
| Maximum luminance contrast | Pure black `#000000` surfaces + pure white `#FFFFFF` text (21:1) |
| Sharp edges | 2px solid borders on every card/input/notice — no translucent or gradient edges |
| Shape over hue | Status uses icon shape + text label; fills and strokes reinforce, never replace |
| Highly visible focus | 3px solid yellow `#FFDD00` focus ring + 2px offset on every focusable element |
| Color-blind safe palette | Green replaced by high-luminance lime; orange replaced by yellow (orange/brown confuses protanopia on black); red stays red but always paired with a cross icon + "Error" label |
| Reduced complexity | No subtle tints, no translucent overlays, no drop shadows as signal |

**Philosophy:** if you can tell the state of the UI with a monochrome screenshot, the design has passed.

---

## 2. Full palette

All hex values. HSL is not the source of truth in this theme — pure colors drive contrast math.

### Backgrounds

| Token | High-Contrast value | Contrast vs. `--text-primary` |
|---|---|---|
| `--bg-base` | `#000000` | 21:1 |
| `--bg-surface` | `#000000` | 21:1 |
| `--bg-elevated` | `#000000` (+2px white border for separation) | 21:1 |
| `--bg-hover` | `#FFFFFF` (inverse — on hover, row flips to light) | 21:1 inverted |
| `--bg-active` | `#FFDD00` (selected row — yellow) | `--text-inverse` |

**Rule:** Surfaces don't use layered translucent whites in high-contrast — elevation is communicated by borders, not bg lightness.

### Foreground

| Token | Value | Contrast vs. black |
|---|---|---|
| `--text-primary` | `#FFFFFF` | 21:1 |
| `--text-secondary` | `#E0E0E0` | 16.9:1 |
| `--text-muted` | `#B0B0B0` (AAA floor — still ≥ 7:1) | 9.1:1 |
| `--text-inverse` | `#000000` | for use on light/yellow surfaces |

### Accent

The default theme's warm orange (`#FF6B35`) does **not** meet AAA against black (≈4.7:1). In high-contrast we switch to yellow, which is the most luminous hue, universally distinguishable, and common in high-contrast OS themes.

| Token | Value | Contrast vs. black | Use |
|---|---|---|---|
| `--accent-primary` | `#FFDD00` | 18:1 | CTA fills, focused borders, icon accents |
| `--accent-hover` | `#FFEE33` | ≈ 19:1 | CTA hover |
| `--accent-muted` | `#FFDD00` @ 0.15 (visual only; avoid for text) | — | Subtle selected-row fills |

### Status (shape always paired)

| State | Color | Icon | Mandatory text |
|---|---|---|---|
| Success | `#00FF66` (lime on black: ≈17:1) | `CheckCircle2` (filled) | "OK" / "Passed" / "Available" |
| Warning | `#FFDD00` (yellow on black: 18:1) | `TriangleAlert` | "Warning" / "Attention" |
| Error | `#FF3344` (red on black: ≈6:1) | `XCircle` (filled) **+ bold text** | "Error" / "Failed" |
| Info | `#66CCFF` (sky on black: ≈11:1) | `Info` | "Info" / "Note" |

**Red is intentionally duller** so it doesn't vibrate against black. Bold text and the filled X icon carry the error semantics.

### Borders

| Token | Value | Notes |
|---|---|---|
| `--border-subtle` | `rgba(255,255,255,0.5)` | Inner dividers |
| `--border-default` | `#FFFFFF` | Input/card borders |
| `--border-strong` | `#FFFFFF` @ 2px | Emphasis, card edges |
| `--border-focus` | `#FFDD00` @ 3px | Keyboard focus |

---

## 3. Shape / size overrides

The token values `--border-width-default`, `--border-width-focus`, `--radius-lg`, `--font-size-base` from `theme-architecture.md` change in high-contrast:

| Token | Default | High-Contrast | Why |
|---|---|---|---|
| `--border-width-default` | `1px` | `2px` | More visible edges; distinguishes overlapping cards |
| `--border-width-focus` | `2px` | `3px` | Focus must be unmissable |
| `--radius-lg` | `12px` | `8px` | Sharper corners read better at small sizes + on low-vision |
| `--radius-md` | `8px` | `6px` | Same |
| `--font-size-base` | `14px` | `15px` | Slight bump; combines with optional Font Scale setting |

---

## 4. Component-level behavior

### Buttons

```
Primary:
  bg:     #FFDD00     (yellow)
  text:   #000000     (black)
  border: 2px solid #000000 (inside button — keeps shape on non-black surfaces)
  focus:  3px solid #FFDD00 + 2px offset → appears as double-yellow ring

Secondary / ghost:
  bg:     #000000
  text:   #FFFFFF
  border: 2px solid #FFFFFF

Destructive:
  bg:     #FF3344
  text:   #FFFFFF
  Icon:   XCircle or Trash (required)
```

### Inputs

```
Background: #000000
Text:       #FFFFFF (size ≥ 15px)
Border:     2px solid #FFFFFF
Focus:      3px solid #FFDD00 + 2px offset
Placeholder: #B0B0B0 (≥ 7:1; still usable)
Disabled:    #000000 bg, #666666 text (≥ 4.5:1 kept); no hover
```

### Switches / checkboxes

```
Unchecked:
  Switch track:  2px solid #FFFFFF, #000000 fill
  Checkbox:      2px solid #FFFFFF, empty

Checked:
  Switch track:  #FFDD00 fill, #000000 thumb (black circle on yellow)
  Checkbox:      #FFDD00 fill, #000000 check icon (2px stroke)

Focus:
  Always adds 3px #FFDD00 ring with 2px offset
```

**Rule:** the thumb of a yellow switch must be a solid black circle, not a small light dot — visibility over aesthetics.

### Selects / dropdowns

```
Trigger: same as Input
Open menu: #000000 bg, 2px solid #FFFFFF border
Items:
  default: #000000 bg, #FFFFFF text
  hover:   #FFFFFF bg, #000000 text (inverse flip)
  selected: background #FFDD00, text #000000; include ✓ icon
```

### Tabs (Global / Project Overrides etc.)

```
TabsList: #000000 bg, 2px solid #FFFFFF
Trigger default: #FFFFFF text, no bg
Trigger active: #FFDD00 bg, #000000 text, +2px border bottom (visible on any bg)
```

### Cards / notices (InlineNotice)

```
ok:    #000000 bg, 2px solid #00FF66 border, CheckCircle2 icon + "OK"
warn:  #000000 bg, 2px solid #FFDD00 border, TriangleAlert icon + "Warning"
info:  #000000 bg, 2px solid #66CCFF border, Info icon + "Info"
error: #000000 bg, 2px solid #FF3344 border, XCircle icon + "Error", bold title
```

Text inside notices is always white; the border color + icon carry the tone. ❌ Never tint the background color to convey tone in high-contrast.

### Chat bubbles

```
User message:
  bg:     #FFDD00  (yellow — reverses "ownership" signal from the orange in Default)
  text:   #000000
  border: 2px solid #000000 (stays visible on white/light settings surfaces)

Agent message:
  bg:     #000000
  text:   #FFFFFF
  border: 2px solid #FFFFFF
```

User bubble is loud + black text to signal "this is yours" regardless of color vision.

### Nav / sidebar

```
Item default: #FFFFFF text on #000000
Item hover:   #FFFFFF bg, #000000 text (inverse)
Item active:  #FFDD00 bg, #000000 text, font-weight 600
  + left border: 4px solid #000000 (draws over the yellow, adds shape cue)
  + aria-current="page"
```

### Kanban columns

```
Column header always shows:
  - Icon (shape-based: clock / bolt / check / warning)
  - Text label ("Backlog", "Executing", …)
  - Count (e.g. "12")
Column body background: #000000 with 2px solid #FFFFFF border
Column header bg: #FFDD00 for ACTIVE column (current focus) only
```

Column status must be readable from shape alone; color is decoration.

### Dialogs / modals

```
Overlay: rgba(0,0,0,0.9) — nearly opaque; avoid see-through popups
Dialog:  #000000 bg, 2px solid #FFFFFF border, radius 8px
Close X: 2px stroke, #FFFFFF; focus ring as above; 32×32 hit target
```

---

## 5. Motion

High-contrast respects the same `prefers-reduced-motion` rule as the Default theme. In addition:

- Spinner uses a tick/step rotation (4 frames/sec) rather than continuous rotation when reduced-motion is on — makes progress obvious without smooth motion.
- No opacity fades (≤ 50% opacity is never used to convey state — opacity is too subtle).
- Collapsible lanes expand instantly (no height transition).

---

## 6. What to avoid

| ❌ Anti-pattern | Why |
|---|---|
| `color: #ff6b35` inline | Hardcoded default-theme accent — must be `var(--accent-primary)` |
| `background: rgba(255,255,255,0.04)` | Translucent overlays disappear on pure black — use solid bg + border |
| Soft drop shadows for elevation | Invisible on black; always use 2px borders |
| Icon-only status indicators | Even well-chosen icons need a text label in this theme |
| Red/green pairing as sole signal | Protanopia/deuteranopia can't distinguish; use shape + text |
| Placeholder-as-label | Placeholder grays to `#B0B0B0` — a vanished label is unacceptable at low vision |
| Hue-only chart colors | Every chart series needs a pattern (solid / dashed / dotted) or marker shape |

---

## 7. Contrast verification table (spot-checked AAA)

| Pair | Ratio | Meets AAA? |
|---|---|---|
| `#FFFFFF` on `#000000` | 21:1 | ✅ |
| `#E0E0E0` on `#000000` | 16.9:1 | ✅ |
| `#B0B0B0` on `#000000` | 9.1:1 | ✅ |
| `#FFDD00` on `#000000` | 18:1 | ✅ |
| `#000000` on `#FFDD00` | 18:1 (text on button) | ✅ |
| `#00FF66` on `#000000` | 17:1 | ✅ |
| `#FF3344` on `#000000` | ~6:1 | ✅ for large text; meets UI-component 4.5:1 AAA — still paired with XCircle + bold "Error" |
| `#66CCFF` on `#000000` | ~11:1 | ✅ |

Any new color introduced must be verified against `#000000` (and `#FFDD00` if used for button text) and added here.

---

## 8. Testing checklist specific to this theme

- [ ] Every status reads correctly in a grayscale screenshot
- [ ] Keyboard sweep: focus ring visible on every interactive element
- [ ] VoiceOver sweep on Settings + Chat panel — reads roles, states, labels
- [ ] Toggle `prefers-reduced-motion`: spinners/transitions degrade gracefully
- [ ] 200% zoom: no overlap, no truncation without a tooltip
- [ ] Contrast of every new token pair documented in §7
- [ ] No use of `rgba(*)` backgrounds for elevation
- [ ] No orange `#FF6B35` anywhere in the DOM when `[data-theme="high-contrast"]` is set (scan via devtools)

---

## 9. Open decisions

- **Icon weight:** lucide icons default to 2px stroke — possibly bump to 2.5px in high-contrast for chunkier silhouettes. Defer until user testing.
- **User bubble color:** yellow is loud; could move to white + 2px black border instead. Defer to first accessibility review.
- **Syntax highlighting (code blocks):** must define a high-contrast code theme; current default is dark + pastel. TODO.

---

## 10. Reference screenshots

*(To be captured once the toggle ships. Store under `assets/public/themes/high-contrast/*.png` — framed per `.claude/rules/assets.md`.)*
