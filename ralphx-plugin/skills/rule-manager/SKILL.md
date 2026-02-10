---
name: rule-manager
description: >
  Audit and iteratively optimize .claude/rules/ files for token efficiency.
  Each invocation: audit all rules, pick the single highest-priority
  optimization, apply it, and log the change. Run when the SessionStart
  hook reports issues, when context seems bloated, or periodically for
  maintenance. Uses bash scripts for fast data collection.
user-invocable: true
---

# Rule Manager

Iteratively optimizes .claude/rules/ for token efficiency. Each run: audit -> pick ONE optimization -> apply -> log.

## Step 1: Read Framework Reference

Read the memory framework doc for context on priorities and standards:
`${CLAUDE_PLUGIN_ROOT}/memory-framework.md`

Also fetch/refresh the official Claude Code memory docs (cached locally):

```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-fetch-docs.sh"
```

## Step 2: Audit

Run the audit script to collect current state:

```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-audit.sh" --json
```

Parse the JSON output. Report to user:
- Total files, lines, estimated tokens
- Files with/without paths: frontmatter
- Files with @ references
- Health indicators (oversized, unscoped, refs)

Also read the optimization log to see what's been done (per-day files):
`.claude/memory/` (read the latest file, or all files for full history)

## Step 3: Identify ONE Optimization

Pick the HIGHEST priority that applies (do NOT do multiple):

### Priority 1: @ References Between Rules
If any rule file has @ references to other rule files:

```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-strip-refs.sh" <filename> --dry-run
```

Show the preview to user. If approved:
```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-strip-refs.sh" <filename>
```

### Priority 2: Unscoped Domain Rule
If a rule lacks paths: frontmatter but is domain-specific:

```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-suggest-paths.sh" <filename>
```

Review the suggestions. For complex cases, use an Explore agent to grep the codebase for the rule's key terms. Show proposal to user. If approved:
```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-apply-paths.sh" <filename> "pattern1" "pattern2"
```

### Priority 3: Oversized Rule (>400 LOC)
If a rule exceeds 400 lines:
- Read the file and identify distinct sections
- Propose splitting into 2-3 focused files with paths: frontmatter
- Create new files, delete original
- This is a manual operation (no script -- requires judgment)

### Priority 4: Stale/Orphaned Rule
```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-find-orphans.sh"
```

If orphans found, propose archiving or deletion.

### Priority 5: All Clean
Report "Rules are healthy. No optimization needed." and **STOP** — do NOT proceed to Step 4 (nothing to log).

## Step 4: Log (only when a change was applied)

Skip this step if Priority 5 was reached (no change made).

After applying a change:
```bash
"${CLAUDE_PLUGIN_ROOT}/skills/rule-manager/scripts/rule-log.sh" "<action>" "<details>"
```

Report: "Done. Run /rule-manager again for next optimization."
