---
description: Enhance a plan with Git workflow references and task dependencies
argument-hint: <path-to-plan-file>
allowed-tools: Read, Write, Edit, Glob, Bash, Grep, AskUserQuestion
---

# Enhance Plan with Git Workflow

Enhance a plan file with Git workflow references (commit lock protocol) and task dependency annotations.

## Arguments

- `$ARGUMENTS` - Optional path to the plan file. If not provided, will detect active plan or find most recent.

## Input Detection Logic

Determine the plan file to enhance in this order:

### 1. Explicit Argument
If `$ARGUMENTS` is provided and non-empty, use that path.

### 2. Most Recent Plan in ~/.claude/plans/
If no argument provided:
```bash
ls -t ~/.claude/plans/*.md 2>/dev/null | head -1
```
Use the most recently modified `.md` file in `~/.claude/plans/`.

### 3. Ask User
If no plans found in `~/.claude/plans/`, use AskUserQuestion to ask:
- "Which plan file would you like to enhance?"
- Options: Allow user to specify a path

## Copy Logic (MANDATORY - DO NOT SKIP)

**CRITICAL: You MUST copy the file using the Bash `cp` command. NEVER recreate the file content using Write tool.**

This is enforced because:
1. Plans can be large and copying preserves exact content
2. Manual recreation loses formatting, whitespace, and introduces errors
3. The original plan is the source of truth

### Step 1: Ensure target directory exists
```bash
mkdir -p specs/plans
```

### Step 2: Derive target filename
1. Read the first `#` heading from the plan to get the title
2. Convert to snake_case (e.g., "My Feature Plan" → `my_feature_plan.md`)
3. If no title found, use the original filename converted to snake_case

### Step 3: Copy using Bash cp command (MANDATORY)

**YOU MUST USE THIS EXACT PATTERN:**
```bash
cp "<source_plan_path>" "specs/plans/<derived_name>.md"
```

**DO NOT:**
- ❌ Use Write tool to create the file with the plan content
- ❌ Read the source and then Write to the destination
- ❌ Manually recreate any part of the plan content

**DO:**
- ✅ Use Bash tool with `cp` command
- ✅ Then use Edit tool to make modifications to the copy

### Step 4: Verify copy succeeded
```bash
ls -la "specs/plans/<derived_name>.md"
```

### Step 5: Enhance the copy using Edit tool
All modifications happen on `specs/plans/<derived_name>.md` using the Edit tool, never the original.

**Exception:** If the source is already inside `specs/plans/`, enhance in place (no copy needed).

## Enhancement Steps

**CRITICAL: Before annotating tasks, read `.claude/rules/task-planning.md` for compilation unit rules.**

### Step 1: Check Idempotency

Read the plan file and check if it already contains a "Commit Lock Workflow" section.

If found, inform the user that the plan has already been enhanced and stop.

### Step 2: Analyze Task Structure

Look for task-like structures in the plan:
- `### Task N:` headings
- `### Step N:` headings
- Numbered lists with implementation steps
- Sections with "Files to Modify" or "Implementation Tasks"

For each task/step found, analyze:
1. Dependencies (look for "depends on", "requires", "after", "blocking", "blocks")
2. Whether it blocks other tasks
3. What files it modifies (for deriving commit scope)
4. **Whether it forms a compilation unit with adjacent tasks** (see task-planning.md)

### Step 3: Validate Compilation Units

**Before adding dependency annotations, check for the chicken-egg problem:**

For each task that modifies existing code:
1. Does it rename a field, function, or type?
2. Does it change a function signature?
3. Does it remove an export?

If yes to any: **All files referencing the changed item must be in the SAME task.**

**Example of what to catch:**
```markdown
### Task 1: Rename `comments` to `feedback` in Request struct
### Task 2: Update handler to use `req.feedback`
    Dependencies: Task 1
```
This is WRONG. Task 1 alone breaks compilation. Merge them:
```markdown
### Task 1: Rename `comments` to `feedback` and update handler
```

See `.claude/rules/task-planning.md` for full compilation unit rules.

### Step 5: Add Dependency Annotations

For each task that doesn't already have dependency annotations, add:

```markdown
### Task N: Description (BLOCKING)
**Dependencies:** None | Task M
**Atomic Commit:** `type(scope): description`
```

Or for non-blocking tasks:
```markdown
### Task N: Description
**Dependencies:** Task M, Task K
**Atomic Commit:** `type(scope): description`
```

**Annotation rules:**
- `(BLOCKING)` - Add to task title if other tasks depend on it
- `**Dependencies:**` - List task numbers that must complete first, or "None"
- `**Atomic Commit:**` - Derive from task description and files modified (see `.claude/rules/task-planning.md`)

### Step 7: Add Git Workflow Section

After the last `## ` heading in the file (or at the end if no major headings), add:

```markdown
## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
```

### Step 8: Report Results

Report to the user:
1. Where the enhanced plan is located
2. Number of tasks found and annotated
3. Whether it was copied or enhanced in place

## Commit Message Conventions

**See `.claude/rules/task-planning.md` for full commit type and scope detection tables.**

Quick reference:
- **Scope:** Derived from files modified (src-tauri → backend, src → frontend, etc.)
- **Type:** Derived from task description (add/create → feat, fix → fix, etc.)

## Example

Input plan:
```markdown
# My Feature Plan

## Implementation Tasks

### Task 1: Create base component
Files: src/components/MyComponent.tsx

### Task 2: Wire component to parent
Depends on Task 1
Files: src/components/Parent.tsx
```

Output (enhanced):
```markdown
# My Feature Plan

## Implementation Tasks

### Task 1: Create base component (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(components): create base MyComponent`

Files: src/components/MyComponent.tsx

### Task 2: Wire component to parent
**Dependencies:** Task 1
**Atomic Commit:** `feat(components): wire MyComponent to parent`

Files: src/components/Parent.tsx

## Commit Lock Workflow (Parallel Agent Coordination)
...
```
