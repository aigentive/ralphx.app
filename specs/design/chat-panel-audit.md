# Right-side chat panel audit — 2026-04-19

Fresh audit of the three right-side chat surfaces across all 3 themes, sampled from screenshots in `.artifacts/theme-audit/{dark,light,high-contrast}/`. Pixel samples taken at viewport 1280x720.

## Summary

- **Heights:** Kanban right chat and task-detail right chat fill their allocated height cleanly (glass panel top y=64 down to y=711, matching the ExecutionControlBar stripe). **Chat overlay** (floating `ChatPanel` used on Extensibility/Activity/Reviews/Settings/Graph-no-task) stops at `bottom: 76px` — wastes ~76px at the bottom when no ExecutionControlBar is rendered in those views (empty strip from y=636 to y=720). Graph view with no task selected shows only the collapsed 40px rail and cannot be analyzed further.
- **Shades:** Kanban/task-detail IntegratedChatPanel uses `withAlpha("var(--bg-surface)", 92)` + `backdrop-filter: blur(20px) saturate(180%)` for the body, and a *different* token (`withAlpha("var(--bg-base)", 50)`) for the header AND composer chrome rows. This creates **three distinct tonal bands** (body lum=30, chrome lum=25, app lum=20) inside a single glass container. ChatPanel (the floating overlay) uses yet another set of tokens (`bg-elevated` outer, `color-mix(text-primary 2%)` header, `border-subtle` composer top) — the two panels do not share a visual language. In high-contrast, the overlay panel's sibling elements (chrome toggle buttons, active tab indicators) paint at pure white/full accent while the panel interior is flat black — visually jarring.
- **Spacing:** Kanban composer has **clean 12px + 12px** padding around the input field (`p-3` wrapper). Header is h-11 (44px) + 1px borders. But the composer chrome band (queued+input) on empty-state panels only shows the 12px-wrapped input with no queue, producing a ~60px chrome band vs 44px header — asymmetric. Also, the outer IntegratedChatPanel has `padding: 8px` on all sides (producing a floating glass look) but the header/body/composer borders land flush against the glass panel's rounded corners, so the inner top-padding region is completely invisible against the surrounding app bg (see dark scan: y=56-63 at lum=20 = bg-base, no visible offset because the glass edge blends into it). The 8px padding only becomes perceptible via the luminous border (1px `overlay-weak`) and the glass container's 10px radius.
- **Layout:** Kanban right panel fills its `shrink-0` column cleanly (x=859..1279). No horizontal overflow. Resize handle is a 12px hit area with 2px visible line at `color-mix(text-primary 15%)` — **does not** match `--border-subtle`. Floating ChatPanel is `position: fixed` with `bottom: 76px`, which hardcodes the ExecutionControlBar height — it desyncs on views that don't render the bar.

## Per-view findings

### Kanban right chat (Project chat, empty state)

**Dark:**
- App bg (lum=20, bg-base hsl=gray-975). Panel inner glass body lum=30 (bg-surface hsl=gray-950 at 92% alpha ≈ 28–30). Header/composer chrome rows lum=25 (bg-base at 50% over glass body).
- Vertical map at x=1000:
  - y=55: app-header bottom border (border-subtle, lum=45)
  - y=56-63: 8px top padding = bg-base (lum=20) ✅
  - y=64: glass top border (overlay-weak, lum=42)
  - y=65-107: header row (lum=25, height=43px ≈ h-11 44px) ✅
  - y=108: header bottom border (overlay-faint, lum=33)
  - y=109-647: body (lum=30)
  - y=648: composer top border (overlay-faint, lum=33)
  - y=649-660: composer top padding (lum=25, 12px)
  - y=661: input field top border (bg-hover, lum=50)
  - y=662-697: input field interior (lum=30)
  - y=698: input field bottom border (lum=50)
  - y=699-710: composer bottom padding (lum=25, 12px)
  - y=711: glass bottom border (lum=42)
- **Issues:**
  - The body is a bg-surface shade (lum=30) but the header+composer are a *lower* tone (lum=25) that is visually noticeably darker than the body. When empty, this produces the "three-tier" appearance: dark app (20) → glass dark chrome (25) → glass body (30). On dark theme this reads as a subtle sandwich. `ContextIndicator` uses `text-muted` (~0.55 alpha) for the icon, which is fine, but the lum=25 vs lum=30 seam is crisp enough to see.
  - `withAlpha("var(--bg-base)", 50)` on the header means the header shade is viewport-bg-dependent: light theme lum=247, dark=25, HC=0 (pure black). On HC this lands at pure black while the body is also pure black (both bg-base and bg-surface = black), so the header+composer bottom borders are the only thing visible — which is correct for HC but dissolves the visual hierarchy entirely on that theme.

**Light:**
- bg-base lum=252, bg-surface lum=242 (near-white). The panel inner body lum=248, header/composer lum=250, input field lum=247.
- The tone differences in light are 2-5 lum points — almost indistinguishable. The header "shade" is a visible band because the underlying glass body is ~1% different, and the light-theme overlay tokens (`overlay-faint` = black 3%, `overlay-weak` = black 5%) produce a faint line between header and body.
- **Issues:**
  - The Send button background at `--bg-hover` lum=227 on light (lum=243 with blur) looks almost identical to the input field lum=247 — only 4 levels of contrast. When disabled, this button is **nearly invisible** on light theme (confirmed: send btn in light kanban is 243,243,245 while the composer around it is 247,247,248 — 4 pt delta).
  - The disabled send icon uses `--text-muted` on a bg 4pt different from the chrome — poor affordance.

**High-contrast:**
- All bg-* tokens collapse to pure black (0,0,0) on HC. The glass blur effect is pure black-on-black, so the entire chat panel body shows as a flat black rectangle with only the `border-subtle` (50% white) perimeter visible.
- The border delineates the panel adequately — the "frame" is still legible.
- **Issues:**
  - HC empty state: the accent-muted background on the empty-state icon box (`--bg-hover` lum=38 in HC) is a mid-gray square on black — looks OK but the chat bubble icon inside uses `--text-muted` → low contrast against the gray square. The icon reads but is muted.
  - Send button uses `--bg-hover` (HC lum=38) with `--text-muted` (low) — disabled state is basically a gray box with no visible icon (see HC kanban.png: send btn is 15,15,15 — nearly invisible).

### Graph right chat
- `GraphSplitLayout` only renders IntegratedChatPanel when a task is selected. Captured screenshots have no task selected, so the right side shows only a 40px collapsed rail (`SeparatorLine`). **Not testable** from current screenshots. Code path uses the same IntegratedChatPanel so findings transfer.

### Chat overlay (floating `ChatPanel` on Extensibility/Activity/Reviews/Settings)

**Dark:**
- Uses `ResizeablePanel` = `position: fixed; top: 56px; right: 0; bottom: 76px` with inner container `margin: 8px; background: var(--bg-elevated); border: 1px solid var(--border-subtle); boxShadow: var(--shadow-md); borderRadius: 10px`.
- Vertical map at x=1050 (in dark chat.png):
  - y=56-63: 8px of fixed-aside bg-elevated (lum=36–37)
  - y=64: glass container top border (lum=42)
  - y=65-103: header (lum=40, bg = `color-mix(text-primary 2%, transparent)` ≈ lum=40)
  - y=104: header bottom border (lum=45, `color-mix(text-primary 4%)`)
  - y=105-579: body (lum=37, bg-elevated)
  - y=580-587: body-to-composer transition (lum=30–37)
  - y=588-627: composer (lum=28-30 which is bg-base darker than panel — **inconsistent**, body is 37, composer area is 28-30)
  - y=628: glass container bottom border
  - y=636-720: BELOW the fixed panel (`bottom: 76px`) — wasted empty strip (~84px high, black with slight vignette)
- **Issues:**
  - The composer area (lum=28-30) is DARKER than the body (lum=37) because composer uses `var(--border-subtle)` top and the floor of the composer is the aside bg lum=36 but the chat input field inside uses `var(--bg-surface)` lum=30 — the composer top border `color: var(--border-subtle)` is visible but the floor transition from lum=37 (body) → lum=30 (composer chrome) is a visible step.
  - The `bottom: 76px` hardcoded in ResizeablePanel creates **~84px of empty space** between the bottom of the chat panel and the viewport bottom on views that don't have an ExecutionControlBar (Extensibility, Activity, Reviews, Settings). Visible as dark void.
  - Panel `background: var(--bg-elevated)` on the outer aside vs inner `background: var(--bg-elevated)` on the rounded container — *same token* — which makes the 8px margin look pointless (can only see the subtle shadow).

**Light:**
- Similar structure. Body lum=255 (pure white bg-elevated). Header lum=250. Composer area lum=247. Input field lum=247 with a visible lum=212 border.
- Visual tonality is slightly gradient-like (pure white body → 250 header → 247 composer) — cleaner than dark but still three-tier.
- **Issues:**
  - Same `bottom: 76px` wasted space — visible as ~84px of bg-base (lum=250) strip below the panel on views without the execution bar. Visible in light chat.png at y=644-716 (lum ≈ 248, bg-base, with slight warm cast).
  - Chat input field has a 1px border at lum=212 (≈ gray-300) which is prominent on light — actually legible and good.

**High-contrast:**
- Panel outer and inner both pure black. Border `--border-subtle` = white 50% alpha — **visible outline**.
- Header `color-mix(text-primary 2%, transparent)` = ~2% of white over black = ~5/255 — essentially invisible → header has **no visible band** to distinguish from body.
- Composer has `borderColor: var(--border-subtle)` top which IS visible on HC (50% white). Combined with no header band, the HC overlay looks like a single-line outlined rectangle with a chat input pinned to the bottom.
- **Issues:**
  - Bright white focus rings around the "Workflows" tab (left side), PanelRightClose icon, and X close button in chat header (see HC chat.png) — these are keyboard focus indicators painting on top of the dark chrome. On HC theme this is intentional for a11y but in an empty chat state it reads as "weird shades" because the buttons appear to have different backgrounds than the panel header they sit in.
  - Bottom-of-viewport empty strip is pure black (bottom: 76px wasted region) — less visually jarring on HC than dark/light since everything is black anyway.

### Task detail right chat

- **Identical render path to Kanban** (both use IntegratedChatPanel in KanbanSplitLayout). The only differences: context label = "Task" (vs "Project"), placeholder = "Ask about this task..." (vs "Send a message..."). Pixel sampling confirms the same three-tier shade structure.
- Header icon is `CheckSquare` vs `FolderKanban` — no layout impact.
- **No unique issues** beyond those listed under Kanban.

## Cross-theme patterns

| Pattern | Dark | Light | HC | Risk |
|---|---|---|---|---|
| Header uses `bg-base@50%` over glass | lum=25 vs body lum=30 — visible band | lum=250 vs body lum=248 — faint band | both pure black — band invisible | Inconsistent tier visibility |
| Composer chrome uses `bg-base@50%` | lum=25 — matches header, visible | lum=247 — faint | pure black | Inconsistent tier visibility |
| IntegratedChatPanel outer padding 8px | Invisible — parent bg matches | Invisible — parent bg matches | Invisible — parent bg matches | Padding serves only rounded corner + shadow; consider removing |
| `ChatPanel` outer `bottom: 76px` | ~84px empty strip below panel | same | same | Hardcoded; views without footer show void |
| `ChatPanel` outer = inner bg token | Both `bg-elevated` — 8px margin pointless | Both white — margin pointless | Both black — margin visible only via border | Margin exists for shadow only |
| Two chat panels use different chrome tokens | IntegratedChat: `bg-base@50%` chrome / `bg-surface@92%` body; ChatPanel: `text-primary@2%` chrome / `bg-elevated` body | Same tokens diverge per theme | Same | Two chat UIs do NOT share design language |
| Send button disabled state uses `bg-hover` | Visible (lum=31 on body 30) — subtle | Nearly invisible (lum=243 on 247) — 4pt delta | Nearly invisible (lum=38 on 0) | Disabled affordance broken on light+HC |
| Resize handle uses `text-primary@15%` not `border-subtle` | Visible | Visible | Visible | Adds a fourth border-ish token; inconsistent |

## Priority fix list

| # | Theme(s) | What | Where | Fix | Effort |
|---|---|---|---|---|---|
| 1 | all | Floating `ChatPanel` bottom gap: ~84px wasted below panel on views without ExecutionControlBar | `frontend/src/components/Chat/ResizeablePanel.tsx:43` (`bottom: "76px"`) | Make `bottom` a prop driven by whether the current view renders the footer; default to `0` and set `76` only when execution bar is visible | S |
| 2 | all | Two chat panels use diverging chrome tokens (body vs chrome mismatch patterns) | `IntegratedChatPanel.tsx:873, 1058` + `ChatPanel.tsx:500-502, 645` | Unify: pick one pattern — either both use `bg-base@50%` over glass, or both use `color-mix(text-primary, 2%)`. Recommend the `text-primary@2%` pattern since it is theme-agnostic (always reads as a subtle lighter band). | M |
| 3 | light, HC | Disabled Send button nearly invisible (4pt delta on light, 38 on 0 on HC) | `ChatInput.tsx:379-383` | Increase disabled bg shade: use `color-mix(text-primary 8%, bg-surface)` instead of `bg-hover`; or add a visible `border` when disabled | S |
| 4 | HC | Empty panel tiers collapse to single black rectangle | HC theme + panel design | Add a subtle `border-subtle` (50% white) between header/body and composer/body — give HC users a visible chrome line. Currently only top+bottom overlay-faint renders, which is 6% white on HC = ~15 lum — visible but thin. Bump to `border-subtle` specifically on HC for body-chrome seams. | S |
| 5 | all | IntegratedChatPanel 8px outer padding invisible (glass edge lands flush with app bg) | `IntegratedChatPanel.tsx:852` | Either (a) remove the 8px padding entirely and let the glass panel extend to the column edges (consistent with how sidebars render), or (b) give the outer container a subtle `bg-base → transparent` gradient so the padding is visible. Recommend removing — the luminous border already delineates. | S |
| 6 | all | ChatPanel floating overlay: outer aside and inner rounded container both use `bg-elevated` — 8px margin purpose unclear | `ResizeablePanel.tsx:44, 52` | Either (a) remove the 8px margin and the nested rounded container, or (b) change outer to `transparent` so margin shows parent bg. | S |
| 7 | dark | Three-tier shade sandwich visible (app 20 → chrome 25 → body 30) is unintentional | `IntegratedChatPanel.tsx:873, 1058` | Unify: body and chrome should use the same base shade with the border as the only delimiter. Use `var(--bg-surface)` for everything, with `border-subtle` or `overlay-weak` horizontal dividers. | M |
| 8 | all | Resize handle line uses `color-mix(text-primary 15%)` rather than `--border-subtle` | `ui/ResizeHandle.tsx:48` | Replace with `bg-[var(--border-subtle)]` for tone consistency with the column divider | XS |
| 9 | all | Composer chrome asymmetric: header h-11 (44px) vs composer empty-state chrome ~60px | `IntegratedChatPanel.tsx:1087` (`p-3`) | Reduce composer wrapper to `py-2 px-3` when no queued messages/questions are present | S |
| 10 | HC | Focus rings on chrome buttons (PanelRight/X) paint bright white over chrome — visually jarring in empty state | HC theme global focus | Already a11y-intentional. No-op — document expected behavior in styleguide. | — |
| 11 | all | `withAlpha("var(--bg-base)", 50)` for chrome rows is theme-viewport-dependent — HC collapses to solid black identical to body | `IntegratedChatPanel.tsx:873, 1058` | Replace with `withAlpha("var(--overlay-weak)", 100)` so it is always an overlay over the glass surface, not a lerp back to the root canvas | S |
| 12 | all | ChatPanel (overlay) composer bg: `border-t var(--border-subtle)` only — no surface change, but input field inside uses `bg-surface` (lum=30 dark) while the composer wrapper is `bg-elevated` (lum=37 dark) → visible floor step | `ChatPanel.tsx:645-646` | Give the composer div the same `bg-elevated` with a `border-t` for consistency with body. Currently the composer wrapper is transparent + the contained input is bg-surface, producing the tonal step. | S |

## Source references

- `frontend/src/components/Chat/IntegratedChatPanel.tsx:841-1119` — panel JSX (outer 8px padding, header `bg-base@50`, composer `bg-base@50`)
- `frontend/src/components/Chat/ChatPanel.tsx:485-700` — overlay panel (uses ResizeablePanel + `color-mix(text-primary, 2%)` header)
- `frontend/src/components/Chat/ResizeablePanel.tsx:18-62` — fixed positioning with `bottom: 76px`
- `frontend/src/components/Chat/ChatInput.tsx:293-400` — composer (12px p-3 wrapper, send button)
- `frontend/src/components/layout/KanbanSplitLayout.tsx:94-156` — Kanban split with right chat column
- `frontend/src/components/layout/GraphSplitLayout.tsx:166-266` — Graph split (fixed timeline or IntegratedChatPanel)
- `frontend/src/components/ui/ResizeHandle.tsx:32-53` — resize handle uses `text-primary@15%`
- `frontend/src/styles/themes/{dark,light,high-contrast}.css` — token definitions (bg-base, bg-surface, bg-elevated, border-subtle, overlay-faint, overlay-weak)
- `frontend/src/styles/tokens/semantic.css:23-27, 91-92` — token hierarchy definitions
