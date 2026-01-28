@specs/manifest.json @specs/plan.md @logs/activity.md @logs/code-quality.md

# RalphX Build Loop

## Quick Reference

| Step | Action |
|------|--------|
| 1 | Read `specs/manifest.json` → find `"status": "active"` phase → read its PRD |
| 2 | Find first task with `"passes": false` |
| 3 | **READ FULL TASK** (Grep -C=50) — list steps, acceptance_criteria, design_quality |
| 4 | Execute task using appropriate workflow |
| 5 | Quality improvement: Explore agent → pick ONE fix → `refactor:` commit |
| 6 | Log to `activity.md`, set `"passes": true`, commit |
| 7 | **STOP** — one task per iteration |

---

## Step 1-2: Find Active Task

```
manifest.json → active phase → PRD file → first task where passes=false
```

**Phase complete?** (all `passes: true`) → Update manifest, log, commit: `chore: complete phase N, activate phase N+1`
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

## Step 4: Execute by Category

| Category | Workflow | Pre-read |
|----------|----------|----------|
| `planning` | PRD Generation | `specs/plan.md` |
| `design*` | Design Workflow | `specs/DESIGN_OVERHAUL_PLAN.md`, `specs/DESIGN.md` |
| other | Implementation | — |

### PRD Generation (planning)
1. Read `specs/plan.md` sections for this phase
2. Create PRD at `output` path with: Overview, Dependencies, Scope, Requirements, Tasks
3. Preserve ALL details — don't summarize
4. Tasks: atomic, TDD, clear acceptance criteria

### Design Workflow (design, design-req, design-doc)
1. Read `specs/DESIGN_OVERHAUL_PLAN.md` + `specs/DESIGN.md`
2. Use `/frontend-design` skill
3. **Anti-AI-Slop checklist:**
   - ❌ NO purple/blue, NO Inter, NO flat surfaces
   - ✅ Warm orange `#ff6b35`, SF Pro, layered shadows, micro-interactions

### Implementation Workflow
1. Follow task steps exactly
2. **TDD mandatory** — tests FIRST
3. Run: `npm run lint && npm run typecheck && cargo clippy && cargo test`

---

## Step 5: Quality Improvement (MANDATORY)

**Every task requires a `refactor:` commit. No exceptions.**

### Read First: `logs/code-quality.md`

This file tracks known quality issues. Check it BEFORE launching any agent.

### Workflow

```
Read logs/code-quality.md
  ├── Has unchecked items? → Pick ONE (by task scope) → Execute → Mark [x] → Commit
  └── All done or empty? → Launch Explore agent → Update file → Pick ONE → Execute → Commit
```

### Scope Matching

| Task Size | Pick From |
|-----------|-----------|
| Small (<50 LOC) | P3 (low impact) |
| Medium (50-150 LOC) | P2 (medium impact) |
| Large (>150 LOC) | P1 (high impact) |

### Explore Agent Prompt (when list empty)

```
Scan src/ and src-tauri/ for code quality issues. Find:
- Type safety (any types, missing error handling)
- Dead code, unused imports
- Naming inconsistencies
- Clippy/lint warnings
- Extraction opportunities (repeated logic, large functions)

Output as markdown checklist grouped by: Frontend P1/P2/P3, Backend P1/P2/P3
```

### Commit

```bash
git commit -m "refactor: [description]"
```

**Skip only for:** pure docs changes, config-only changes

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
- Planning: preserve ALL master plan details
- Implementation: tests FIRST
