---
description: Convert a plan file into a phase PRD using the project template
argument-hint: <path-to-plan-file>
allowed-tools: Read, Write, Glob, Bash
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

### Step 8: Confirm with User

Show the user:
1. Where the plan was copied: `specs/plans/<name>.md`
2. Where the PRD was created: `specs/phases/prd_phase_<N>_<short_name>.md`
3. Phase number assigned
4. Number of tasks generated

Ask if they want to:
- Activate this phase in the manifest (update `specs/manifest.json`)
- Just create the files without activating

### Task Conversion Guidelines

When converting plan steps to PRD tasks:

1. **One major implementation step = one task**
2. **Include the plan section reference** so the agent knows where to look
3. **Add standard steps:**
   - "Read specs/plans/<name>.md section '<Section>'"
   - Implementation steps from the plan
   - Linting step (cargo clippy for backend, npm run lint for frontend)
   - Commit step with appropriate prefix

**Example conversion:**

Plan step:
```
### Step 1: Wire execution_state to Unified Commands
Modify create_chat_service() to accept execution_state...
```

Becomes PRD task:
```json
{
  "category": "backend",
  "description": "Wire execution_state to unified chat commands",
  "plan_section": "Step 1: Wire execution_state to Unified Commands",
  "steps": [
    "Read specs/plans/chat_resumption_unified.md section 'Step 1'",
    "Modify create_chat_service() to accept execution_state parameter",
    "...",
    "Run cargo clippy && cargo test",
    "Commit: fix(chat): wire execution_state to unified chat commands"
  ],
  "passes": false
}
```

### Category Detection

- Files in `src-tauri/` → `backend`
- Files in `src/` → `frontend`
- Files in `ralphx-mcp-server/` → `mcp`
- Files in `ralphx-plugin/` → `agent`
- Files in `specs/` or `docs/` → `documentation`
