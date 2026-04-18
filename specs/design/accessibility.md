# RalphX Accessibility Spec

> **Scope:** Mandatory accessibility rules across all RalphX UI. Themes must comply; features must comply; tests must enforce compliance.
>
> **Targets:** WCAG 2.1 AA baseline for the Default theme, WCAG 2.1 AAA for the High-Contrast theme.

**Related:**
- `specs/design/styleguide.md` — tokens + components (Default theme)
- `specs/design/theme-architecture.md` — how theme switching works
- `specs/design/themes/high-contrast.md` — high-contrast override spec

---

## 1. Principles (NON-NEGOTIABLE)

| # | Principle | Implication |
|---|---|---|
| 1 | **Color is never the only signal** | Every status/state needs icon + text + color. ❌ "Red = error" alone. ✅ `⚠ 2 errors` |
| 2 | **Shape differentiates, color reinforces** | Colorblind users must read status from icon shape first; color adds emphasis |
| 3 | **Focus is always visible** | Every focusable element shows a ring on `:focus-visible`; never `outline: none` without a replacement |
| 4 | **Keyboard reaches everything** | No hover-only actions, no click-only drag. Every interaction has a keyboard path |
| 5 | **Motion is opt-out** | Respect `prefers-reduced-motion`. Animations ≤ 400ms, no parallax, no unsolicited motion on load |
| 6 | **Semantic HTML first** | `<button>` before `role="button"`. Form labels with `htmlFor`. Headings in order |
| 7 | **Screen readers never silent** | Every icon-only button has `aria-label`. Live updates announced via `aria-live` |
| 8 | **Contrast verified, not assumed** | Every token pair pre-computed + listed in this doc. New colors require a contrast check before shipping |

---

## 2. WCAG 2.1 targets

| Element | Default theme (AA) | High-Contrast theme (AAA) |
|---|---|---|
| Normal text (< 18px / < 14px bold) | **4.5:1** minimum | **7:1** minimum |
| Large text (≥ 18px or ≥ 14px bold) | **3:1** | **4.5:1** |
| UI components (borders, icons, form fields) | **3:1** | **4.5:1** |
| Focus ring vs. adjacent color | **3:1** | **4.5:1** + 3px minimum thickness |
| Status icon fill vs. card bg | **3:1** | **7:1** |
| Disabled state text | Exempt from WCAG, but still ≥ 3:1 if convenient | ≥ 4.5:1 |

**Testing:** Use the WebAIM contrast checker or axe-core. Any token pair combination documented in `themes/*.md` must include the computed ratio.

---

## 3. Color-blindness rules

Three most common conditions (covering ~8% of men, ~0.5% of women):

| Condition | Impact | Design rule |
|---|---|---|
| **Deuteranopia** (red/green weak) | ~6% of males | ❌ Never red-vs-green alone. Red/green semantics must carry an icon shape (✓, ✗, ⚠) and a label |
| **Protanopia** (red-blind) | ~1% of males | Red looks dark brown/gray. Combine with shape + high contrast |
| **Tritanopia** (blue/yellow, rare) | ~0.01% | Less common, but still — don't use blue-vs-yellow as sole signal |

### Status vocabulary (required everywhere)

| State | Icon (lucide) | Shape | Default color | High-contrast color | Text label (required) |
|---|---|---|---|---|---|
| **Success** | `CheckCircle2` | ⭕ filled-check | `--status-success` `#009E73` (Okabe-Ito bluish-green) | HC: `#00FF66` lime | "OK" / "Passed" / "Available" |
| **Warning** | `TriangleAlert` | △ triangle | `--status-warning` `#F0E442` (Okabe-Ito yellow) | HC: `#FFDD00` | "Warning" / "Attention" |
| **Error** | `XCircle` | ⭕ filled-x | `--status-error` `#D55E00` (Okabe-Ito vermillion) | HC: `#FF3344` + bold | "Error" / "Failed" |
| **Info** | `Info` | ⓘ circle-i | `--status-info` `#0072B2` (Okabe-Ito blue) | HC: `#66CCFF` sky | "Info" / "Note" |
| **In-progress** | `Loader2` (spin) | motion | `--accent-primary` `#FF6B35` | HC: `#FFDD00` yellow ring, no spin in reduced-motion | "Loading" / "Working" |

Light theme uses darker AA-on-white variants — success `#007A58`, warning `#A87800`, error `#B24800`, info `#005A8A`, accent `#EA5A26`. See `themes/light.md` §2 for the full table.

**Rule:** ❌ A green dot with no icon or text. ✅ A green `CheckCircle2` icon + "Available" label. In high-contrast, the fill changes and the text stays mandatory.

### Numeric-only data must have redundant cues

| ❌ Bad | ✅ Good |
|---|---|
| `15 ↓` in red = loss | `↓ 15 (loss)` with down-triangle icon |
| Kanban column color-only | Column header has icon (backlog=clock, executing=bolt, done=check) |

---

## 4. Focus management

| Rule | Spec |
|---|---|
| Focus ring must be visible on all interactive elements | `:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px }` |
| Never `outline: none` without a replacement | If you hide the ring, draw your own with box-shadow or border |
| Focus ring survives on dark + light surfaces | Use token `--focus-ring` (blue hsl(220 80% 60%) in Default, bright yellow `#FFDD00` in High-Contrast) |
| Focus ring is thicker in high-contrast | Default: 2px solid. High-contrast: 3px solid + 2px offset |
| Tab order follows visual order | DOM order matches reading order; avoid `tabindex > 0` |
| Modal traps focus | Radix/shadcn Dialog handles this; don't disable |
| Focus returns on close | Return focus to the element that opened the modal |
| Skip links | `<a href="#main">Skip to content</a>` available via keyboard on every top-level view |

---

## 5. Keyboard contract

| Interaction | Keys |
|---|---|
| Activate button | `Enter` or `Space` |
| Submit form / send message | `Enter` (or `Cmd/Ctrl+Enter` when a textarea) |
| Cancel / close dialog | `Escape` |
| Navigate list / tabs | `Arrow` keys (implemented via Radix primitives) |
| Select option in dropdown | `Arrow` + `Enter` |
| Toggle accordion / collapsible | `Enter` or `Space` |
| Tab chord | `Cmd+[` / `Cmd+]` for app-level tabs (if applicable) |

**Rule:** Every clickable `div` must become a `<button>` or have `role="button"` + `tabIndex={0}` + onKeyDown handler that maps `Enter`/`Space` to `onClick`. Grep for `role="button"` without accompanying keyboard handler = bug.

---

## 6. Screen reader contract

| Element | Required attribute |
|---|---|
| Icon-only button | `aria-label="..."` describing the action |
| Decorative icon | `aria-hidden="true"` |
| Loading/busy state | `aria-busy="true"` on the container |
| Dynamic content updates | Wrap region in `role="status"` or `aria-live="polite"`; errors get `aria-live="assertive"` |
| Form input | `<label>` linked via `htmlFor` OR `aria-labelledby` OR `aria-label` |
| Error text on field | `aria-describedby="<error-id>"` + `aria-invalid="true"` |
| Disabled element | `disabled` attr (not just visual) |
| Expanded state | `aria-expanded` + `aria-controls` |
| Selected state | `aria-selected` (in listbox/tab) |
| Tooltip | Anchor has `aria-describedby` pointing at the tooltip content |

**Announcements for streaming chat:**
- Agent responses: polite live region with throttled chunks (avoid per-token announce)
- User message send: polite "Message sent"
- Agent-run-completed: polite "Agent finished"
- Errors: assertive

---

## 7. Motion and animation

| Rule | Spec |
|---|---|
| Respect `prefers-reduced-motion: reduce` | Set `animation-duration: 0.01ms !important; transition-duration: 0.01ms !important` on the universal selector inside the media query |
| No auto-playing motion > 5s | Spinners/streaming indicators are OK if < 5s or respond to reduce-motion |
| No parallax | Period |
| No flashing > 3 Hz | Seizure safety |
| Animation max 400ms | Faster feels snappy, slower feels laggy |

Implementation: keep this block in `globals.css`:

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

---

## 8. Forms

| Rule | Spec |
|---|---|
| Every input has a visible label | No placeholder-as-label. Placeholder is example text, not identity |
| Error messages are associated | `aria-describedby` linking input to error text |
| Required fields marked | Both `required` attr and visible `*` marker with `aria-label="required"` on the asterisk |
| Validation non-blocking | Don't prevent submission on mere warnings; prevent on errors with an announcement |
| Password field: reveal toggle | Button with `aria-label="Show password"` / `aria-label="Hide password"` |
| Switch vs checkbox | Switch = immediate effect (saves on toggle). Checkbox = form state (saves on submit). Pick correctly |

---

## 9. Language and copy

| Rule | Spec |
|---|---|
| Set `lang` on `<html>` | `<html lang="en">` |
| Use plain language | Short sentences, no jargon in user-facing errors |
| Icon-only labels use verbs | `aria-label="Delete task"` not `aria-label="Trash icon"` |
| Avoid idioms for global users | Even single-locale apps benefit from literal copy |
| Error messages actionable | Tell the user what to do, not just what happened |

---

## 10. Testing requirements

| Check | Frequency | Tool |
|---|---|---|
| Automated a11y lint | On every PR | `eslint-plugin-jsx-a11y` (already in ESLint config — enforce it) |
| Axe-core unit tests | Per-component | `@axe-core/react` inside Vitest for interactive components |
| Manual keyboard sweep | Per release | Tab through every screen, verify no trap, verify every action reachable |
| Screen reader sweep | Per release | VoiceOver (macOS) on at least 1 top-level view per PR that changes UX |
| Contrast check | On theme change | WebAIM or design tool. Document in theme file |
| `prefers-reduced-motion` check | On any new animation | Chrome DevTools Rendering tab → Emulate → Reduce motion |

**CI gate:** We will add an axe-core CI check for the Chat panel and Settings dialog as the first targets (they have the most a11y surface area).

---

## 11. Settings-specific requirements

Settings is the densest a11y surface. It must:

| Requirement | Current state | Gap |
|---|---|---|
| Left-rail nav is keyboard-reachable | ✅ `role="button" tabIndex={0}` with `onKeyDown` | OK |
| Active section indicated non-color | ✅ `aria-current="page"` added + `--nav-active-bg/-text` tokens (white/dark in Default, orange/white in Light, yellow/black in HC) | OK |
| Tabs keyboard-navigable | ✅ Radix Tabs primitive handles arrows | OK |
| Collapsible lanes announce expanded state | ✅ `aria-expanded` + `aria-controls` wired | OK |
| Warning lane auto-expands | ✅ Already in place | OK |
| Status notice has text label, not color-only | ✅ `InlineNotice` always pairs icon + text label + color | OK |
| Modal focus trap | ✅ Radix Dialog | OK |
| Restart-required notice has role | ⚠ Still visually a pill without `role="status"` | **Known gap** — wrap in `<div role="status">` next a11y sweep |
| Form fields have labels | ✅ `<Label htmlFor>` wiring | OK |
| Save button state | ⚠ "Saving…" copy present; needs `aria-busy` on form | **Known gap** |

---

## 12. High-contrast toggle (shipped)

Shipped in `Settings → Preferences → Accessibility`:

- **Theme** select — Dark / Light / High contrast
- **High contrast mode** switch — shortcut that forces `data-theme="high-contrast"` and restores the last base theme (`dark` or `light`) when toggled off
- **Motion** select — Follow system / Always reduce (`data-motion="reduce"`)
- **Font size** select — Default / Large (110%) / Extra large (125%) (`data-font-scale="lg|xl"`)

Persistence:
- `localStorage.ralphx-theme` — active theme
- `localStorage.ralphx-last-base-theme` — for HC toggle-off restore
- `localStorage.ralphx-motion`
- `localStorage.ralphx-font-scale`

Applied pre-hydration by the inline script in `index.html`; re-asserted on React mount via `syncThemeAttributesFromStore()` in `main.tsx`.

Architecture: see `specs/design/theme-architecture.md`.

---

## 13. References

- WCAG 2.1: https://www.w3.org/WAI/WCAG21/quickref/
- WAI-ARIA Authoring Practices: https://www.w3.org/WAI/ARIA/apg/
- Color Blind Accessibility (Stark, Who Can Use): external
- WebAIM Contrast Checker: https://webaim.org/resources/contrastchecker/
- axe DevTools: https://www.deque.com/axe/
