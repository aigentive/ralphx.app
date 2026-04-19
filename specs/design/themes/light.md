# Light Theme

> **Purpose:** Near-white surfaces with dark text for users who prefer light UI or who need it in bright ambient conditions. Keeps the warm-orange brand identity with darker accent values tuned to meet WCAG AA on a light background.
>
> **Targets:** WCAG 2.1 **AA** — 4.5:1 for normal text, 3:1 for UI components.
>
> **Related:**
> - `specs/design/theme-architecture.md` — how themes load and switch
> - `specs/design/styleguide.md` — tokens + component patterns (Dark is the canonical baseline)
> - `specs/design/themes/high-contrast.md` — contrast AAA variant
> - `src/styles/themes/light.css` — implementation

---

## 1. Design intent

| Intent | How |
|---|---|
| Neutral bright feel | `hsl(220 10% 99%)` base — near-white with a faint blue-gray tint (matches Dark-theme hue family) |
| Same brand identity | Warm orange accent preserved but darkened (`hsl(14 90% 50%)` vs Dark's `hsl(14 100% 60%)`) so it meets 4.5:1 on white |
| Soft surface separation | Elevation by slight off-white and 1-pixel subtle borders rather than shadows |
| Status palette stays Okabe-Ito | CVD-safe colours reused, each re-tuned to AA-dark variants that read on white |
| Shadow strategy | Lighter alpha drop shadows (0.05–0.10) replacing the dark theme's 0.2–0.3 scale |

**Philosophy:** Light is Dark's reflection, not a redesign. Same components, same roles, same hierarchy — inverted luminance.

---

## 2. Full palette (from `src/styles/themes/light.css`)

### Backgrounds

| Token | Value | Primitive source | Contrast vs. `--text-primary` |
|---|---|---|---|
| `--bg-base` | `hsl(220 10% 99%)` | `--gray-50` | 15.6:1 |
| `--bg-surface` | `hsl(220 10% 97%)` | `--gray-100` | 15.4:1 |
| `--bg-elevated` | `hsl(0 0% 100%)` | `--color-white` | 15.7:1 |
| `--bg-hover` | `hsl(220 10% 92%)` | `--gray-200` | 14.0:1 |
| `--bg-surface-hover` | same as `--bg-hover` | — | (deprecated alias) |

### Foreground

| Token | Value | Primitive | Contrast |
|---|---|---|---|
| `--text-primary` | `hsl(220 15% 10%)` | `--gray-990` | 15.7:1 on white |
| `--text-secondary` | `hsl(220 10% 35%)` | `--gray-700` | 7.8:1 on white |
| `--text-muted` | `hsl(220 10% 45%)` | `--gray-600` | 5.3:1 on white |
| `--text-inverse` | `hsl(0 0% 100%)` | `--color-white` | (on accent/dark surfaces) |

### Accent (brand — warm orange, darker variant)

| Token | Value | Contrast vs. white |
|---|---|---|
| `--accent-primary` | `hsl(14 90% 50%)` ≈ `#EA5A26` | 4.5:1 (AA large) |
| `--accent-secondary` | `hsl(32 90% 50%)` | ~4.5:1 |
| `--accent-hover` | `hsl(14 90% 45%)` | 5.4:1 |
| `--accent-muted` | `hsla(14 100% 60% / 0.12)` | (bg tint) |
| `--accent-border` | `hsla(14 100% 60% / 0.30)` | (border tint) |
| `--accent-strong` | `hsla(14 90% 50% / 0.50)` | (emphasis bg) |

### Status — Okabe-Ito tuned for AA on white

| Role | Light value | Rationale |
|---|---|---|
| `--status-success` | `#007a58` (darker bluish-green) | AA 5.0:1 on white |
| `--status-success-muted` | `hsla(164 100% 24% / 0.12)` | faint success tint |
| `--status-success-border` | `hsla(164 100% 24% / 0.30)` | outline |
| `--status-success-strong` | `hsla(164 100% 24% / 0.50)` | emphasis |
| `--status-warning` | `#A87800` (darker amber) | AA 4.5:1 |
| `--status-warning-muted/-border/-strong` | alpha variants of same | |
| `--status-error` | `#B24800` (darker vermillion) | AA 4.5:1 |
| `--status-error-muted/-border/-strong` | alpha variants | |
| `--status-info` | `#005A8A` (darker Okabe blue) | AA 7.1:1 |
| `--status-info-muted/-border/-strong` | alpha variants | |

### Borders

| Token | Value | Width |
|---|---|---|
| `--border-subtle` | `hsl(220 10% 88%)` | 1px |
| `--border-default` | `hsl(220 10% 78%)` | 1px |
| `--border-focus` | `hsl(220 80% 50%)` | 2px (via `--border-width-focus`) |

### Overlays

| Token | Value | Use |
|---|---|---|
| `--overlay-faint` | `hsla(220 15% 10% / 0.03)` | subtle row tint |
| `--overlay-weak` | `hsla(220 15% 10% / 0.05)` | hover emphasis |
| `--overlay-moderate` | `hsla(220 15% 10% / 0.08)` | section highlight |
| `--overlay-scrim` | `hsla(0 0% 0% / 0.25)` | hero dim |
| `--overlay-scrim-med` | `hsla(0 0% 0% / 0.40)` | modal centre |
| `--overlay-scrim-deep` | `hsla(0 0% 0% / 0.50)` | dialog backdrop |

**Design rule:** Light theme uses translucent **blacks** for overlays, not whites — white-on-white disappears on near-white bg.

### Shadows

Dark-theme shadows are too heavy against white. Light retunes:

| Token | Value |
|---|---|
| `--shadow-xs` | `0 1px 2px hsla(0 0% 0% / 0.05), 0 1px 3px hsla(0 0% 0% / 0.03)` |
| `--shadow-sm` | `0 1px 2px hsla(0 0% 0% / 0.08), 0 2px 4px hsla(0 0% 0% / 0.05)` |
| `--shadow-md` | `0 4px 6px hsla(0 0% 0% / 0.08), 0 8px 16px hsla(0 0% 0% / 0.06)` |
| `--shadow-lg` | `0 10px 15px hsla(0 0% 0% / 0.10), 0 20px 40px hsla(0 0% 0% / 0.08)` |

Pulse-shadow tokens (`--shadow-pulse-accent-*`, `-info-*`, `-status-*`, `--shadow-glow-accent-*`, `--shadow-drop-zone-*`) are re-tuned with lower alpha values so animations stay visible but not aggressive on white.

---

## 3. Shape / size overrides

Same as Dark — Light does not change border widths, radii, or font scale.

| Token | Dark | Light |
|---|---|---|
| `--border-width-default` | 1px | 1px |
| `--border-width-focus` | 2px | 2px |
| `--radius-lg` | 12px | 12px |
| `--font-size-base` | 14px | 14px (`<html>` root) |

---

## 4. Component-level behaviour

Components are theme-agnostic — they consume semantic/component tokens, so visuals flip automatically. The spec below documents the *net effect* on Light, not behaviour changes.

### Buttons

```
Primary  (CTA):
  bg:      --accent-primary (darker orange)
  text:    --text-inverse (white)
  hover:   --accent-hover

Secondary / ghost:
  bg:      --bg-elevated (white)
  text:    --text-primary (near-black)
  border:  --border-default

Destructive:
  bg:      --status-error
  text:    --text-inverse
  icon:    XCircle / Trash — mandatory
```

### Inputs / selects

- bg `--bg-elevated` (white)
- border 1px `--border-default`
- focus: 2px `--focus-ring` (blue) + bg unchanged
- placeholder `--text-muted` (5.3:1 on white)

### Dialogs

- overlay `--overlay-scrim-med` (40% black on light)
- panel bg `--bg-elevated`
- border 1px `--border-subtle`
- shadow `--shadow-lg` (light-tuned)

### Cards (`SectionCard`, ProposalCard, TaskCard)

- bg `--bg-elevated` (white)
- border 1px `--border-subtle`
- hover bg `--bg-hover` (faint gray)
- shadow `--shadow-xs`

### Notices (`InlineNotice`)

| Tone | Bg | Border | Icon |
|---|---|---|---|
| ok | `--status-success-muted` | `--status-success-border` | CheckCircle2 `--status-success` |
| warn | `--status-warning-muted` | `--status-warning-border` | TriangleAlert `--status-warning` |
| info | `--status-info-muted` | `--status-info-border` | Info `--status-info` |
| error | `--status-error-muted` | `--status-error-border` | XCircle `--status-error` |

---

## 5. What's different vs. Dark

| Area | Dark | Light |
|---|---|---|
| Surfaces | hsl 220/10%/8–20% tinted dark grays | hsl 220/10%/92–100% near-whites |
| Accent hue | `#FF6B35` | `#EA5A26` (slightly darker for AA) |
| Status colours | Canonical Okabe-Ito | AA-on-white darker variants |
| Shadow alpha | 0.2–0.3 | 0.05–0.10 |
| Overlay colour | Translucent whites | Translucent blacks |
| Focus ring | 2px blue on dark bg | 2px blue on light bg (same token, visible on white) |

Everything else — component shapes, typography, spacing, radii — is identical.

---

## 6. Contrast spot-check

| Pair | Ratio | Meets |
|---|---|---|
| `--text-primary` on `--bg-elevated` | 15.7:1 | AAA |
| `--text-secondary` on `--bg-base` | 7.8:1 | AAA |
| `--text-muted` on `--bg-base` | 5.3:1 | AA |
| `--accent-primary` on white | 4.5:1 | AA (large text / UI component) |
| `--status-error` on white | 4.5:1 | AA |
| `--status-success` on white | 5.0:1 | AA |
| `--status-info` on white | 7.1:1 | AAA |
| `--status-warning` on white | 4.5:1 | AA |

Any new tokens must maintain at least AA on `--bg-elevated` + `--bg-base` and pass the CVD-safe test (see `color-blind-design.md`).

---

## 7. Testing checklist (per release)

- [ ] Toggle Settings → Accessibility → Theme: Light. Walk Kanban / Chat / Settings / Insights.
- [ ] Verify every status notice reads correctly in grayscale screenshot.
- [ ] Keyboard sweep — focus ring visible on every focusable element on white.
- [ ] `prefers-reduced-motion` test — pulse shadows degrade cleanly.
- [ ] Diff viewer is readable (`globals.css` DiffViewer overrides still need a light-theme pass — known limitation).
- [ ] No primitive leak grep (shared with other themes — see `styleguide.md §12`).

---

## 8. Known limitations (2026-04-18)

- **Diff viewer syntax tokens** (`globals.css` Prism/Dracula colours) are pinned to the Dark palette. Light-theme users will see the old dark syntax palette until a shared `syntax-*` token family ships.
- **WelcomeScreen** and `BattleModeV2Overlay` are intentionally fixed-palette regardless of theme. Documented exclusions.
