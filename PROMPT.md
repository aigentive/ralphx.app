@specs/manifest.json @specs/plan.md @logs/activity.md

We are building the RalphX project. The master plan is in `specs/plan.md`.

## Step 1: Determine Active PRD

Read `specs/manifest.json` to find the current phase:
- Find the phase where `"status": "active"`
- The `"prd"` field contains the path to the active PRD file
- Read that PRD file

## Step 2: Check for Remaining Tasks

In the active PRD, look for tasks with `"passes": false`.

**If tasks remain** → Continue to Step 3

**If ALL tasks have `"passes": true`** → Handle Phase Transition:
1. Update `specs/manifest.json`:
   - Set current phase's `"status"` to `"complete"`
   - Set `"currentPhase"` to next phase number
   - Set next phase's `"status"` to `"active"`
2. Update the `logs/activity.md` header section:
   ```markdown
   ## Current Status
   **Last Updated:** YYYY-MM-DD HH:MM:SS
   **Phase:** [New phase name]
   **Tasks Completed:** 0 / [total tasks in new PRD]
   **Current Task:** [First task description]
   ```
3. Append a phase completion entry with full timestamp (`### YYYY-MM-DD HH:MM:SS - Phase N Complete`)
4. Commit the changes:
   ```
   git add specs/manifest.json logs/activity.md
   git commit -m "chore: complete phase N, activate phase N+1"
   ```
5. **If no next phase exists** (all phases complete), output:
   ```
   <promise>COMPLETE</promise>
   ```
6. **Otherwise**, continue with the newly active PRD

## Step 3: Read the Full Task (STOP AND DO THIS)

Find the first task where `"passes": false`.

**⚠️ STOP: Before doing ANYTHING else, you MUST read and output the full task JSON object.**

Use Grep to find the task and read its full structure:
```bash
Grep pattern="description.*[first few words of task]" path="[prd file]" output_mode="content" -C=50
```

**Then output the task's fields in your response:**
- `steps`: [list all steps]
- `acceptance_criteria`: [list all criteria if present]
- `design_quality`: [list all design requirements if present]

**If you cannot list the steps, you have NOT read the task properly. Go back and read it.**

**CRITICAL: Read ALL fields of the task before starting work.** Each task may contain:

| Field | Purpose |
|-------|---------|
| `description` | What the task is about (summary only - not sufficient on its own) |
| `steps` | **Required actions** - follow these step by step |
| `acceptance_criteria` | **What to verify** - task is NOT complete until all criteria pass |
| `design_quality` | **Visual standards** - for UI tasks, verify these design requirements |
| `passes` | Mark `true` only when ALL steps completed AND all criteria verified |

**DO NOT start working until you have read the full task JSON object.** If you only see the `description`, you have NOT read the full task.

## Step 4: Read Required Context Documents

**For Design Phases (13-14):** Before starting ANY task, you MUST read the design plan:
```
Read specs/DESIGN_OVERHAUL_PLAN.md
```
This contains:
- Premium design principles (layered shadows, micro-interactions, typography)
- Anti-AI-slop guardrails (no purple, no Inter, warm orange accent)
- Reference apps (Linear, Raycast, Arc, Vercel)
- Component patterns and shadcn/ui integration

**For Phase 13 specifically:** Also read `specs/DESIGN.md` once it exists (created in Task 2).

**For Phase 14 specifically:** You MUST read `specs/DESIGN.md` for the design requirements before implementing each page.

## Step 5: Identify Task Type

Check the task's `"category"` field:
- If `"planning"` → Follow **PRD Generation Workflow**
- If `"design"` or `"design-req"` or `"design-doc"` → Follow **Design Workflow** (read design plan first!)
- Otherwise → Follow **Implementation Workflow**

---

## PRD Generation Workflow (category: planning)

### 1. Read the Master Plan
- Open `specs/plan.md` and thoroughly read all sections mentioned in the task steps
- Understand the full scope and details for this phase
- Note all implementation patterns, code examples, and requirements

### 2. Create the Phase PRD
- Create the PRD file at the path specified in `"output"` field
- Follow the PRD template structure:
  ```markdown
  # RalphX - Phase N: [Phase Name]

  ## Overview
  [Brief description]

  ## Dependencies
  - Previous phases that must be complete

  ## Scope
  [What's included and excluded]

  ## Detailed Requirements
  [Extracted from master plan - preserve ALL specifics]

  ## Implementation Notes
  [Key patterns, decisions, gotchas]

  ## Task List

  ```json
  [
    {
      "category": "setup|feature|testing",
      "description": "Task description",
      "steps": ["Step 1", "Step 2"],
      "passes": false
    }
  ]
  ```
  ```

- Extract ALL relevant details - don't summarize, preserve specifics
- Create atomic tasks - each completable in one session
- Include TDD requirements - tests before implementation
- Add clear acceptance criteria

### 3. Verify Against Master Plan
- Cross-reference each task against the master plan
- Ensure no requirements are missed
- Check code examples and patterns are preserved

### 4. Log Progress
Update `logs/activity.md`:
- Update the **header section** with current task count and next task
- Append a timestamped entry using format `### YYYY-MM-DD HH:MM:SS - [Title]`:
  - Which phase PRD was created
  - Number of tasks generated
  - Key sections covered

### 5. Update Task Status
Set `"passes": true` for this task in the active PRD

### 6. Commit
```
git add .
git commit -m "docs: create Phase N PRD - [phase name]"
```

---

## Design Workflow (category: design, design-req, design-doc)

### 1. Read the Design Plan (MANDATORY)
```bash
Read specs/DESIGN_OVERHAUL_PLAN.md
```
Understand:
- Premium design principles
- Anti-AI-slop guardrails
- Reference apps for inspiration
- shadcn/ui and Lucide integration patterns

### 2. Read specs/DESIGN.md (if it exists)
For Phase 13 Task 3+, and all of Phase 14, read the master design document.

### 3. Use /frontend-design Skill
For design requirement tasks:
```
/frontend-design
```
This skill helps plan and document premium design patterns.

### 4. Follow Task Steps
- Document requirements in specs/DESIGN.md (Phase 13)
- Implement designs (Phase 14)
- Always verify against acceptance_criteria and design_quality arrays

### 5. Anti-AI-Slop Checklist (EVERY DESIGN TASK)
Before marking complete, verify:
- ❌ NO purple/blue gradients
- ❌ NO Inter font (use SF Pro)
- ❌ NO generic icon grids
- ❌ NO flat, lifeless surfaces
- ✅ Warm orange accent (#ff6b35)
- ✅ Layered shadows for depth
- ✅ Micro-interactions (hover lift, press scale)
- ✅ Proper typography (letter-spacing, line-height)

### 6. Log, Update, Commit
Same as Implementation Workflow.

---

## Implementation Workflow (other categories)

### 1. Start the Application (if needed)
```bash
npm run tauri dev    # For Tauri apps
npm run dev          # For Vite/React only
cargo test           # For Rust-only work
```

### 2. Implement the Task
- Follow task steps exactly
- Write tests FIRST (TDD mandatory)
- Implement to make tests pass
- Run checks:
  ```bash
  npm run lint
  npm run typecheck
  cargo clippy
  cargo test
  ```

### 3. Log Progress
Update `logs/activity.md`:
- Update the **header section** with current task count and next task
- Append a timestamped entry using format `### YYYY-MM-DD HH:MM:SS - [Title]`:
  - What changed
  - Commands run
  - Test results

### 4. Update Task Status
Set `"passes": true` in the active PRD

### 5. Commit
```
git add .
git commit -m "feat: [description]"
```

---

## Important Rules

- **Work on ONE task per iteration, then STOP** - Complete exactly one task, commit, and end your response. Do not continue to the next task.
- Always log progress in `logs/activity.md`
- Always commit after completing a task
- Do NOT run `git init`, change remotes, or push
- For planning: preserve ALL details from master plan
- For implementation: write tests FIRST
- Handle phase transitions automatically via manifest

## Completion

When ALL phases are complete (no more active phases in manifest):

<promise>COMPLETE</promise>
