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
2. Log the phase completion in `logs/activity.md` with full timestamp (`### YYYY-MM-DD HH:MM:SS - Phase N Complete`)
3. Commit the manifest update:
   ```
   git add specs/manifest.json logs/activity.md
   git commit -m "chore: complete phase N, activate phase N+1"
   ```
4. **If no next phase exists** (all phases complete), output:
   ```
   <promise>COMPLETE</promise>
   ```
5. **Otherwise**, continue with the newly active PRD

## Step 3: Identify Task Type

Find the single highest priority task where `"passes": false`.

Check the task's `"category"` field:
- If `"planning"` → Follow **PRD Generation Workflow**
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
Append a timestamped entry to `logs/activity.md` using format `### YYYY-MM-DD HH:MM:SS - [Title]`:
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
Append a timestamped entry to `logs/activity.md` using format `### YYYY-MM-DD HH:MM:SS - [Title]`:
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

- Work on ONE task per iteration
- Always log progress in `logs/activity.md`
- Always commit after completing a task
- Do NOT run `git init`, change remotes, or push
- For planning: preserve ALL details from master plan
- For implementation: write tests FIRST
- Handle phase transitions automatically via manifest

## Completion

When ALL phases are complete (no more active phases in manifest):

<promise>COMPLETE</promise>
