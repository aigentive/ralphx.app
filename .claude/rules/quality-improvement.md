# Quality Improvement Loop

## Overview

Every code task requires a `refactor:` commit. Use `logs/code-quality.md` to track issues.

## Workflow

```
1. Read logs/code-quality.md
2. Items exist? → Pick ONE by scope
3. VERIFY:
   a. Issue still exists? (read file:line, check if problem remains)
   b. NOT in active PRD? (cross-reference with current phase task list)
   ├── Valid & not in PRD? → Execute → Mark [x] → Commit
   ├── Stale (already fixed)? → Strikethrough ~~text~~ (stale) → Pick next
   └── In PRD (planned work)? → Strikethrough ~~text~~ (PRD) → Pick next
4. List empty/all marked? → Launch Explore agent → Update file → Pick ONE → Verify → Execute
```

**Why verify?**
- Other tasks or parallel agents may have already fixed the issue
- Some issues may be planned work in the active PRD — don't duplicate effort

## Priority & Scope Matching

| Priority | When | Pick Order |
|----------|------|------------|
| **P0 - Critical** | Gaps from phase verification | **ALWAYS FIRST** (any task size) |
| P1 - High | Architecture, major refactors | Large tasks (>150 LOC) |
| P2 - Medium | Error handling, extraction | Medium tasks (50-150 LOC) |
| P3 - Low | Lint, naming, cleanup | Small tasks (<50 LOC) |

**P0 items are picked before any P1/P2/P3 regardless of task size.**

## Quality Targets

### Frontend (src/)
- Replace `any` with proper types
- Fix naming inconsistencies
- Add missing error handling
- Remove dead code
- Extract repeated logic into hooks/functions
- Fix lint warnings

### Backend (src-tauri/)
- Fix clippy warnings
- Improve error handling (domain-specific variants)
- Fix naming inconsistencies
- Remove dead code
- Extract repeated logic into helpers

## Explore Agent Prompt

When `logs/code-quality.md` is empty, use this prompt:

```
Scan src/ and src-tauri/ for code quality issues. Find:
- Type safety issues (any types, missing error handling)
- Dead code, unused imports
- Naming inconsistencies
- Clippy/lint warnings
- Extraction opportunities (repeated logic, large functions)

Output as markdown checklist:
- [ ] [P1/P2/P3] [Frontend/Backend] Description - file:line

Group by: Frontend P1, P2, P3 then Backend P1, P2, P3
```

## File Format: logs/code-quality.md

```markdown
## Frontend (src/)

### P1 - High Impact
- [ ] Replace `any` in useChat hook - src/hooks/useChat.ts:45

### P2 - Medium Impact
- [ ] Extract validation logic - src/components/Form.tsx:120-150

### P3 - Low Impact
- [ ] Fix unused import - src/utils/helpers.ts:3

## Backend (src-tauri/)
[same structure]

## Last Explored
**Date:** YYYY-MM-DD HH:MM
**Areas:** src/, src-tauri/
```

## Skip Conditions

Quality improvement NOT required for:
- Pure documentation changes
- Configuration-only changes

## Verification

Task is NOT complete until:
1. `refactor:` commit exists in git log
2. Completed item marked `[x]` in `logs/code-quality.md`
