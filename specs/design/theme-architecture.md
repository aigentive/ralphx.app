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

## 4. File layout (shipped — 2026-04-18)

```
frontend/src/styles/
├── globals.css              entry — imports + Tailwind @theme inline,
│                            base body/scrollbar/motion/font-scale,
│                            diff-viewer overrides, keyframes
├── tokens/
│   ├── primitives.css       Tier 1 — raw scales (gray 50-975, orange,
│   │                        amber, yellow, blue, Okabe-Ito CVD-safe,
│   │                        HC brights, alphas 2-70, spacing,
│   │                        radii, shadows, typography)
│   ├── semantic.css         Tier 2 — role tokens + shadcn bridge
│   │                        (:root = Dark theme default)
│   └── components.css       Tier 3 — per-component composites
│                            (dialog, input, button, card, notice,
│                            diff, overlay ladder, shadow-pulse)
└── themes/
    ├── light.css            [data-theme="light"] overrides
    └── high-contrast.css    [data-theme="high-contrast"] overrides
```

Tier rule: primitives → semantic → components → themes override semantic+component.
Components consume Tier 2 + 3 only, never Tier 1.

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

## 8. Themes shipped + planned

| Theme | `data-theme` | Status |
|---|---|---|
| **Dark** | (none — matches `:root`) | ✅ shipped |
| **Light** | `data-theme="light"` | ✅ shipped |
| **High-Contrast** | `data-theme="high-contrast"` | ✅ shipped |
| **Sepia / low-blue** | TBD | 🗒 considered |
| **Custom brand themes** | TBD | 🗒 long horizon |

Adding a new theme = one new file under `themes/<name>.css` with the `[data-theme="<name>"]` selector, overrides for the semantic tokens that need to change, and registration in `themeStore.ts` + `AccessibilitySection.tsx`. No component changes required.

---

## 9. Settings UI for theme toggle (shipped)

```
Settings → Preferences → Accessibility
 ├─ [Select] Theme                   Dark (default) / Light / High contrast
 ├─ [Switch] High contrast mode      shortcut — forces HC; restores last base on off
 ├─ [Select] Motion                  Follow system / Always reduce
 └─ [Select] Font size               Default / Large (110%) / Extra large (125%)
```

Persistence: `localStorage.ralphx-theme`, `localStorage.ralphx-motion`, `localStorage.ralphx-font-scale`, `localStorage.ralphx-last-base-theme`. All applied as `data-*` attrs on `<html>` by the inline bootstrap script in `index.html`; React re-asserts on mount via `syncThemeAttributesFromStore()` in `main.tsx`.

---

## 10. Ship checklist — status 2026-04-18

- [x] `globals.css` split into tokens + themes (`primitives.css` / `semantic.css` / `components.css` / `themes/light.css` / `themes/high-contrast.css`)
- [x] Every hardcoded color in `frontend/src/components/**` migrated to a token (see `styleguide.md` §12 end-of-session sweep)
- [x] `themes/high-contrast.css` values match `themes/high-contrast.md` spec
- [x] Pre-hydration bootstrap added to `index.html`
- [x] `Settings → Accessibility` panel with theme selector + HC toggle + motion + font-scale
- [ ] Axe-core tests in both themes for Settings + Chat + Kanban (planned, not wired)
- [ ] Visual regression snapshots per theme (planned)
- [ ] Manual keyboard sweep in all 3 themes (pending)
- [ ] Manual VoiceOver sweep on Settings + Chat (pending)
- [ ] CI guard: fail build on primitive-leak grep / Tailwind-palette grep (see styleguide §12)
