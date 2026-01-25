# Auditor System - Draft Plan

**Status:** Draft - Deferred for future implementation

---

## Problem Statement

During RalphX development (Phase 12), we discovered systemic issues:
- Visual verification was largely skipped during UI phases
- Steps marked complete weren't actually logged
- Required artifacts (screenshots) were missing

Without automated detection, these issues compound over time and require manual discovery.

---

## Proposed Solution

An **Auditor System** that automatically detects and corrects systemic issues in completed work. The auditor:
1. Analyzes completed tasks against defined rules
2. Detects patterns of skipped steps or missing artifacts
3. Generates reconciliation task proposals
4. Integrates with the ideation system for task creation

---

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

## Open Questions

1. **Trigger priority:** When multiple triggers fire simultaneously, which takes precedence?

2. **Rule conflicts:** How to handle when rules contradict each other?

3. **Historical scope:** Should auditor check all completed tasks or only recent ones?

4. **Auto-reconcile safety:** What safeguards prevent auto-reconcile from creating too many tasks?

5. **Methodology rules:** Should methodology rules override or supplement default rules?

6. **Agent model:** Should auditor use same model as worker or a cheaper/faster one?

---

## Implementation Outline

### Backend Components

- `AuditConfig` and `AuditRule` types
- `AuditTriggerService` - monitors events and fires triggers
- `AuditRuleEngine` - evaluates rules against task data
- `ReconciliationService` - generates task proposals from findings
- `AuditorAgent` definition (read-only: Read, Glob, Grep)

### Frontend Components

- Audit settings panel in project settings
- Audit findings view
- Reconciliation approval UI
- `/audit` command integration

### Database

- `audit_runs` table - audit execution history
- `audit_findings` table - detected issues
- `audit_settings` - per-project audit configuration

---

## Dependencies

When this is implemented, it will require:
- Task state machine (Phase 3) - complete
- Activity log system - complete
- Project settings infrastructure - complete
- Ideation system for proposals (Phase 10) - complete

---

## History

- Originally part of Phase 12 (Reconciliation)
- Deferred after Phase 12 to run after design phases
- Moved to draft status to avoid repeated phase bumping

---

## Related Documents

- `specs/phases/prd_phase_12_reconciliation.md` - Original home of auditor tasks
- `specs/plan.md` - Master plan references auditor concept
