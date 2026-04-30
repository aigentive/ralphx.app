# RalphX Design Agent Reference

Use this reference when a UI/UX task needs RalphX-specific design grounding.

## Required RalphX Sources

- `CLAUDE.md`: owner strategy loading, design system pointer, warm-orange accent, SF Pro preference, and frontend rules.
- `AGENTS.md`: Codex-facing repo rules and frontend verification constraints.
- `frontend/src/CLAUDE.md`: frontend path rules when touching React code.
- `specs/design/styleguide.md`: tokens, component behavior, and layout rules.
- `specs/DESIGN.md`: current product design direction.
- `.claude/rules/icon-only-buttons.md`: accessible tooltip requirement.
- `.claude/rules/frontend-interaction-performance.md`: first-paint and interaction responsiveness.
- `.claude/rules/wkwebview-css-vars.md`: literal theme-token rules for Tauri WKWebView.

## Design Review Checklist

- The UI matches surrounding density, spacing, type scale, and component anatomy.
- The flow includes loading, empty, error, disabled, permission, and recovery states where relevant.
- Primary and secondary actions are visually clear and placed where the user expects them.
- Icons come from the established icon system when available.
- Icon-only buttons have accessible labels and tooltips.
- Text does not overflow or overlap at desktop and mobile widths.
- Color usage is purposeful, contrast-safe, and not dominated by purple/blue palettes.
- State changes are visible without blocking first paint.
- Visual artifacts use project-local assets or explicit placeholders.
- Verification evidence is concrete: screenshots, focused tests, build output, or manual viewport notes.

## Output Patterns

For a design plan:

```markdown
## Direction
- ...

## Flow
- ...

## Components
- ...

## States
- ...

## Verification
- ...
```

For a design review:

```markdown
## Findings
- [Impact] Surface: issue, evidence, recommendation.

## Open Questions
- ...

## Next Action
- ...
```

For implementation:

```markdown
Changed:
- path: what changed

Validation:
- command or inspection result

Remaining risk:
- ...
```
