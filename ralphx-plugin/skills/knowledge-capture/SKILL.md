---
name: knowledge-capture
description: >
  Capture specialized project knowledge from the current session and
  package it into scoped .claude/rules/ files. Triggered by the Stop hook
  when significant knowledge was discovered. Evaluates session context,
  classifies learnings, and creates or updates rule files with paths:
  frontmatter. Can also be invoked manually with /knowledge-capture.
user-invocable: true
---

# Knowledge Capture

Packages session learnings into scoped .claude/rules/ files.

## Step 1: Read Framework Reference

Read the memory framework doc for knowledge capture criteria and format:
`${CLAUDE_PLUGIN_ROOT}/memory-framework.md`

Also fetch/refresh the official Claude Code memory docs (cached locally):
```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-fetch-docs.sh"
```

## Step 2: Evaluate Session

Review the current session's conversation to identify:

### Worth Capturing
- Complex multi-component interactions (e.g., "Component A triggers Event B which updates State C")
- Error recovery procedures that took multiple attempts to resolve
- Deep internal system knowledge (state machines, data flows, edge cases)
- Framework/library quirks and workarounds (e.g., React ref equality, Tauri serialization)
- Patterns that would save >15 minutes if known upfront next time

### NOT Worth Capturing
- One-off fixes unlikely to recur
- Knowledge already documented in existing .claude/rules/ files
- Generic programming knowledge (things Claude already knows)
- Temporary workarounds that should be fixed properly
- Simple bug fixes with obvious causes

If nothing worth capturing -> output "No significant learnings to capture." and stop.

## Step 3: Classify Each Learning

For each learning worth capturing, determine:

### 3a. Scope (which files does it relate to?)
- Identify the files that were involved in the discovery
- Determine glob patterns that cover those files
- Example: discovery about TaskGraph -> paths: ["src/components/TaskGraph/**"]

### 3b. Existing Rule Check
Run the audit script to see current rules:
```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-audit.sh" --json
```

Check if any existing rule file covers the same domain:
- If yes -> append to that file (add new ## section)
- If no -> create new file

### 3c. Format Entry

Use the auto-memory style format:

```markdown
## [Descriptive Title]
- **Problem**: What went wrong or was complex
- **Fix**: How it was resolved
- **File**: Key file paths involved (e.g., `src/components/TaskGraph/TaskGraphView.tsx`)
- **Pattern**: Reusable principle (if applicable)
```

## Step 4: Package

### If appending to existing rule:
1. Read the existing rule file
2. Append the new entry at the end (before any closing sections)
3. Verify file stays under 400 lines
4. If >400 lines -> split (flag for /rule-manager)

### If creating new rule:
1. Create `.claude/rules/<domain>-patterns.md` with:
   ```yaml
   ---
   paths:
     - "path/to/relevant/**"
   ---
   ```
2. Add the knowledge entry
3. Verify the filename is descriptive and kebab-case

## Step 5: Log

```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-log.sh" "Knowledge captured" "Added [title] to [filename] (paths: [patterns])"
```

## Step 6: Report

Output summary:
- What was captured
- Which file(s) were created/updated
- What paths: scoping was applied
- Token cost of new content
