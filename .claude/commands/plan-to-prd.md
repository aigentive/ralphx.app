---
description: Convert a plan file into a phase PRD using the project template
argument-hint: <path-to-plan-file>
allowed-tools: Read, Write, Edit, Glob, Bash, Grep, AskUserQuestion
---

# Plan to PRD Converter

Convert a plan file into a properly formatted phase PRD for the RalphX project.

## Arguments

- `$ARGUMENTS` - Path to the plan file (e.g., `~/.claude/plans/my-plan.md` or a local path)

## Instructions

### Step 1: Read the Plan File

Read the plan file at: `$ARGUMENTS`

If the file doesn't exist, inform the user and stop.

### Step 2: Extract Plan Name

From the plan file path, derive an appropriate name for both:
- The plan copy in `specs/plans/` (e.g., `chat_resumption_unified.md`)
- Use the plan's title or first heading to inform the name

### Step 3: Copy Plan to specs/plans/

Copy the plan content to `specs/plans/<derived_name>.md`

If a file with that name already exists, ask the user if they want to overwrite or use a different name.

### Step 4: Read the PRD Template

Read the phase PRD template at: `specs/templates/phase_prd_template.md`

### Step 5: Determine Next Phase Number

Read `specs/manifest.json` to find:
1. The current phase number
2. All existing phases

The new phase will be `currentPhase + 1` (or the next available number if phases are non-sequential).

### Step 6: Analyze Plan Content

From the plan file, extract:
- **Title/Name**: From the first `#` heading
- **Problem Summary**: What issue this solves
- **Goals**: Main objectives (typically 2-4)
- **Implementation Steps**: Convert to task list
- **Files to Modify**: Use to identify categories (backend/frontend)
- **Verification Steps**: Use for verification checklist

### Step 7: Generate Phase PRD

Create a new file at `specs/phases/prd_phase_<N>_<short_name>.md` using the template structure:

**Fill in:**
- Phase number and name from the plan title
- Overview from problem summary
- Reference to the copied plan file in `specs/plans/`
- Goals extracted from the plan
- Dependencies (infer from plan or ask user if unclear)
- Task list converted from implementation steps:
  - Each major step becomes a task
  - Include `plan_section` pointing to the relevant section in the detailed plan
  - Set all tasks to `"passes": false`
  - Use appropriate categories: `backend`, `frontend`, `mcp`, `agent`, `documentation`
- Key architecture decisions from the plan
- Verification checklist from the plan's verification section

### Step 8: Ask User About Activation

Show the user:
1. Where the plan was copied: `specs/plans/<name>.md`
2. Where the PRD was created: `specs/phases/prd_phase_<N>_<short_name>.md`
3. Phase number assigned
4. Number of tasks generated

Ask if they want to:
- **Activate now** - Set phase status to "active" (requires current active phase to be complete)
- **Save for later** - Set phase status to "pending" (default)

### Step 9: Update Manifest

**ALWAYS add the new phase to `specs/manifest.json`** regardless of user choice.

**If user chose "Activate now":**
1. Set the current active phase's status to "complete"
2. Update `currentPhase` to the new phase number
3. Add the new phase with `"status": "active"`

**If user chose "Save for later" (default):**
1. Keep `currentPhase` unchanged
2. Add the new phase with `"status": "pending"`

**New phase entry format:**
```json
{
  "phase": <N>,
  "name": "<Phase Name from plan title>",
  "prd": "specs/phases/prd_phase_<N>_<short_name>.md",
  "status": "active" | "pending",
  "description": "<Brief 1-line description>"
}
```

### Step 10: Commit All Changes

**ALWAYS commit the plan, PRD, and manifest using the commit lock protocol.**

Reference: `.claude/rules/commit-lock.md`

```bash
# 1. Establish project root
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Acquire commit lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Wait and retry per protocol (see commit-lock.md)
fi
echo "plan-to-prd $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 3. Stage files
git -C "$PROJECT_ROOT" add specs/plans/<plan_name>.md
git -C "$PROJECT_ROOT" add specs/phases/prd_phase_<N>_<short_name>.md
git -C "$PROJECT_ROOT" add specs/manifest.json

# 4. Commit
git -C "$PROJECT_ROOT" commit -m "docs: add Phase <N> PRD for <phase name>

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

# 5. Release lock (ALWAYS, even on failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

**Commit message format:**
- If activating: `docs: add and activate Phase <N> PRD for <phase name>`
- If pending: `docs: add Phase <N> PRD for <phase name>`

### Step 11: Report Results

Report to the user:
1. Files created/modified
2. Phase status (active or pending)
3. Commit hash
4. Next steps (e.g., "Run `/activate-prd` when ready to start this phase")

### Task Conversion Guidelines

When converting plan steps to PRD tasks:

1. **One major implementation step = one task**
2. **Include the plan section reference** so the agent knows where to look
3. **Extract dependencies** from plan step relationships
4. **Add standard steps:**
   - "Read specs/plans/<name>.md section '<Section>'"
   - Implementation steps from the plan
   - Linting step (cargo clippy for backend, npm run lint for frontend)
   - Commit step with appropriate prefix

### Dependency Detection

Look for these patterns in plan steps to extract dependencies:

**Keywords to detect:**
- "depends on", "requires", "after" → task is blockedBy the referenced task
- "blocking", "blocks", "(BLOCKING)" → task blocks the referenced task
- Step order → earlier steps typically block later ones
- Explicit markers like `**Dependencies:** Task N`

**When converting steps, populate:**
- `"id"`: Sequential integer starting at 1
- `"blocking"`: IDs of tasks that cannot start until this one completes
- `"blockedBy"`: IDs of tasks that must complete before this one starts
- `"atomic_commit"`: Commit message for this task

**Example conversion with dependencies:**

Plan step:
```markdown
### Step 1: Wire execution_state to Unified Commands (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(chat): wire execution_state to unified commands`

Modify create_chat_service() to accept execution_state...

### Step 2: Update chat panel to use new API
**Dependencies:** Step 1
**Atomic Commit:** `feat(chat): update panel to use unified API`

Connect the ChatPanel component...
```

Becomes PRD tasks:
```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Wire execution_state to unified chat commands",
    "plan_section": "Step 1: Wire execution_state to Unified Commands",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "fix(chat): wire execution_state to unified commands",
    "steps": [
      "Read specs/plans/chat_resumption_unified.md section 'Step 1'",
      "Modify create_chat_service() to accept execution_state parameter",
      "...",
      "Run cargo clippy && cargo test",
      "Commit: fix(chat): wire execution_state to unified commands"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Update chat panel to use unified API",
    "plan_section": "Step 2: Update chat panel to use new API",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(chat): update panel to use unified API",
    "steps": [
      "Read specs/plans/chat_resumption_unified.md section 'Step 2'",
      "Connect the ChatPanel component...",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): update panel to use unified API"
    ],
    "passes": false
  }
]
```

### Deriving Commit Messages

If the plan doesn't have explicit `atomic_commit` annotations, derive them:

| Files Modified | Scope |
|----------------|-------|
| `src-tauri/**` | backend service/module name |
| `src/**` | frontend component/feature name |
| `ralphx-mcp-server/**` | mcp |
| `ralphx-plugin/**` | plugin |

| Task Description Contains | Type |
|---------------------------|------|
| "create", "add", "implement", "new" | feat |
| "fix", "repair", "correct", "resolve" | fix |
| "update", "modify", "change" | feat |
| "refactor", "extract", "split" | refactor |
| "document", "readme", "template" | docs |

### Category Detection

- Files in `src-tauri/` → `backend`
- Files in `src/` → `frontend`
- Files in `ralphx-mcp-server/` → `mcp`
- Files in `ralphx-plugin/` → `agent`
- Files in `specs/` or `docs/` → `documentation`
