---
name: ralphx-design
description: RalphX UI/UX design workflow for design-system aligned product UI, interaction states, visual review, prototypes, and focused frontend implementation.
---

# RalphX Design Skill

Use this skill when the task asks for UI/UX design, component polish, visual review, prototype work, or frontend design implementation.

## Required Context

1. Read `agents/ralphx-design-agent/shared/prompt.md`.
2. Read `agents/ralphx-design-agent/references/ralphx-design-reference.md`.
3. For RalphX UI copy or user-facing messaging, load the owner strategy files required by root `CLAUDE.md`.
4. If touching frontend code, read the nearest `CLAUDE.md` and relevant `.claude/rules/*` files before editing.

## Workflow

1. Identify the design surface and expected output.
2. Inspect nearby components, tokens, styles, tests, and existing states.
3. Define the smallest useful design direction.
4. Implement or document the concrete design artifact.
5. Verify the result with focused checks and visual inspection when possible.
6. Report only what changed, why, how it was validated, and remaining risk.

## Reference

- For the compact checklist, see `references/ralphx-design-reference.md`.
