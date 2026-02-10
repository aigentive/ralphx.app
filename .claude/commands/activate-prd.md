# Activate PRD

Manually activate a different PRD file, updating the manifest to change the active phase.

## Arguments
- `$ARGUMENTS` - Path to the PRD file to activate (e.g., `specs/phases/prd_phase_03_state_machine.md`)

## Instructions

You are helping the user manually switch the active PRD. This is typically used for:
- Emergency course corrections
- Inserting a new phase before continuing
- Skipping ahead to a specific phase
- Going back to fix issues in a previous phase

### Step 1: Assess Current State

Read `specs/manifest.json` to understand:
1. Which phase is currently active
2. How many tasks are complete vs remaining in the current PRD

### Step 2: Validate the Target PRD

Check that the provided PRD file exists: `$ARGUMENTS`

If the file doesn't exist, inform the user and ask if they want to:
- Create a new PRD at that path
- Choose from existing PRDs in `specs/phases/`

### Step 3: Present Options

Use AskUserQuestion to ask the user what to do with the **currently active PRD**:

**Question:** "You're switching from Phase N ([current phase name]) to [target phase]. What should happen to the current phase?"

**Options:**

1. **Mark as complete** (Recommended if current phase tasks are done)
   - Sets current phase status to `"complete"`
   - Use when: All tasks in current PRD have `passes: true`

2. **Mark as paused**
   - Sets current phase status to `"paused"`
   - Use when: Need to temporarily work on something else, will return later

3. **Mark as blocked**
   - Sets current phase status to `"blocked"`
   - Use when: Current phase can't continue until target phase is done

4. **Keep as-is (just switch focus)**
   - Leaves current phase status unchanged
   - Use when: Running parallel work or testing

Also ask about the **target PRD**:

**Question:** "How should the target phase be set up?"

**Options:**

1. **Activate from beginning** (Recommended for new phases)
   - Sets target phase status to `"active"`
   - All tasks remain as-is

2. **Reset and activate**
   - Sets target phase status to `"active"`
   - Resets all tasks in target PRD to `passes: false`
   - Use when: Need to redo a phase

3. **Insert as new phase**
   - If target is a new PRD not in manifest, adds it to the phases array
   - Renumbers subsequent phases if needed

### Step 4: Update Manifest

Based on user's choices, update `specs/manifest.json`:

```json
{
  "currentPhase": [new phase number],
  "phases": [
    // Update statuses as per user's choices
  ]
}
```

### Step 5: Commit Changes

```bash
git add specs/manifest.json
git commit -m "chore: manual PRD switch - activate [target phase name]"
```

### Step 6: Confirm

Tell the user:
- The manifest has been updated
- Which phase is now active
- How to continue: `./ralph.sh N` or continue in current session

---

## Example Usage

```
/activate-prd specs/phases/prd_phase_02_data_layer.md
```

This would:
1. Show current state (e.g., "Phase 1 Foundation is active, 8/15 tasks complete")
2. Ask what to do with Phase 1 (pause, complete, block, etc.)
3. Ask how to set up Phase 2 (activate, reset, etc.)
4. Update manifest and commit
5. Confirm the switch

---

## Edge Cases

### Target PRD not in manifest
If the PRD file exists but isn't in the manifest phases array:
- Offer to insert it as a new phase
- Ask where in the sequence it should go
- Update phase numbers accordingly

### No currently active phase
If no phase has `"active"` status:
- Skip the "what to do with current phase" question
- Just activate the target

### Target is already active
If target PRD is already the active phase:
- Inform user "This phase is already active"
- Ask if they want to reset it (set all tasks to `passes: false`)
