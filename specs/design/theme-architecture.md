# RalphX Theme Architecture

> **Purpose:** How multiple themes (Default, High-Contrast, future variants) share the same component tree without forking code. Components consume tokens; themes define token values.
>
> **Related:**
> - `specs/design/styleguide.md` — tokens + component patterns (Default values)
> - `specs/design/accessibility.md` — a11y requirements every theme must satisfy
> - `specs/design/themes/high-contrast.md` — the second theme spec

---

## 1. Principle: two layers, one component

| Layer | Responsibility | Location |
|---|---|---|
| **Token layer** | Names semantic CSS variables (`--bg-surface`, `--text-primary`, `--accent-primary`, …). One name per role | `frontend/src/styles/globals.css` |
| **Theme layer** | Assigns concrete values (hex, hsl, px) to token names. Multiple themes override the same names | `frontend/src/styles/themes/*.css` (new) |
| **Component layer** | References tokens only (`bg-[var(--bg-surface)]`). Never hardcodes colors or sizes | `frontend/src/components/**` |

**Rule:** a component should work correctly in **every** theme without conditional logic. If a component renders differently per theme, the theme has gaps in its token coverage — not the component.

---

## 2. Theme activation

### DOM attribute
Themes activate via a `data-theme` attribute on `<html>`:

```html
<html data-theme="default">     <!-- or "high-contrast" -->
```

CSS selectors scope variable definitions:

```css
:root,
[data-theme="default"] {
  --bg-surface: hsl(220 10% 12%);
  --text-primary: hsl(220 10% 90%);
  /* … */
}

[data-theme="high-contrast"] {
  --bg-surface: #000000;
  --text-primary: #ffffff;
  /* … */
}
```

`:root` carries the Default theme so the app renders correctly before any JS runs.

### Runtime toggle

A tiny bootstrap script runs **synchronously** in `index.html` to avoid a flash-of-wrong-theme (FOWT):

```html
<script>
  (function () {
    try {
      const stored = localStorage.getItem('ralphx-theme');
      if (stored === 'high-contrast') {
        document.documentElement.setAttribute('data-theme', 'high-contrast');
      }
    } catch (_) { /* no-op */ }
  })();
</script>
```

The React app reads/writes `localStorage.ralphx-theme` via a small store (Zustand or direct wrappers, mirroring `settings-ui-state.ts`). Toggling calls `document.documentElement.setAttribute('data-theme', next)`.

### Accessibility signal (system preference)

Listen to `window.matchMedia('(prefers-contrast: more)')` on mount. If the user's OS asks for more contrast AND they have never manually chosen a theme, default to `high-contrast`. Manual choice always wins.

```tsx
const prefersMoreContrast = matchMedia('(prefers-contrast: more)').matches;
const userChoice = localStorage.getItem('ralphx-theme');
if (!userChoice && prefersMoreContrast) {
  document.documentElement.setAttribute('data-theme', 'high-contrast');
}
```

---

## 3. Token taxonomy

The token layer defines **roles**, not raw colors. Each token must be usable across every theme.

### Background roles

| Token | Role | Default | High-Contrast |
|---|---|---|---|
| `--bg-base` | App root | `hsl(220 10% 8%)` | `#000000` |
| `--bg-surface` | Panels, sidebars | `hsl(220 10% 12%)` | `#000000` |
| `--bg-elevated` | Cards, dropdowns | `hsl(220 10% 16%)` | `#000000` with white border |
| `--bg-hover` | Row hover | `hsl(220 10% 20%)` | `#FFFFFF` text-inverse on hover |
| `--bg-active` | Active row / selected | (new) `hsl(14 100% 60%)` at 0.12 | `#FFDD00` with black text |

### Foreground roles

| Token | Role | Default | High-Contrast |
|---|---|---|---|
| `--text-primary` | Body | `hsl(220 10% 90%)` | `#FFFFFF` |
| `--text-secondary` | Subtitles | `hsl(220 10% 60%)` | `#E0E0E0` |
| `--text-muted` | Helper | `hsl(220 10% 45%)` | `#B0B0B0` (still ≥ 7:1 on black) |
| `--text-inverse` | On accent bg | `hsl(220 10% 6%)` | `#000000` |

### Interactive roles

| Token | Role | Default | High-Contrast |
|---|---|---|---|
| `--accent-primary` | CTA, focused borders | `hsl(14 100% 60%)` `#FF6B35` | `#FFDD00` (yellow; orange fails AAA on black) |
| `--accent-hover` | CTA hover | `hsl(14 100% 55%)` | `#FFEE33` |
| `--accent-muted` | Tinted bg | `hsla(14 100% 60% / 0.15)` | `#FFFFFF` at 0.15 |
| `--focus-ring` | Keyboard focus | `hsl(220 80% 60%)` | `#FFDD00` + 3px |
| `--border-subtle` | Inner dividers | `hsl(220 10% 18%)` | `#FFFFFF` at 0.5 |
| `--border-default` | Input borders | `hsl(220 10% 22%)` | `#FFFFFF` solid |
| `--border-strong` | New — for high-contrast emphasis | = `--border-default` | `#FFFFFF` 2px |

### Status roles

| Token | Default | High-Contrast |
|---|---|---|
| `--status-success` | `hsl(145 60% 45%)` `#2EB867` | `#00FF66` on black (or inverse black-on-lime) |
| `--status-warning` | `hsl(45 90% 55%)` `#F4C025` | `#FFDD00` on black |
| `--status-error` | `hsl(0 70% 55%)` `#DD3C3C` | `#FF3333` + thick stroke |
| `--status-info` | `hsl(220 80% 60%)` `#477EEB` | `#66CCFF` on black |

### Size / shape roles (theme-adjustable)

| Token | Default | High-Contrast |
|---|---|---|
| `--border-width-default` | `1px` | `2px` |
| `--border-width-focus` | `2px` | `3px` |
| `--radius-lg` | `12px` | `8px` (sharper corners read better at small sizes on high-contrast) |
| `--font-size-base` | `14px` | `15px` (slight bump) |

---

## 4. File layout

```
frontend/src/styles/
├── globals.css              # imports all tokens + themes
├── tokens/
│   ├── roles.css            # semantic role names (declared, no values)
│   ├── spacing.css          # 8pt grid
│   ├── typography.css       # font tokens
│   └── radius-shadow.css    # radii + shadows
└── themes/
    ├── default.css          # :root + [data-theme="default"] overrides
    └── high-contrast.css    # [data-theme="high-contrast"] overrides
```

**Migration plan:** Current `globals.css` is monolithic. Phase 1 splits it into `themes/default.css` + `tokens/*` without changing any values; Phase 2 adds `themes/high-contrast.css`.

---

## 5. What components must NOT do

| ❌ Anti-pattern | ✅ Correct |
|---|---|
| `style={{ background: "#1a1a1a" }}` | `className="bg-[var(--bg-elevated)]"` |
| `className="bg-[rgba(255,255,255,0.04)]"` | Use `bg-elevated` or add a new token if needed |
| `className="text-white"` | `text-[var(--text-primary)]` |
| Conditional: `theme === "hc" ? X : Y` | Let CSS vars handle it |
| Inline `border: "1px solid #333"` | `border border-[var(--border-subtle)]` |
| Hardcoded px sizes where the theme can change them | Token like `--border-width-default` |

The drift table in `styleguide.md` §12 catalogs current violations. All must be cleared before the toggle ships.

---

## 6. What components MAY do (per-theme escape hatches)

Rare cases where per-theme logic is unavoidable — e.g., swapping an icon fill style — use a `data-theme`-scoped CSS rule, **not** JS:

```css
[data-theme="high-contrast"] .status-dot {
  outline: 2px solid currentColor;
  background: var(--bg-base);
}
```

Components keep rendering the same markup; theme sheets add the override.

---

## 7. Tests and CI

| Check | Required for |
|---|---|
| Contrast computed for every token pair | Each theme spec file (`themes/*.md`) |
| Visual regression snapshots in both themes | New components via Playwright |
| Axe-core accessibility scan in both themes | Settings dialog, Chat panel, Kanban (first 3 targets) |
| "No hardcoded colors" lint rule | Custom ESLint rule scanning for hex/rgba literals in `components/**` — planned |

---

## 8. Future themes (out of scope but planned for)

| Candidate theme | Use case |
|---|---|
| **Light mode** | Users who prefer it or need it for photosensitive conditions |
| **Sepia / low blue** | Night-time reading |
| **Custom brand themes** | Per-workspace branding (long horizon) |

Architecture must allow any of these to slot in by adding a new `data-theme` value + CSS file. No component changes required.

---

## 9. Settings UI for theme toggle (future shape)

```
Settings → Accessibility
 ├─ [Switch] High contrast mode
 │    "Maximum contrast colors, thicker borders, shape-based status indicators."
 ├─ [Switch] Reduce motion
 │    "Disable animations beyond OS-level setting."
 └─ [Select] Font scale
        Default / Large (110%) / Extra large (125%)
```

Persistence: `localStorage.ralphx-theme`, `localStorage.ralphx-motion`, `localStorage.ralphx-font-scale`. All three apply as `data-*` attrs on `<html>` and are read by the bootstrap script pre-hydration.

---

## 10. Checklist before shipping the toggle

- [ ] `globals.css` split into tokens + themes
- [ ] Every hardcoded color in `frontend/src/components/**` migrated to a token (drift table in `styleguide.md` §12)
- [ ] `themes/high-contrast.css` values match `themes/high-contrast.md` spec exactly
- [ ] Pre-hydration bootstrap added to `index.html`
- [ ] `Settings → Accessibility` panel added with the toggle UI
- [ ] Axe-core tests pass in both themes for Settings + Chat panels
- [ ] All interactive elements have visible focus in both themes
- [ ] Manual keyboard sweep passes in both themes
- [ ] Manual VoiceOver sweep on Settings + Chat in both themes
