# High-Contrast — Border-Driven Layout Plan

**Status:** Proposal · 2026-04-19
**Scope:** HC theme only. Dark/Light untouched.
**Inspiration:** Google AI Studio (light). Every block is fenced by a thin
divider; no elevation cues; the rail, the toolbar, the content pane, and
the control panel are each visually "closed" shapes on a flat canvas.

---

## 1. Goal

Take the current HC theme — pure-black surfaces, white strokes only where we
explicitly remembered — and turn it into a fully **fenced** layout where
every meaningful region has a visible perimeter. No shadow-based elevation.
No depth-based hierarchy. Walls do all the work.

The user's brief: *"need borders to separate layout blocks across all pages
for high-contrast theme"*.

### What "done" looks like

| Today (partial) | Target (comprehensive) |
|----------------|-----------------------|
| Cards, dialogs, and popovers have stroke. | Every page-level block (rail / content / right panel / toolbar / footer) is fenced by a stroke. |
| Ghost buttons got borders in a recent pass. | Every interactive control (button, chip, tab, toggle, row) reads as bordered geometry. |
| Chat panel has stroke between header / body / composer. | Every multi-zone panel across the app has stroke between its zones. |
| Shadows are `none`. | Stays `none`. Borders remain the only elevation cue. |
| Light-scoped Settings modal ships. | Settings modal stays light-scoped — it's an exception, not the rule. |

---

## 2. Divider hierarchy (token plan)

Inspired by the reference: dividers come in **two weights**, and everything
else is either "stronger" (nav active, focus) or "softer" (disabled).

| Token | Value | Role | Example |
|-------|-------|------|---------|
| `--border-subtle` | `rgba(255,255,255,0.50)` (existing) | Default divider between **rows, sections, tabs, inline groups** | Between plan list items, between form rows, between tab contents |
| `--border-default` | `#ffffff` (existing) | **Region perimeter** — fences page-level blocks and zone transitions | Between left rail and main, between main and chat, between header and body |
| `--border-focus` | `--yellow-500` (existing) | Focus ring only | `:focus-visible` |
| *(new)* `--border-inline` | `rgba(255,255,255,0.30)` | **Inline/field chrome** — inputs, chips, toggle outlines | Text inputs, badges, status pills |

Why three weights:
- Subtle **50%** is a divider: it separates peers inside one region.
- Default **100%** is a wall: it marks where one region ends and another
  begins.
- Inline **30%** keeps micro-controls readable without every chip screaming
  for attention. Currently chips use `--border-subtle` and the rail looks
  uniformly hot. Adding a lighter token gives us hierarchy inside a region.

---

## 3. Component-level rules (NON-NEGOTIABLE)

These extend the existing HC rules (focus ring, ghost-button stroke,
nav active, count-badge). Apply globally to HC only.

### 3.1 Region perimeters (`--border-default`)

| Region | Rule |
|--------|------|
| Top app nav bar | `border-bottom: 1px` separating nav from view |
| Left rail (any page) | `border-right: 1px` separating rail from content |
| Right chat panel | `border-left: 1px` separating chat from content |
| Bottom execution bar | `border-top: 1px` separating footer from content |
| Floating overlays (graph filters, timeline) | Full `1px` perimeter |
| Dialog/modal shell | Full `1px` perimeter (already exists) |

### 3.2 Zone separators within a region (`--border-default`)

| Zone transition | Rule |
|-----------------|------|
| Panel header → body | `border-bottom` on header |
| Panel body → composer/footer | `border-top` on footer |
| Tab list → tab content | `border-bottom` on tab list |
| Dialog header → body | `border-bottom` on header (already) |
| Dialog body → footer | `border-top` on footer (already) |

### 3.3 Row/list dividers (`--border-subtle`)

| List type | Rule |
|-----------|------|
| Plan browser rows | `border-bottom` between rows (shipped) |
| Kanban task cards | Full perimeter, no shadow (mostly shipped) |
| Activity message rows | `border-bottom` between messages |
| Settings nav items | `border-bottom` between group children |
| Kanban column header / body | `border-bottom` on column header |
| Stat card grid | Full perimeter on each card, gap between |

### 3.4 Inline controls (`--border-inline`)

| Control | Rule |
|---------|------|
| Text input / textarea | `1px` full border, `--border-default` on focus |
| Select trigger | `1px` full border |
| Count/status chip | `1px` full border (replaces color-only differentiation) |
| Toggle track | `1px` border around track; thumb uses `--accent-primary` |
| Checkbox/radio unchecked | `1px` border; checked fills with `--accent-primary` |
| Badge (neutral) | `1px` border |

### 3.5 Buttons (recap, already shipped in `763c996b3` + `d1d2dfe26`)

Every `<button>` without an existing border utility gets
`1px solid --border-subtle`. Primary/active buttons get the border AND keep
their `--accent-primary` fill. The rule is global and consolidated — no
per-variant duplication.

---

## 4. Page-by-page application

Notation: `BD` = `border-default`, `BS` = `border-subtle`, `BI` = `border-inline`.

### Top-level chrome (every view)

| Element | Component | Border |
|---------|-----------|--------|
| App header (nav bar) | `layout/Navigation.tsx` | `border-bottom: BD` |
| Bottom execution bar | `execution/ExecutionControlBar.tsx` | `border-top: BD` |
| Right chat panel root | `Chat/ChatPanel.tsx`, `Chat/IntegratedChatPanel.tsx` | `border-left: BD` |

### Ideation

| Element | Border |
|---------|--------|
| Plan browser rail | `border-right: BD` |
| Plan list row | `border-bottom: BS` (shipped) |
| "New Plan" button | inherited from button rule (shipped) |
| Plan detail tabs (Plan/Verification/Proposals) | `border-bottom: BD` on tab list |
| Proposal card | `1px BS` perimeter, no shadow |
| Tier heading row | `border-bottom: BS` |
| Chat panel header | `border-bottom: BD` (shipped) |
| Chat composer | `border-top: BD` (shipped) |
| Reopen / Reset & Re-accept buttons | inherited (shipped) |

### Graph

| Element | Border |
|---------|--------|
| Graph canvas container | `border: BD` on overlay panels only (canvas stays open) |
| Floating filters panel | `1px BD` perimeter |
| Floating timeline | `1px BD` perimeter, `border-bottom: BS` between rows |
| Node group background shape | React Flow nodes keep their own stroke; HC override bumps to `BD` |

### Kanban

| Element | Border |
|---------|--------|
| Column container | `1px BS` perimeter |
| Column header | `border-bottom: BS` |
| Task card | `1px BS` perimeter (already tokenised via `--card-border`) |
| Plan selector popover trigger | inherited button rule |
| Search input | `1px BI`, `BD` on focus |
| Kanban → chat split | `border-left: BD` on chat side |

### Insights

| Element | Border |
|---------|--------|
| KPI stat card | `1px BS` perimeter, no shadow |
| Trend chart card | `1px BS` perimeter |
| Metrics detail card | `1px BS` perimeter |
| EME sticky sidebar | `border-left: BD` |
| Usage insights card | `1px BS` perimeter |

### Activity

| Element | Border |
|---------|--------|
| Filter toolbar | `border-bottom: BS` |
| Filter pill | `1px BI` |
| Message row | `border-bottom: BS` |
| Mode toggle (realtime/historical) | inherited tab rule (`BD` underline on active) |

### Extensibility

| Element | Border |
|---------|--------|
| Tab list | `border-bottom: BD` |
| Tab panel card (Workflow/Artifact/Research/Methodology) | `1px BS` perimeter |
| Card action button | inherited |

### Settings (light-scoped exception)

Stays on the **inverted light palette** committed in `d1d2dfe26` /
`61c12790f`. Inside the modal, treat borders the same as Light:
`border-subtle = gray-400`, `border-default = gray-500`. The HC rules above
do not apply to descendants of `[data-testid="settings-dialog"]` because
the scoped token overrides cascade.

Exception justification: Settings is information-dense, form-heavy, and was
unreadable as pure-black-on-black. The user accepted this exception on
2026-04-19.

---

## 5. Shadow policy

| Surface | HC shadow |
|---------|-----------|
| Everything | `none` (shipped) |

Reaffirmed. Any component still reading `--shadow-*` on HC gets `none` and
falls back to its border for elevation. If a component renders as a flat
black rectangle after this plan lands, the fix is to add a border — never
to reintroduce a shadow.

---

## 6. Implementation phases

Ordered so each phase ships a visible improvement and leaves the app in a
shippable state.

### Phase 1 — Region perimeters (foundation)

1. Add `--border-inline: rgba(255,255,255,0.30)` to
   `styles/themes/high-contrast.css` (and a `:root` default that maps to
   `--border-subtle` for Dark/Light so non-HC reads a sane value).
2. Add HC-only rules:
   - `Navigation` root → `border-bottom: BD`
   - `ExecutionControlBar` root → `border-top: BD`
   - Right-chat root (`ChatPanel`, `IntegratedChatPanel`) →
     `border-left: BD`
3. Left rail of Ideation → `border-right: BD` (Kanban rail is the board
   itself — skip).

Ship & screenshot. Confirm each page now has a clearly fenced outer frame.

### Phase 2 — Zone separators in multi-zone panels

4. Tab lists across the app get `border-bottom: BD` on HC. Source of truth:
   `ui/tabs.tsx`. Add a scoped rule so every Radix `[role="tablist"]` picks
   it up.
5. Panel header/footer separators — extend the chat rule (shipped) to
   Ideation tabs, Activity filter bar, Insights KPI grouping strip.

Ship & screenshot.

### Phase 3 — Row/list dividers

6. Activity message rows get `border-bottom: BS`.
7. Settings nav items get `border-bottom: BS` between group children (Only
   inside the light-scoped modal these pick up gray-400 — still reads as a
   list).
8. Kanban column header → body `border-bottom: BS`.

Ship & screenshot.

### Phase 4 — Inline control chrome

9. Inputs / selects / textareas get `1px BI` default, `BD` on focus
   (extend `ui/input.tsx`, `ui/textarea.tsx`, `ui/select.tsx` with an HC
   override rule, not per-component changes).
10. Count/status chips — extend the existing HC count-zero rule to all
    count states: chip always has a border, color changes by state.
11. Toggle / checkbox / radio — add `1px BI` around the track / box even
    when unchecked. Radix primitives expose `data-state`.

Ship & screenshot.

### Phase 5 — Audit & close gaps

12. Run Playwright theme-switch-audit across all 6 pages in HC.
13. Diff against the reference: every page should show a clear "outer
    frame + inner dividers + inline chrome" structure.
14. Fix any block that still reads as a loose cloud of text on black.

---

## 7. Risk register

| Risk | Mitigation |
|------|------------|
| Double-stroke when inline border rules stack with existing component borders. | Use `:not([class*="border"])` guards (already the pattern for buttons). |
| Over-bordered look making HC feel busy. | Three-tier token (`BD` / `BS` / `BI`) gives hierarchy. Audit step in Phase 5 catches over-stroke. |
| Inline styles with `border: none` (like TextBubble) bypass CSS. | Use `!important` in scoped HC rules (already done for TextBubble). Document in the rule comment. |
| CSS selector drift when testids change. | Prefer role/aria selectors (`[role="tablist"]`, `[role="button"]`) over testids where possible — they track semantics, not names. |
| Reference-Light aesthetic creeping back into HC. | Keep the divider token values in one place (§2 table). If someone proposes changing them, require a spec update. |

---

## 8. Verification checklist

Before closing this plan:

- [ ] Every page from §4 visually shows an outer fence on HC.
- [ ] Every multi-zone panel shows stroke between header / body / footer.
- [ ] Every list shows row dividers on HC.
- [ ] Every input and chip has a visible perimeter at rest.
- [ ] Focus ring stays yellow and stays 3px (HC rule §3 focus).
- [ ] Settings dialog keeps light palette — no regression into HC black.
- [ ] `npm run test:run -- src/styles` green.
- [ ] `./scripts/check-design-tokens.sh` all 7 guards green.
- [ ] Playwright theme-switch audit captures cleanly across 6 pages × HC.

---

## 9. References

- Spec: `specs/design/themes/high-contrast.md`
- Companion: `specs/design/light-hc-polish.md`
- Architecture: `specs/design/theme-architecture.md`
- Chat audit: `specs/design/chat-panel-audit.md`
- Accessibility: `specs/design/accessibility.md`
- Inspiration: Google AI Studio (light theme) — form-dense layout with
  consistent divider weight across sections.
