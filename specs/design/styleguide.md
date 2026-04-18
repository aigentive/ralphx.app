# RalphX Styleguide — Default Theme

> **Initial spec — this will grow.** Single source of truth for tokens, components, and layout patterns across the app. Tokens live in `frontend/src/styles/globals.css`; this doc documents them with hex values + canonical usage.
>
> **Theme note:** The values here describe the **Default** theme. A future **High-Contrast** theme remaps the same tokens — see `specs/design/themes/high-contrast.md` and `specs/design/theme-architecture.md`. Components must consume tokens, never hardcode hex/rgba, so a single component works in both themes.

**Related docs:**
- `specs/DESIGN.md` — product design spec
- `specs/design/accessibility.md` — mandatory a11y rules
- `specs/design/theme-architecture.md` — multi-theme architecture
- `specs/design/themes/high-contrast.md` — High-Contrast theme spec
- `specs/design/macos-tahoe-style-guide.md` — aesthetic reference
- `frontend/src/CLAUDE.md` — frontend implementation rules

**Design philosophy:** macOS Tahoe — flat surfaces, blue-gray tinted neutrals, warm orange accent used sparingly. No gradients, glassmorphism, or purple.

---

## 1. Color tokens

All values source from `frontend/src/styles/globals.css`. Hex columns are computed from HSL — HSL is the source of truth.

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

## 12. Known drift — blocks High-Contrast toggle (migrate before shipping it)

Every row below is a hardcoded color in a component file. In the Default theme the override happens to look correct; in any other theme it breaks. Each must become a token reference before the High-Contrast toggle can ship.

### Settings surface (audited 2026-04-18)

| File | Line(s) | Issue | Fix |
|---|---|---|---|
| `SettingsView.shared.tsx` | 284-290 | `SectionCard` uses `rgba(255,255,255,0.04)` bg + blur + `rgba(255,255,255,0.08)` border (glassmorphism) | Switch to `var(--bg-elevated)` + `var(--border-subtle)` |
| `IdeationSettingsPanel.tsx` | 244-247 | Gradient-border trick on Card | Use shared `SectionCard` |
| `ExternalMcpSettingsPanel.tsx` | 178-183 | Same gradient-border trick | Use shared `SectionCard` |
| `ExternalMcpSettingsPanel.tsx` | 32 | RestartNotice: `rgba(255,107,53,0.08)` bg + `rgba(255,107,53,0.2)` border | Tokenize to `--accent-muted` + new `--accent-border` |
| `ExternalMcpSettingsPanel.tsx` | 53 | FieldRow hover: `hover:bg-[rgba(45,45,45,0.3)]` | `hover:bg-[var(--bg-hover)]/30` |
| `ExternalMcpSettingsPanel.tsx` | 313 | Save button: `text-white` | Use `text-[var(--text-inverse)]` so HC flips correctly |
| `IdeationSettingsPanel.tsx` | 55 | Hover: `rgba(45,45,45,0.3)` | `var(--bg-hover)` |
| `SettingsDialog.tsx` | 99 | DialogContent border: `rgba(255,255,255,0.08)` | `var(--border-subtle)` |
| `SettingsDialog.tsx` | 110-113 | Header bg: `rgba(18,18,18,0.85)` + `rgba(255,255,255,0.06)` border | `var(--bg-surface)` + `var(--border-subtle)` |
| `SettingsDialog.tsx` | 131 | Close btn hover: `rgba(255,255,255,0.06)` | `var(--bg-hover)` |
| `SettingsDialog.tsx` | 168 | Nav hover: `rgba(255,255,255,0.04)` | `var(--bg-hover)/40` |
| `SettingsDialog.tsx` | 172-174 | Nav active: `#ffffff` / `#0a0a0a` hardcoded | Define `--nav-active-bg` / `--nav-active-text` tokens (HC: yellow / black) |
| `ApiKeyEntry.tsx` | 179-180 | Accent card: `rgba(255,107,53,0.08)` + `rgba(255,107,53,0.15)` | `--accent-muted` + `--accent-border` |
| `ApiKeyEntry.tsx` | 211, 244, 271, 292, 306 | `hover:bg-[var(--bg-surface-hover)]` — token doesn't exist | Migrate to `var(--bg-hover)` |
| `ProjectAnalysisSection.tsx` | 60-61, 217-218 | `rgba(255,107,53,0.05)` + `rgba(255,107,53,0.15)` | Tokenize |
| `ProjectAnalysisSection.tsx` | 160, 205, 229 | `rgba(255,107,53,0.08)` hover + `bg-surface-hover` | Tokenize |
| `ProjectMultiSelect.tsx` | 73-74 | Mix of `rgba(255,107,53,0.08)` and `bg-surface-hover` | Tokenize |
| `EditableAnalysisEntry.tsx` | 87, 144, 153, 167, 201, 207, 308 | Hardcoded orange alpha, gray alpha, red alpha, and `bg-surface-hover` | Full tokenization pass |
| `IdeationEffortSection.tsx` | 116 | Hover: `rgba(45,45,45,0.3)` | `var(--bg-hover)` |
| `CreateKeyDialog.tsx` | 145-146, 261-262 | Inline `var(--bg-elevated)` + `rgba(255,255,255,0.08)` / `rgba(0,0,0,0.3)` | Use `SectionCard` + token borders |
| `AuditLogViewer.tsx` | 94 | `bg-surface-hover` | `var(--bg-hover)` |

### Cross-cutting rules

| Pattern | Fix |
|---|---|
| Any reference to `--bg-surface-hover` | Rename callsites to `--bg-hover`. Optionally add a temporary CSS alias `--bg-surface-hover: var(--bg-hover)` in `globals.css` to de-risk the migration |
| `rgba(255,107,53,*)` in any alpha | Use `--accent-primary` + `color-mix(in srgb, var(--accent-primary) N%, transparent)` OR tokenize the specific alpha (`--accent-muted` for 0.15) |
| `rgba(239,68,68,*)` | Use `--status-error` with `color-mix` |
| `rgba(255,255,255,0.04\|0.06\|0.08)` overlays | Introduce `--overlay-subtle` / `--overlay-default` tokens; never use white-alpha directly |
| `#FFFFFF` / `#000000` as component literals | Use `--text-primary` / `--text-inverse` / theme-specific tokens |

### Migration phases

| Phase | Scope | Gate |
|---|---|---|
| 1 | Tokenize every hardcoded color in the **Settings** surface (table above) | No visual change in Default theme (snapshot tests + visual diff) |
| 2 | Extract `globals.css` into `tokens/*.css` + `themes/default.css` | `:root` still resolves to identical values |
| 3 | Add `themes/high-contrast.css` mirroring `themes/high-contrast.md` | All contrast pairs logged in the theme file |
| 4 | Pre-hydration bootstrap script in `index.html` | No flash-of-wrong-theme on reload |
| 5 | Add **Settings → Accessibility** panel (High contrast / Reduce motion / Font scale) | Manual QA pass in both themes |
| 6 | Extend tokenization to Chat, Kanban, Task detail views | Axe-core tests pass in both themes |

### Checklist: adding a new token

1. Declare the role (name only) in `frontend/src/styles/tokens/roles.css`.
2. Add the default value in `frontend/src/styles/themes/default.css`.
3. Add the high-contrast value in `frontend/src/styles/themes/high-contrast.css` **with the computed contrast ratio in the commit message**.
4. Document the token in the appropriate table in this file and in `themes/high-contrast.md`.
5. Use it in components via `bg-[var(--token)]` / `text-[var(--token)]` — never via Tailwind palette shortcuts.

---

## 13. Contribution rules

- **Always add new tokens to `globals.css` first.** Never hardcode rgba() or hsl() in component files.
- **Use Tailwind arbitrary values (`bg-[var(--token)]`) for tokens.** Don't invent new class names.
- **When introducing a new UI pattern, add it to this doc** with canonical class string under the appropriate section.
- **When you fix drift listed in §12, remove it from the table** in the same PR.
- **One accent color only.** Orange. ❌ No purple / blue / green for decorative purposes.
- **Prefer composition over variants.** `<SectionCard>`, `<SettingRow>`, `<InlineNotice>` — use them. Don't rebuild the card shell inline.
