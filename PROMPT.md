@specs/manifest.json @specs/plan.md @logs/activity.md @logs/code-quality.md

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

**Phase complete?** (all `passes: true`) → Run **Gap Verification** before transitioning:

```
1. Read entire PRD + its referenced plan/specs
2. Launch Explore agent: "Verify implementation against PRD. Find gaps, missed TODOs, incomplete features"
3. Gaps found? → Add to logs/code-quality.md as P0 (Critical) → Continue to next iteration
4. No gaps? → Update manifest, log, commit: `chore: complete phase N, activate phase N+1`
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

---

## Step 5: Quality Improvement (MANDATORY)

**Every task requires a `refactor:` commit. No exceptions.**

```
Read logs/code-quality.md → Pick ONE (P0 first, then by scope) → VERIFY (exists + NOT in PRD) → Execute → Mark [x]
Stale/PRD? → Strikethrough, pick next | Scope exhausted? → ESCALATE (P3→P2→P1) | ALL exhausted? → Explore agent
```

**NO SKIPPING. "Nothing to do" is NOT valid.** Escalate scope or replenish with Explore agent.

**Full workflow:** `.claude/rules/quality-improvement.md`

**Skip only for:** pure docs, config-only changes

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
- **Self-improving loop** — Found something mandatory? Add one-liner to this PROMPT.md file
