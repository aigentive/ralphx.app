# RalphX Styleguide

> **Single source of truth** for the design system — token architecture, component patterns, layout primitives, and contribution rules.
>
> **Themes:** Dark (default) · Light · High-Contrast. Components consume semantic tokens and stay theme-agnostic. See `specs/design/theme-architecture.md` and `specs/design/themes/high-contrast.md`.

**Related docs:**
- `specs/DESIGN.md` — product design spec
- `specs/design/accessibility.md` — mandatory a11y rules
- `specs/design/color-blind-design.md` — CVD rules + Okabe-Ito palette
- `specs/design/theme-architecture.md` — multi-theme architecture
- `specs/design/themes/high-contrast.md` — High-Contrast theme spec
- `specs/design/macos-tahoe-style-guide.md` — aesthetic reference
- `frontend/src/CLAUDE.md` — frontend implementation rules

**Design philosophy:** macOS Tahoe — flat surfaces, blue-gray tinted neutrals, warm orange accent used sparingly. No gradients, glassmorphism, or purple.

---

## 0. Token architecture (3 tiers)

| Tier | File | Purpose | Named by | Consumers |
|---|---|---|---|---|
| **1. Primitives** | `src/styles/tokens/primitives.css` | Raw palette scales, spacing, radii, shadows, typography | **Value** (`--gray-925`, `--orange-500`, `--cvd-bluish-green`) | Semantic layer only — never by components |
| **2. Semantic** | `src/styles/tokens/semantic.css` | Role names mapped to primitives; `:root` = Dark theme | **Role** (`--bg-surface`, `--text-primary`, `--status-success`) | Components consume these |
| **3. Components** | `src/styles/tokens/components.css` | Per-component composite tokens | **Component-context** (`--dialog-bg`, `--notice-ok-bg`, `--input-border-focus`) | Specific components |
| **Themes** | `src/styles/themes/{light,high-contrast}.css` | Override semantic + component tokens per `data-theme` | Same role names | Cascade — components don't change |

**Rule of thumb for adding tokens:**
- New raw color / size → `primitives.css`
- New role the app reasons about → `semantic.css`
- Reusable composite for a component → `components.css`
- Theme-specific override → corresponding `themes/*.css`

**Component rule:** never reference primitives directly. Always consume the semantic layer (or component-tier tokens). This keeps theme switching a one-file change.

```css
/* ❌ bad — component references primitive directly */
.my-card { background: var(--gray-925); }

/* ✅ good — semantic role */
.my-card { background: var(--bg-elevated); }

/* ✅ good — component-tier composite when multiple props vary together */
.my-card { background: var(--card-bg); }
```

---

## 1. Color tokens

Values below describe the **Dark theme** (the `:root` defaults). Light + High-Contrast values appear in each theme file and are tabulated in `themes/*.md`.

### Backgrounds

| Token | HSL | Hex | Use |
|---|---|---|---|
| `--bg-base` | hsl(220 10% 8%) | `#121416` | App root background |
| `--bg-surface` | hsl(220 10% 12%) | `#1C1E22` | Side rails, popover surfaces, dialog body |
| `--bg-elevated` | hsl(220 10% 16%) | `#25272D` | Cards, dropdowns, modals on top of surface |
| `--bg-hover` | hsl(220 10% 20%) | `#2E3138` | Row hover, subtle interactive feedback |

**Rule:** Never hardcode `rgba(45,45,45,0.3)` / `rgba(255,255,255,0.04)` for hover — use `var(--bg-hover)`. The deprecated `--bg-surface-hover` alias found in a few files must be migrated to `--bg-hover`.

### Text

| Token | HSL | Hex | Use |
|---|---|---|---|
| `--text-primary` | hsl(220 10% 90%) | `#E3E5E8` | Body, labels, active states |
| `--text-secondary` | hsl(220 10% 60%) | `#8F96A3` | Subtitles, field names, inactive tab text |
| `--text-muted` | hsl(220 10% 45%) | `#676F7E` | Helper text, descriptions, placeholders |

### Accent (warm orange)

| Token | HSL | Hex | Use |
|---|---|---|---|
| `--accent-primary` | hsl(14 100% 60%) | `#FF6B35` | Primary actions, icons, focused borders |
| `--accent-secondary` | hsl(32 100% 65%) | `#FFAB4D` | Hover on primary buttons |
| `--accent-hover` | hsl(14 100% 55%) | `#FF5419` | Hover on flat accent text |
| `--accent-muted` | hsla(14 100% 60% / 0.15) | `rgba(255,107,53,0.15)` | Tinted backgrounds on section icons, selection states |

**Rule:** Orange is **the only** accent. ❌ No purple. ❌ No blue call-to-action. Use accent sparingly — primary buttons, focused borders, active-section dots, status badges.

### Status (badges / alerts only)

| Token | HSL | Hex | Use |
|---|---|---|---|
| `--status-success` | hsl(145 60% 45%) | `#2EB867` | Healthy state, green check |
| `--status-warning` | hsl(45 90% 55%) | `#F4C025` | Caution, amber triangle |
| `--status-error` | hsl(0 70% 55%) | `#DD3C3C` | Errors, destructive actions |
| `--status-info` | hsl(220 80% 60%) | `#477EEB` | Informational blue |

### Borders

| Token | HSL | Hex | Use |
|---|---|---|---|
| `--border-subtle` | hsl(220 10% 18%) | `#292C32` | Dividers inside cards, secondary separators |
| `--border-default` | hsl(220 10% 22%) | `#32363E` | Input borders, strong section dividers |
| `--border-focus` | hsl(220 80% 60%) | `#477EEB` | Keyboard-focus ring on inputs |

**Rule:** Only use hardcoded `rgba(255,255,255,0.08)` as a temporary card edge glow where the design calls for a glass effect — otherwise prefer `border-subtle` / `border-default`.

---

## 2. Typography

| Role | Font | Size | Weight | Letter-spacing | Line-height |
|---|---|---|---|---|---|
| Display / H1 | `--font-display` | 20–24px | 600 | `-0.02em` | 1.2 |
| Section title (H3) | `--font-display` | 14px (0.875rem) | 600 | `-0.01em` | 1.3 |
| Lane / row title | `--font-display` | 15px | 600 | 0 | 1.3 |
| Body | `--font-body` | 14px | 400 | 0 | 1.5 |
| Label (field) | `--font-body` | 14px | 500 | 0 | 1.4 |
| Sub-label | `--font-body` | 12px (0.75rem) | 500 | 0 | 1.3 |
| Helper / description | `--font-body` | 12px | 400 | 0 | 1.5 |
| Eyebrow / caption | `--font-body` | 11px | 600 | `0.08em` uppercase | 1.3 |
| Mono / code | `--font-mono` | 12–13px | 400 | 0 | 1.4 |

**Fonts:** SF Pro (system). ❌ Never Inter.

---

## 3. Spacing (8pt grid)

| Token | Value | Tailwind |
|---|---|---|
| `--space-1` | 4px | `1` |
| `--space-2` | 8px | `2` |
| `--space-3` | 12px | `3` |
| `--space-4` | 16px | `4` |
| `--space-5` | 20px | `5` |
| `--space-6` | 24px | `6` |
| `--space-8` | 32px | `8` |
| `--space-10` | 40px | `10` |
| `--space-12` | 48px | `12` |
| `--space-16` | 64px | `16` |

**Standard paddings:**
- Card inner: `20px` all sides (`p-5`)
- Row vertical (compact): `12px` (`py-3`)
- Row vertical (comfortable): `24–32px` (`py-6`/`py-8`) — used for collapsible lanes
- Dialog header: `12px 16px` (`px-4 py-3`)

---

## 4. Radii

| Token | Value | Use |
|---|---|---|
| `--radius-sm` | 4px | Pills, tiny chips |
| `--radius-md` | 8px | Inputs, buttons, notices, table cells |
| `--radius-lg` | 12px | Section cards |
| `--radius-xl` | 16px | Dialogs |
| `--radius-full` | 9999px | Avatars, switches |

---

## 5. Shadows

| Token | Value | Use |
|---|---|---|
| `--shadow-xs` | 2-stop subtle | Default card resting state |
| `--shadow-sm` | 2-stop light | Popovers, small overlays |
| `--shadow-md` | 2-stop medium | Modals, heavy overlays |
| `--shadow-lg` | 2-stop strong | Dialogs at root |
| `--shadow-glow` | Focus ring | Keyboard focus on primary buttons |

**Rule:** Tahoe aesthetic — shadows are subtle. ❌ Never use drop-shadow as decoration.

---

## 6. Transitions

| Token | Value | Use |
|---|---|---|
| `--transition-fast` | 150ms ease | Hover, focus, toggle |
| `--transition-normal` | 200ms ease | Section transitions, tab content |
| `--transition-slow` | 300ms ease | Modal/dialog enter/exit |

---

## 7. Layout primitives

### Dialog shell (`Dialog` / `DialogContent`)

```
- Body:    bg-[var(--bg-surface)]
- Border:  1px solid var(--border-subtle)
- Radius:  rounded-xl (16px)
- Shadow:  --shadow-lg
- Header:  bg-[var(--bg-surface)], border-b border-[var(--border-subtle)], px-4 py-3
```

### Left rail / sidebar nav

```
- Width:          280px
- Bg:             transparent (sits on dialog --bg-surface)
- Border-right:   1px solid var(--border-subtle)
- Group eyebrow:  11px/600/uppercase/tracking-[0.08em] text-secondary opacity-60
- Item:           mx-2 rounded-md px-3 min-h-[36px] text-sm
- Item hover:     bg-[rgba(255,255,255,0.04)]   (will migrate → bg-hover/40)
- Item active:    bg:#FFFFFF · color:#0A0A0A · font-weight:600
```

### Section card (`SectionCard` in `SettingsView.shared.tsx`)

```
- Bg:        --bg-elevated (CANONICAL — no glassmorphism, no gradient border tricks)
- Border:    1px solid --border-subtle
- Radius:    rounded-lg (12px)
- Shadow:    --shadow-xs
- Header:    flex items-start gap-3 p-5 pb-0
- Icon box:  p-2 rounded-lg bg:--accent-muted border:1px solid rgba(255,107,53,0.2)
- Separator: my-4 bg:--border-subtle
- Body:      px-5 pb-5
```

### Setting row (`SettingRow`)

```
- Padding:  py-3 -mx-2 px-2
- Divider:  border-b border-[var(--border-subtle)]; last:border-0
- Hover:    hover:bg-[var(--bg-hover)]/30
- Label:    text-sm font-medium text-primary
- Desc:     text-xs text-muted mt-0.5
```

### Tabs

```
- TabsList:        h-9 rounded-md bg:--bg-surface p-1 border:--border-subtle
- TabsTrigger:     rounded-sm px-3 py-1 text-xs font-medium text-secondary
- Trigger[active]: bg:--bg-elevated text:--text-primary shadow-sm
```

---

## 8. Form components

### Select (shadcn Radix wrapper)

```
- Trigger:           h-9 items-center rounded-md border px-3 bg:--bg-surface border:--border-default
- Trigger[focus]:    focus:ring-1 focus:ring-[var(--accent-primary)]
- Content:           bg:--bg-elevated border:--border-default
- Item[focus]:       bg:--accent-muted
- Item descriptions: stacked flex-col with label (text-primary) + description (text-xs text-muted)
```

**Rule:** Triggers show ONLY the primary label (use `SelectValue` children). Descriptions belong in dropdown items — never truncated inside the trigger.

### Input / number / password

```
- bg:             --bg-surface
- border:         1px solid --border-default
- focus border:   --accent-primary
- focus ring:     1px --accent-primary
- Number inputs: hide native spinners via [&::-webkit-inner-spin-button]:appearance-none
```

### Switch

```
- bg[checked]:    --accent-primary
- bg[unchecked]:  --border-default
- Track:          22px × 12px (approx)
```

### Checkbox

```
- bg[checked]:       --accent-primary
- border[checked]:   --accent-primary
- border[unchecked]: --border-default
```

---

## 9. Feedback / notices (`InlineNotice`)

Three tones, always rounded-md + icon left + soft bg:

| Tone | Bg | Border | Icon | Use |
|---|---|---|---|---|
| `ok` | rgba(255,255,255,0.03) | --border-subtle | CheckCircle2 (`--status-success`) | Healthy state, detection confirmed |
| `warn` | rgba(251,146,60,0.06) | rgba(251,146,60,0.2) | TriangleAlert (`--warning`) | Missing features, attention needed |
| `info` | rgba(255,107,53,0.05) | rgba(255,107,53,0.18) | Info (`--accent-primary`) | Policy notes, locked-by-design info |

---

## 10. Interactive states

| State | Applied to | Spec |
|---|---|---|
| **Hover** | Buttons, rows | `hover:bg-[var(--bg-hover)]/30` or token |
| **Focus (keyboard)** | Inputs, buttons | `focus-visible:ring-1 focus-visible:ring-[var(--accent-primary)]` |
| **Active (nav)** | Sidebar item | `bg:#FFFFFF color:#0A0A0A font-weight:600` |
| **Disabled** | Any | `opacity-50 pointer-events-none` |
| **Loading** | Buttons | Replace text with `Loader2 animate-spin` |

---

## 11. Progressive disclosure

For pages with dense per-item config (e.g. harness lanes):
- Collapsed row: title + primary control + summary pills + compact status chip
- Expanded row: full form fields + notice cards
- Warnings force-expand (override user collapse state)
- Persist collapse state per-tab in localStorage via `settings-ui-state.ts`

---

## 12. Cohesiveness state + regression guards

As of 2026-04-18 the token migration is **complete** for all non-excluded components. Running totals from the end-of-session sweep:

| Check | Count | Source |
|---|---|---|
| Primitive-token leaks in components | **0** | `grep 'var\(--(gray\|orange\|amber\|yellow-\|blue-\|cvd\|hc\|alpha-)' src/components` |
| Tailwind default-palette refs | **0** | `grep '\b(bg\|text\|border\|ring\|from\|to\|via)-(red\|green\|blue\|amber\|emerald\|rose\|yellow\|indigo\|purple\|pink\|sky\|slate\|zinc\|neutral\|stone)-[0-9]{2,3}\b' src/components` (excluding `.test.` + WelcomeScreen + BattleModeV2) |
| Inline `rgba(…)` / `rgb(…)` literals | **0** | `grep 'rgba\(\|rgb\(' src/components` (excluding test/WelcomeScreen/BattleModeV2/`color-mix`) |
| `bg-surface-hover` references | **0** | Deprecated alias — kept in CSS for emergency safety, not used |
| Brand hex in live non-doc code | **0** | Docstrings/comments still mention `#ff6b35` for humans — acceptable |
| Full test suite | **7518 / 7518** | `npx vitest run` |

### Intentional exclusions

These paths are out of scope for token migration:

| Path | Reason |
|---|---|
| `src/components/WelcomeScreen/**` | Marketing splash with an intentional fixed brand palette + radial gradients. Not theme-responsive by design. |
| `src/components/TaskGraph/battle-v2/BattleModeV2Overlay.tsx` | Neon-cyberpunk game-mode canvas; colours are gameplay art, not UI surface. |
| `*.test.tsx` / `*.test.ts` | Assertions pin expected token/class names. Update tests when source semantics change. |
| `src/styles/**` | Token/theme sources. Hardcoded values here are intentional — that's where the values live. |

### CI regression guards (recommended, not yet wired)

Add to `npm run lint` as a pre-commit step or a CI job:

```bash
# Fail the build if any primitive-tier tokens leak into components
grep -rEn 'var\(--(gray|orange|amber|yellow-[0-9]|blue-[0-9]|cvd|hc|alpha-)' \
  frontend/src/components --include='*.tsx' --include='*.ts' \
  | grep -v 'WelcomeScreen' \
  && { echo "Primitive leak detected"; exit 1; }

# Fail the build if any Tailwind default palette leaks into components
grep -rEn '\b(bg|text|border|ring|from|to|via)-(red|green|blue|amber|emerald|rose|yellow|indigo|purple|pink|sky|slate|zinc|neutral|stone)-[0-9]{2,3}\b' \
  frontend/src/components --include='*.tsx' --include='*.ts' \
  | grep -v '\.test\.' | grep -v 'WelcomeScreen' | grep -v 'BattleModeV2' \
  && { echo "Tailwind palette leak detected"; exit 1; }
```

### Adding a new token — canonical checklist

1. Decide the tier:
   - Raw scale value → `src/styles/tokens/primitives.css`
   - New role the app reasons about → `src/styles/tokens/semantic.css`
   - Reusable composite for a specific component → `src/styles/tokens/components.css`
2. Add default (Dark) value to the correct file. If semantic, **reference a primitive** (`--bg-x: var(--gray-925)`), never a raw literal.
3. Add override in `src/styles/themes/light.css`.
4. Add override in `src/styles/themes/high-contrast.css` **with computed contrast ratio in the commit message**.
5. If Tailwind needs to consume it, register in the `@theme inline` block in `globals.css` (`--color-your-token: var(--your-token);`).
6. Document in this file's relevant section (§1 colour tokens, §3 spacing, §4 radii, §5 shadows, etc.).
7. Use via `bg-[var(--token)]` / `text-[var(--token)]` or the Tailwind palette utility if registered. Never inline rgba/hex in components.

---

## 13. Contribution rules

- **Never hardcode `rgba()`, `hsl()`, or hex in component files.** Use the semantic/component tier. If a needed value doesn't exist, add the token first (see §12 checklist).
- **Never reference primitives directly from components.** `var(--gray-*)`, `var(--orange-*)`, `var(--cvd-*)`, `var(--hc-*)`, `var(--yellow-N)`, `var(--blue-N)`, `var(--alpha-*)` belong inside `tokens/semantic.css` or theme files, not `.tsx`.
- **Use Tailwind semantic utilities when available** — `text-status-error`, `bg-accent-primary`, `text-text-primary/70` — before dropping to arbitrary-value syntax.
- **When a pattern repeats 3+ times, promote it.** Either add a component token to `tokens/components.css` or (better) a shared component under `components/ui/`.
- **One accent color only.** Orange. ❌ No purple / blue / green for decorative purposes.
- **Prefer composition over variants.** `<SectionCard>`, `<SettingRow>`, `<InlineNotice>` — use them. Don't rebuild the card shell inline.
- **Test assertions follow the token names.** If you migrate a source literal to a token, grep the paired test file and update `.toHaveStyle` / `.toHaveClass` assertions.
- **Use `withAlpha(token, %)` from `@/lib/theme-colors`** for dynamic-expression alpha composition. Never concatenate hex strings (`` `${color}80` `` is the banned pattern).
- **Shadows are tokens too.** `--shadow-xs/sm/md/lg`, `--shadow-pulse-*`, `--shadow-glow-*`, `--shadow-drop-zone-*` — pick one before inlining a new box-shadow stack.
- **Keyframes consume tokens** so animations flip themes. See the `@keyframes executing-pulse` / `reviewing-pulse` / `status-pulse` block in `globals.css` as the pattern.
- **Excluded paths are documented in §12.** Anything else must follow these rules.
