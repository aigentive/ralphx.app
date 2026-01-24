# RalphX - Phase 15: Auditor System

## Overview

This phase implements the Auditor System - automatic detection and correction of systemic issues in completed work. The auditor analyzes completed tasks, detects patterns of skipped steps or missing artifacts, and generates reconciliation tasks.

**Moved from Phase 12:** These tasks were originally part of Phase 12 (Reconciliation) but have been deferred to run after the design phases.

## Goals

1. Define audit configuration and rule types
2. Implement artifact checking and pattern detection
3. Create the auditor agent for retrospective analysis
4. Implement trigger system (queue_empty, manual, time-based)
5. Generate reconciliation task proposals from findings
6. Integrate with project settings and UI

## Dependencies

- Phase 14 must be complete (design implementation finished)
- Task state machine from Phase 3
- Activity log system
- Project settings infrastructure

## Why Auditor Exists

RalphX discovered during Phase 12 that:
- Visual verification was largely skipped during UI phases
- Steps marked complete weren't actually logged
- Required artifacts (screenshots) were missing

The Auditor prevents this by automatically detecting such issues and creating tasks to fix them.

## Key Concepts

### Audit Triggers

| Trigger | When | Use Case |
|---------|------|----------|
| `queue_empty` | All tasks approved/completed | End of work session |
| `task_count` | Every N tasks approved | Periodic checkup |
| `terminal_state` | Task enters failed/cancelled | Post-mortem |
| `manual` | User runs `/audit` | On-demand |
| `time_based` | Every X hours | Long sessions |

### Audit Rules

Rules define what to check:
- `activity_pattern` - Regex match against activity log
- `artifact_exists` - Glob pattern for file existence
- `step_logged` - Check if step keyword appears in log
- `custom` - Custom check function

### Severity Levels

- `low` - Minor issue, informational
- `medium` - Should be addressed
- `high` - Must be addressed soon
- `critical` - Blocks further progress

---

## Task List

**IMPORTANT: Work on ONE task per iteration.** Find the first task with `"passes": false`, complete it, update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "auditor",
    "description": "Define AuditConfig and AuditRule types",
    "steps": [
      "Create src/types/audit.ts with AuditTrigger, AuditConfig, AuditRule interfaces",
      "Define trigger types: task_count, queue_empty, terminal_state, manual, time_based",
      "Define check types: activity_pattern, artifact_exists, step_logged, custom",
      "Define severity levels: low, medium, high, critical",
      "Add Zod schemas for validation",
      "Run npm run typecheck"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement default audit rules",
    "steps": [
      "Create src/lib/audit/defaultRules.ts",
      "Export defaultAuditRules array (we'll decide later, for now blank list)",
      "Write unit tests for rule matching logic"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement artifact checker",
    "steps": [
      "Create src/lib/audit/artifactChecker.ts",
      "Implement glob-based file existence checks",
      "Check screenshots directory for PNG files",
      "Check for test files matching patterns",
      "Return structured results (found/missing, paths)",
      "Write unit tests"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Create auditor agent definition",
    "steps": [
      "Create ralphx-plugin/agents/auditor.md",
      "Define role: retrospective analysis of completed work",
      "Define inputs: activity log, task list, audit rules",
      "Define outputs: reconciliation task proposals",
      "Define tools: Read, Glob, Grep (no Write/Edit - read-only)",
      "Add to plugin.json agents list"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement audit trigger system",
    "steps": [
      "Create src/lib/audit/triggers.ts",
      "Implement task_count trigger (after N tasks approved)",
      "Implement queue_empty trigger (all tasks in terminal state)",
      "Implement manual trigger (user command)",
      "Implement time_based trigger (interval timer)",
      "Integrate with task state machine events",
      "Write unit tests for each trigger"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement reconciliation task generator",
    "steps": [
      "Create src/lib/audit/reconciliation.ts",
      "Generate task proposals from audit findings",
      "Map severity to task priority",
      "Group related findings into single tasks",
      "Format task descriptions with evidence from audit",
      "Support autoReconcile (auto-add) vs approval mode",
      "Write unit tests"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Add audit settings to project configuration",
    "steps": [
      "Add audit section to project settings schema",
      "Default: enabled=true, triggers=[queue_empty, manual], autoReconcile=false",
      "Add UI for audit settings in Settings panel",
      "Persist settings to database",
      "Write tests for settings CRUD"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement /audit command",
    "steps": [
      "Create ralphx-plugin/skills/audit/SKILL.md",
      "Define /audit slash command for manual trigger",
      "Show audit progress in UI",
      "Display findings summary when complete",
      "Allow user to approve/reject reconciliation tasks",
      "Document command usage"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Add methodology-aware audit rules (extensibility)",
    "steps": [
      "Extend MethodologyConfig with audit rules",
      "Add phase_complete trigger for BMAD",
      "Add wave_complete trigger for GSD",
      "Allow methodologies to define custom rules",
      "Merge methodology rules with default rules",
      "Write tests for methodology integration"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "End-to-end test of auditor system",
    "steps": [
      "Create test project with intentionally skipped steps",
      "Trigger audit manually",
      "Verify correct findings are detected",
      "Verify reconciliation tasks are generated",
      "Test autoReconcile mode",
      "Test approval mode",
      "Document test results in activity.md"
    ],
    "passes": false
  }
]
```

---

## Integration Points

| System | How Auditor Integrates |
|--------|------------------------|
| **Supervisor** | Auditor is "retrospective supervisor" - same severity model |
| **Review System** | Auditor reviews batches of tasks, not individual tasks |
| **Activity Log** | Primary data source for pattern detection |
| **Ideation** | Audit findings can feed into ideation as proposals |
| **Methodologies** | Each methodology can define additional audit rules |

---

## Example Audit Rules

```typescript
const exampleRules: AuditRule[] = [
  {
    name: "visual_verification_logged",
    description: "UI tasks should have visual verification in activity log",
    check: "activity_pattern",
    pattern: "screenshot|visual verification|agent-browser",
    severity: "medium"
  },
  {
    name: "screenshots_captured",
    description: "UI tasks should produce screenshots",
    check: "artifact_exists",
    path: "screenshots/*.png",
    severity: "medium"
  },
  {
    name: "tests_created",
    description: "Tasks with TDD requirement should create test files",
    check: "artifact_exists",
    path: "**/*.test.{ts,tsx}",
    severity: "high"
  },
  {
    name: "tauri_dev_used",
    description: "UI verification should use 'tauri dev' not just 'npm run dev'",
    check: "activity_pattern",
    pattern: "tauri dev",
    severity: "low"
  }
];
```
