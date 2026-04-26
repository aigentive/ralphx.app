> **Maintainer note:** Keep this file compact. Prefer one-line rules, links to source docs, and explicit non-negotiables over prose.

# Frontend Interaction Performance

Performance is a product requirement, not a polish pass. Applies to user-triggered transitions, especially Agents/chat panes, drawers, terminals, diff viewers, artifact panels, and other heavy surfaces.

| Rule | Detail |
|---|---|
| First paint wins (NON-NEGOTIABLE) | Click handlers must update visible shell/open/closed state before expensive work; do not wait for imports, Tauri invokes, storage writes, query hydration, diff parsing, or terminal startup before showing the UI transition. |
| Lazy heavy surfaces (NON-NEGOTIABLE) | Lazy-load heavy panes/widgets/runtimes and hydrate them after a paint boundary (`requestAnimationFrame` + macrotask/idle); shell renders first, content follows. |
| Warm-up heavy paths | Use safe intent/idle warm-ups (`onPointerEnter`, `onFocus`, viewport proximity, post-idle) for expensive modules, data, and runtimes when likely next action is clear; warm-up must be cancel-safe, deduped, and must not block the actual click transition. |
| Decouple visibility from work (NON-NEGOTIABLE) | Opening/closing panels is separate from fetching data, persisting preferences, starting processes, and mounting expensive children. |
| Defer teardown (NON-NEGOTIABLE) | Closing a heavy panel should visually close first, then unmount costly subtrees after paint; avoid expensive unmount in the same click commit. |
| Stable shells | Persisted-open panes still render a lightweight frame first so app/page hydration is not blocked by heavy module evaluation. |
| Transcript hydration (NON-NEGOTIABLE) | Existing conversations should paint chat chrome and visual message placeholders first; sorting/filtering/parsing and virtualized transcript hydration must happen after a paint boundary for every `IntegratedChatPanel` host. |
| Container-aware composition | Use container width/state for responsive control density; avoid viewport-only rules when panels can shrink inside split layouts. |
| Memoize intentionally | Split controller vs. heavy content components, pass stable callbacks/props, and memoize expensive derived state only after isolating render boundaries. |
| Opportunistic cleanup (NON-NEGOTIABLE) | When current-scope frontend work encounters a safe first-paint/lazy-hydration/decoupling opportunity, add focused coverage and fix it instead of leaving known blocking behavior behind. |
| TDD required (NON-NEGOTIABLE) | New or fixed heavy interactions need focused tests proving first-click visibility is synchronous and import/fetch/persist/process work is deferred. |
