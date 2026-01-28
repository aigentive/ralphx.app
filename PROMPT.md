@specs/manifest.json @specs/plan.md @logs/activity.md @logs/code-quality.md @.claude/rules/quality-improvement.md

# RalphX Build Loop

## Quick Reference

| Step | Action |
|------|--------|
| 1 | Read `specs/manifest.json` → find `"status": "active"` phase → read its PRD |
| 2 | Find first task with `"passes": false` |
| 3 | **READ FULL TASK** (Grep -C=50) — list steps |
| 4 | Execute task using appropriate workflow |
| 5 | Quality improvement: Explore agent → pick ONE fix → `refactor:` commit |
| 6 | Log to `activity.md`, set `"passes": true`, commit |
| 7 | **STOP** — one task per iteration |

---

## Step 1-2: Find Active Task

```
manifest.json → active phase → PRD file → first task where passes=false
```

**Phase complete?** (all `passes: true`) → Run **Gap Verification** (`.claude/rules/gap-verification.md`):

```
Gaps found? → Add to code-quality.md as P0 → Continue iterations
No gaps? → Update manifest, commit: `chore: complete phase N, activate phase N+1`
```

**All phases complete?** → Output `<promise>COMPLETE</promise>`

---

## Step 3: Read Full Task (CRITICAL)

**⚠️ STOP — Read the FULL task JSON before doing anything.**

```bash
Grep pattern="description.*[task words]" path="[prd]" output_mode="content" -C=50
```

**Output these fields in your response:**
- `steps`: [list each step]
- `acceptance_criteria`: [list if present]
- `design_quality`: [list if present]

| Field | Purpose |
|-------|---------|
| `description` | Summary only — NOT sufficient alone |
| `steps` | **Required actions** — follow exactly |
| `acceptance_criteria` | **Must verify** — task incomplete until all pass |
| `design_quality` | **Visual standards** — for UI tasks |

---

## Step 4: Execute Task

1. Follow task steps exactly
2. **TDD mandatory** — tests FIRST
3. Run: `npm run lint && npm run typecheck && cargo clippy && cargo test`
4. **UI tasks?** → Read `specs/DESIGN.md`, use `/frontend-design` skill
5. **Adding a TODO?** → Log it immediately to `logs/code-quality.md` (P2 for functional gaps, P3 for cleanup)

---

## Step 5: Quality Improvement (MANDATORY)

**Every task requires a `refactor:` commit. No exceptions.**

```
Pick ONE (P0 first, then by scope) → VERIFY (exists + NOT in PRD) → Execute → Mark [x]
Stale/PRD? → Strikethrough, pick next | Exhausted? → ESCALATE → Deferred Validation → Explore agent
```

**Deferred Validation:** Before Explore, validate 2-3 strikethrough items (not `excluded`). Issue exists? → Unmark. Gone? → Increment counter `(reason:N)`, archive at `:2`.

**Cleanup:** `[x]` > 10/section → move oldest to `logs/code-quality-archive.md`

**Full workflow:** `.claude/rules/quality-improvement.md` | **Skip for:** pure docs, config-only

---

## Step 6: Log & Commit

**Update `logs/activity.md`:**
- Header: task count, current task
- Entry: `### YYYY-MM-DD HH:MM:SS - [Title]` with what/commands/results

**Set `"passes": true`** in PRD

**Commit:** `git commit -m "feat|fix|docs: [description]"`

---

## Rules

- **ONE task per iteration, then STOP**
- Always log + commit
- NO: `git init`, remotes, push
- **TDD mandatory** — tests FIRST
- **Document patterns inline** — New architectural pattern? Add one-liner to `src/CLAUDE.md` or `src-tauri/CLAUDE.md`
- **Task tools for complex work** — >3 files, refactoring, >100 LOC? Use TaskCreate/TaskUpdate/TaskList

---

## Self-Improving Rules

> **Add rules here when you discover something mandatory.**
> Found a gotcha? A required pattern? A common mistake? Add a one-liner below.
> This section grows over time, making the loop smarter with each iteration.

<!-- AGENTS: Add new rules as bullet points below this comment -->
- **Verify wiring, not just existence** — Optional flags must be ENABLED at entry points (Phase 20: `useViewRegistry` never enabled)
