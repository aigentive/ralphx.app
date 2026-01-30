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

## Copy Logic

If the plan file is **outside** `specs/plans/`:
1. Derive a name from the plan's title (first `#` heading) or filename
2. Convert to snake_case (e.g., `my-plan.md` → `my_plan.md`)
3. Copy to `specs/plans/<derived_name>.md`
4. Enhance the copy (not the original)

If the plan file is **inside** `specs/plans/`:
- Enhance in place

## Enhancement Steps

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

### Step 3: Add Dependency Annotations

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
- `**Atomic Commit:**` - Derive from task description and files modified

### Step 4: Add Git Workflow Section

After the last `## ` heading in the file (or at the end if no major headings), add:

```markdown
## Commit Lock Workflow (Parallel Agent Coordination)

Reference: `.claude/rules/commit-lock.md`

### Before Committing
```bash
# 1. Establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Check/acquire lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Read lock content, wait 3s, retry up to 30s
  # If stale (same content >30s), delete and proceed
fi

# 3. Create lock
echo "<stream-name> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 4. Stage and commit
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

### After Committing
```bash
# ALWAYS release lock (success or failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Lock Rules
1. Acquire lock BEFORE `git add`
2. Release lock AFTER commit (success OR failure)
3. Stale = same content + >30 sec old
4. Never force-delete active lock from another agent
```

### Step 5: Report Results

Report to the user:
1. Where the enhanced plan is located
2. Number of tasks found and annotated
3. Whether it was copied or enhanced in place

## Commit Scope Detection

Derive commit scope from files being modified:

| Files Modified | Scope |
|----------------|-------|
| `src-tauri/**` | backend service/module name |
| `src/**` | frontend component/feature name |
| `ralphx-mcp-server/**` | mcp |
| `ralphx-plugin/**` | plugin |
| `specs/**`, `docs/**` | docs |
| `.claude/**` | commands, config |

## Commit Type Detection

| Task Description Contains | Type |
|---------------------------|------|
| "create", "add", "implement", "new" | feat |
| "fix", "repair", "correct", "resolve" | fix |
| "update", "modify", "change" | feat (enhancement) |
| "refactor", "extract", "split", "reorganize" | refactor |
| "document", "readme", "template" | docs |
| "test", "spec", "verify" | test |
| Otherwise | chore |

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
