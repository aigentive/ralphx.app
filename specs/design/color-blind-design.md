# Color-Blind Design Spec

> **Purpose:** Mandatory rules so every screen, state, and signal in RalphX is perceivable by users with color vision deficiency (CVD). Complements `specs/design/accessibility.md` with CVD-specific palettes and patterns.
>
> **Scope:** Applies in **every theme**, not just High-Contrast. The Default theme must never rely on color alone.

---

## 1. Why this matters

| Population | Rate |
|---|---|
| Men with CVD (any form) | ~8% (≈1 in 12) |
| Women with CVD | ~0.5% (≈1 in 200) |
| Global CVD population | ~300 million |

Most common forms are red/green (protanopia + deuteranopia together ≈ 99% of cases). Blue/yellow (tritanopia) is rare but still non-zero.

---

## 2. Types of CVD + what each user sees

| Condition | Affected cone | Most-confused pairs | Approx prevalence (males) |
|---|---|---|---|
| **Deuteranopia / deuteranomaly** (green-weak/blind) | M-cone | red↔green, green↔yellow, blue↔purple | ~6% |
| **Protanopia / protanomaly** (red-weak/blind) | L-cone | red↔green (reds appear brown/dark), red↔black | ~1% |
| **Tritanopia / tritanomaly** (blue-weak/blind) | S-cone | blue↔green, purple↔red, yellow↔pink | <0.01% |
| **Monochromacy / achromatopsia** (total) | — | all hues | ~0.003% |

**Implication:** any red-vs-green signal fails for ~7% of male users. Any blue-vs-yellow signal is safer but still needs a non-color backup.

---

## 3. WCAG 2.1 requirements

| Success Criterion | Level | Requirement |
|---|---|---|
| **1.4.1 Use of Color** | A | Color is never the sole visual means of conveying information, indicating action, prompting response, or distinguishing elements |
| **1.4.3 Contrast (Minimum)** | AA | 4.5:1 normal text / 3:1 large text |
| **1.4.6 Contrast (Enhanced)** | AAA | 7:1 / 4.5:1 |
| **1.4.11 Non-text Contrast** | AA | 3:1 for UI component borders, focus rings, icons |

Our Default theme targets AA; the High-Contrast theme targets AAA.

---

## 4. Core rules (NON-NEGOTIABLE)

Every status, signal, or interactive state must carry **at least two** of these cues, where one of them must NOT be color:

| # | Rule | Applies to |
|---|---|---|
| 1 | **Color + icon + text** trio for every status | Success / Warning / Error / Info cards |
| 2 | **Shape ≠ color** for distinction | Graph nodes, chart markers, status dots |
| 3 | **Pattern or texture** on large fills | Charts with > 3 series, density maps |
| 4 | **Underline on every link** | Links must be underlined in running text (not color-only) |
| 5 | **Asterisk `*` or "(required)"** for required fields | Never red-only borders |
| 6 | **Prefix diffs** with `+` / `−` | Code diffs, line counters |
| 7 | **Label on every tag / pill** | Tag color is decoration; text carries meaning |
| 8 | **Position matters** | Success toasts bottom-right + icon; errors top-center + icon |

Failing rule #1 is the single most common CVD bug. Grep for any `text-[var(--status-*)]` or bare colored dot in the codebase and verify it is paired with an icon + text label.

---

## 5. CVD-safe palette reference (Okabe-Ito 2008)

Proposed by Masataka Okabe & Kei Ito and endorsed by *Nature Methods* — tested to remain distinguishable under all three dichromatic conditions AND in grayscale.

| Role hint | Hex | RGB | Use |
|---|---|---|---|
| Black | `#000000` | 0,0,0 | Baseline, text |
| Orange | `#E69F00` | 230,159,0 | Primary accent (safer than red) |
| Sky Blue | `#56B4E9` | 86,180,233 | Info, selection |
| Bluish Green | `#009E73` | 0,158,115 | Success (distinct from yellow under deuteranopia) |
| Yellow | `#F0E442` | 240,228,66 | Warning highlight |
| Blue | `#0072B2` | 0,114,178 | Primary link, "active" |
| Vermillion | `#D55E00` | 213,94,0 | Error / destructive (orange-red; reads differently than pure red under protanopia) |
| Reddish Purple | `#CC79A7` | 204,121,167 | Secondary accent |

### Why we don't adopt it wholesale

Our Default theme keeps warm orange `#FF6B35` for brand identity. But when we introduce additional colors (charts, graphs, tags, sparklines, diffs) we **must** use the Okabe-Ito palette above. Never pick random hues.

### Sub-rule: red ↔ green avoidance

Our **error** stays red because it's paired with `XCircle` + "Error" text.  
Our **success** uses a hue with distinct *luminance* from error (Bluish Green `#009E73` reads darker than pure Red `#DD3C3C` under deuteranopia) AND a check icon.

---

## 6. Component rules

### Status indicators

| Variant | Icon (lucide) | Default color | High-Contrast color | Required text |
|---|---|---|---|---|
| Success | `CheckCircle2` | `#009E73` / `--status-success` | `#00FF66` | "OK" / "Passed" / "Available" |
| Warning | `TriangleAlert` | `#F0E442` / `--status-warning` | `#FFDD00` | "Warning" / "Needs attention" |
| Error | `XCircle` | `#D55E00` / `--status-error` | `#FF3344` (+ bold) | "Error" / "Failed" |
| Info | `Info` | `#56B4E9` / `--status-info` | `#66CCFF` | "Info" / "Note" |
| Loading | `Loader2` (spin) | `--accent-primary` | `#FFDD00` ring | "Loading" / "Working" |

**Rule:** You can't ship a red dot without an `XCircle` + "Error" label. Period.

### Buttons

- Primary: solid orange (Default) / solid yellow (HC) — distinct by shape (rounded filled rect) + text label. No color-only variants.
- Destructive: red fill + white text + `Trash` or `XCircle` icon prefix.
- Disabled: 50% opacity + `aria-disabled="true"` + cursor `not-allowed`. Never rely on "greyed-out red" to convey disabled-destructive.

### Links

```css
a { color: var(--accent-primary); text-decoration: underline; text-underline-offset: 2px; }
a:hover { text-decoration-thickness: 2px; }
```

**Always underlined.** No exceptions in running prose. Nav links can drop the underline because position + parent nav conveys link-ness.

### Required form fields

```
[Label] *(required)
[text input]
```

Render the `*` in `--accent-primary` AND include `aria-label="required"` on the `*`. The `(required)` text is for screen readers and can be `.sr-only` visually. Never rely on red-bordered input to mean "required".

### Diff viewers (code)

Lines prefixed with `+` / `−` in a mono font, colored green/red as decoration. The `+/−` glyph is the authoritative signal.

```
+ const foo = "bar";
- const foo = "baz";
```

### Charts (heatmaps / sparklines / series)

- **Series distinction:** use Okabe-Ito palette in order (orange → sky blue → bluish green → yellow → blue → vermillion → reddish purple). First 4 are safe for all CVD types in combination.
- **Line markers:** every line chart series gets a distinct marker shape (circle / square / triangle / diamond). Shape is the primary distinguisher, color is a bonus.
- **Heatmap legend:** always show numeric labels alongside color cells.
- **Sequential data (low → high):** use single-hue scales with clear luminance ramp — ColorBrewer "Blues" or "YlOrRd" are CVD-safe. Never "Spectral" or rainbow.

### Kanban columns

Column background color is decoration. Each column header carries:
- Icon (clock for backlog, bolt for executing, check for done, warning-triangle for blocked)
- Text label
- Count badge
- Status is readable from icon + text alone

### Tags / badges

Tag backgrounds can use the Okabe-Ito palette BUT the tag text must carry the meaning. Never ship an unlabeled colored dot as a tag.

### Toasts

- Position differs by kind: errors top-center (attention), success bottom-right (ambient), info bottom-center.
- Every toast includes an icon on the left.
- Error toasts include `role="alert"` / `aria-live="assertive"`; others `aria-live="polite"`.

---

## 7. Testing protocol

| When | Tool | Pass criterion |
|---|---|---|
| Design spec review | Coblis (color-blindness-simulator.com) or Stark plugin | All status states readable under deuteranopia + protanopia + tritanopia + grayscale |
| PR with new colors | Okabe-Ito check — palette member or documented exception | Exception must be CVD-tested and added to this spec |
| Pre-release smoke | `macOS → Accessibility → Display → Colour Filters` toggle | Navigate 3 key flows (Kanban / Chat / Settings) under each filter |
| Automated | `@axe-core/react` in test suite | 0 violations on target pages |

Every new component test should include at least one assertion that the status is readable WITHOUT color — e.g., `expect(screen.getByText(/available/i)).toBeInTheDocument()` alongside any color-dependent assertion.

---

## 8. Current-state audit (2026-04-18)

### Good

| Location | Why it passes |
|---|---|
| `InlineNotice` (harness pages) | Status label + icon + border colour — color is redundant |
| Settings left-nav active state | White-on-dark surface change (not color-only) |
| High-Contrast theme status tokens | Pre-paired with required icons in theme spec |

### Needs attention (TODO)

| Location | Gap | Fix |
|---|---|---|
| Kanban column color strips | Columns may rely on hue alone | Add icon + count to every column header |
| Activity stream dots | Colored dots without text | Wrap with `<VisuallyHidden>` label + icon |
| Chart components (if/when added) | No palette locked in | Default chart series to Okabe-Ito order |
| Toast notifications | Verify every variant has an icon | Audit `sonner` toast configuration |
| Diff viewer | Verify `+`/`−` glyph is always present | Already correct in `@git-diff-view/react` — keep |
| Required field asterisks | Currently handled ad hoc | Add `<RequiredMark>` component with built-in `aria-label` |
| Link styling | Default `a` may drop underline | Set default `text-decoration: underline` in `globals.css` for `article` / markdown surfaces |

---

## 9. Component API: `RequiredMark` (to add)

Shared component that renders `*` with accessibility semantics baked in:

```tsx
<label>
  Task title <RequiredMark />
</label>

// RequiredMark.tsx
export function RequiredMark() {
  return (
    <span
      className="text-[var(--accent-primary)] ml-0.5"
      aria-label="required"
    >
      *
    </span>
  );
}
```

---

## 10. References

- [WCAG 2.1 Success Criterion 1.4.1 — Use of Color (W3C)](https://www.w3.org/WAI/WCAG21/Understanding/use-of-color.html)
- [Okabe-Ito palette hex codes & CVD rationale](https://conceptviz.app/blog/okabe-ito-palette-hex-codes-complete-reference)
- [Color blindness types + prevalence (Wikipedia)](https://en.wikipedia.org/wiki/Color_blindness)
- [Designing for Color Blindness — Colorblind.io](https://colorblind.io/guides/designing-for-color-blindness)
- [WebAIM — Contrast and Color Accessibility](https://webaim.org/articles/contrast/)
- [MDN — Use of Color (a11y guide)](https://developer.mozilla.org/en-US/docs/Web/Accessibility/Guides/Understanding_WCAG/Perceivable/Use_of_color)
- [National Eye Institute — Types of Color Vision Deficiency](https://www.nei.nih.gov/learn-about-eye-health/eye-conditions-and-diseases/color-blindness/types-color-vision-deficiency)
- [Coblis color-blindness simulator](https://www.color-blindness.com/coblis-color-blindness-simulator/)
- [Stark — Figma/Sketch accessibility plugin](https://www.getstark.co/)
- [ColorBrewer 2 — colorblind-safe palettes](https://colorbrewer2.org/)
