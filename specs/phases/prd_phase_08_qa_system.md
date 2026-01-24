# RalphX - Phase 8: QA System

## Overview

This phase implements the built-in QA system with two-phase approach: QA Prep (background parallel to execution) and QA Testing (post-execution verification using agent-browser). The system enables per-task or global QA configuration with acceptance criteria generation, implementation refinement, and browser-based visual testing.

## Dependencies

- Phase 3 must be complete (State Machine with QA states: `qa_prepping`, `qa_refining`, `qa_testing`, `qa_passed`, `qa_failed`)
- Phase 4 must be complete (Agentic Client for spawning QA agents)
- Phase 5 must be complete (Frontend Core for QA event handling)
- Phase 6 must be complete (Kanban UI for QA status display)
- Phase 7 must be complete (Agent System for QA agent profiles)

## Scope

### Included
- QA configuration system (global and per-task)
- task_qa table and migrations for QA artifacts
- QA Prep Agent implementation (background, non-blocking)
- QA Executor Agent implementation (refinement + testing)
- agent-browser skill integration for visual testing
- QA result storage and display
- QA-related state transitions and side effects
- QA status badges and task detail panel
- Settings UI for QA configuration
- Task creation with QA options
- Screenshot capture and storage

### Excluded
- Custom QA agent profiles (uses built-in profiles)
- External CI/CD integration
- Advanced test coverage metrics
- Cross-browser testing (single browser only)

## Detailed Requirements

### Two-Phase QA Architecture

```
PLANNED (user action)
    │
    ├──→ [Spawn QA Prep Agent] ──→ Generates acceptance criteria ──→ Stores in task_qa
    │                                    (runs in background)
    │
    └──→ [Auto-pick up for execution] ──→ IN_PROGRESS ──→ EXECUTION_DONE
                                                            │
                                                            ▼
                                                    QA_REFINING
                                                    • Waits for QA Prep if still running
                                                    • Refines test plan based on git diff
                                                            │
                                                            ▼
                                                    QA_TESTING
                                                    • Runs browser tests via agent-browser
                                                    • Captures screenshots
                                                            │
                                                ┌───────────┴───────────┐
                                                ▼                       ▼
                                          QA_PASSED               QA_FAILED
                                                │                       │
                                                ▼                       ▼
                                        PENDING_REVIEW          REVISION_NEEDED
```

### QA Configuration Types

```typescript
// Global QA settings (stored in project settings)
interface QASettings {
  qa_enabled: boolean;              // Default: true
  auto_qa_for_ui_tasks: boolean;    // Default: true
  auto_qa_for_api_tasks: boolean;   // Default: false
  qa_prep_enabled: boolean;         // Default: true
  browser_testing_enabled: boolean; // Default: true
  browser_testing_url: string;      // Default: http://localhost:1420
}

// Per-task QA configuration
interface TaskQAConfig {
  needs_qa: boolean | null;         // null = use global setting
  qa_prep_status: 'pending' | 'running' | 'completed' | 'failed';
  qa_test_status: 'pending' | 'waiting_for_prep' | 'running' | 'passed' | 'failed';
}
```

### Database Schema

```sql
-- Extended task schema for QA
ALTER TABLE tasks ADD COLUMN needs_qa BOOLEAN DEFAULT NULL;
ALTER TABLE tasks ADD COLUMN qa_prep_status TEXT;
ALTER TABLE tasks ADD COLUMN qa_test_status TEXT;

-- QA artifacts table
CREATE TABLE task_qa (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),

  -- Phase 1: QA Prep (runs in parallel with execution)
  acceptance_criteria TEXT,      -- JSON array of criteria
  qa_test_steps TEXT,            -- JSON array of test steps (initial)
  prep_agent_id TEXT,            -- Agent that generated this
  prep_started_at DATETIME,
  prep_completed_at DATETIME,

  -- Phase 2: QA Refinement (after execution completes)
  actual_implementation TEXT,    -- Summary of what was actually done
  refined_test_steps TEXT,       -- Test steps updated based on actual implementation
  refinement_agent_id TEXT,
  refinement_completed_at DATETIME,

  -- Phase 3: Test Execution (browser tests)
  test_results TEXT,             -- JSON array of test results
  screenshots TEXT,              -- JSON array of screenshot paths
  test_agent_id TEXT,
  test_completed_at DATETIME,

  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_task_qa_task_id ON task_qa(task_id);
```

### Acceptance Criteria Format

```json
{
  "acceptance_criteria": [
    {
      "id": "AC1",
      "description": "User can see the task board with 7 columns",
      "testable": true,
      "type": "visual"
    },
    {
      "id": "AC2",
      "description": "Dragging a task to 'Planned' column triggers execution",
      "testable": true,
      "type": "behavior"
    }
  ]
}
```

### QA Test Steps Format

```json
{
  "qa_steps": [
    {
      "id": "QA1",
      "criteria_id": "AC1",
      "description": "Verify task board renders with correct columns",
      "commands": [
        "agent-browser open http://localhost:1420",
        "agent-browser wait --load",
        "agent-browser snapshot -i -c",
        "agent-browser is visible [data-testid='column-draft']",
        "agent-browser is visible [data-testid='column-planned']",
        "agent-browser screenshot screenshots/task-board-columns.png"
      ],
      "expected": "All 7 columns visible"
    }
  ]
}
```

### QA Test Results Format

```json
{
  "qa_results": {
    "task_id": "task-123",
    "overall_status": "passed",
    "total_steps": 5,
    "passed_steps": 5,
    "failed_steps": 0,
    "steps": [
      {
        "step_id": "QA1",
        "status": "passed",
        "screenshot": "screenshots/qa1-result.png",
        "actual": null,
        "expected": null,
        "error": null
      }
    ]
  }
}
```

### QA Prep Agent Profile

```typescript
const qaPrepProfile: AgentProfile = {
  id: "qa-prep",
  name: "QA Prep Agent",
  role: "qa_prep",
  claudeCode: {
    agentDefinition: ".claude/agents/qa-prep.md",
    skills: ["acceptance-criteria-writing", "qa-step-generation"],
  },
  execution: {
    model: "sonnet",
    maxIterations: 10,
    timeoutMinutes: 5,
    permissionMode: "default",
  },
  io: {
    inputArtifactTypes: ["task_spec", "context"],
    outputArtifactTypes: ["acceptance_criteria", "qa_steps"],
  },
};
```

### QA Executor Agent Profile

```typescript
const qaExecutorProfile: AgentProfile = {
  id: "qa-executor",
  name: "QA Executor Agent",
  role: "qa_executor",
  claudeCode: {
    agentDefinition: ".claude/agents/qa-executor.md",
    skills: ["agent-browser", "qa-evaluation"],
  },
  execution: {
    model: "sonnet",
    maxIterations: 30,
    timeoutMinutes: 15,
    permissionMode: "acceptEdits",
  },
  io: {
    inputArtifactTypes: ["task_spec", "acceptance_criteria", "qa_steps", "code_change"],
    outputArtifactTypes: ["qa_results", "screenshots"],
  },
};
```

### Agent-Browser Commands Reference

```bash
# Navigation
agent-browser open <url>
agent-browser close
agent-browser reload

# Page Analysis
agent-browser snapshot
agent-browser snapshot -i         # Interactive elements only
agent-browser snapshot -c         # Compact output
agent-browser snapshot -i -c      # Interactive + compact (recommended)

# Screenshots
agent-browser screenshot <path.png>
agent-browser screenshot --full <path.png>

# Interactions
agent-browser click @e1
agent-browser fill @e1 "text"
agent-browser type @e1 "text"
agent-browser press Enter
agent-browser hover @e1
agent-browser scroll @e1
agent-browser drag @e1 @e2

# Data Extraction
agent-browser get text @e1
agent-browser get value @e1
agent-browser get attr @e1 href

# State Verification
agent-browser is visible @e1
agent-browser is enabled @e1
agent-browser is checked @e1

# Wait Conditions
agent-browser wait @e1
agent-browser wait 2000
agent-browser wait --load
```

### Side Effects for QA Transitions

```typescript
const QA_SIDE_EFFECTS: Record<string, SideEffect[]> = {
  // READY triggers QA prep in background (if enabled)
  "ready->executing": [
    { type: "spawn_background_agent", profile: "qa-prep", condition: "qaEnabled" },
    { type: "spawn_agent", profile: "worker" },
  ],

  // EXECUTION_DONE triggers QA or review
  "execution_done->qa_refining": [
    { type: "wait_for_agent", profile: "qa-prep" },
    { type: "spawn_agent", profile: "qa-refiner" },
  ],

  "qa_refining->qa_testing": [
    { type: "spawn_agent", profile: "qa-tester" },
  ],

  "qa_testing->qa_passed": [
    { type: "emit_event", event: "qa_passed" },
  ],

  "qa_testing->qa_failed": [
    { type: "notify_user", message: "QA tests failed" },
    { type: "emit_event", event: "qa_failed" },
  ],

  "qa_failed->revision_needed": [
    { type: "create_revision_task", includeQaFailures: true },
  ],

  "qa_passed->pending_review": [
    { type: "spawn_agent", profile: "reviewer" },
  ],
};
```

### UI Components

#### TaskQABadge Component
Shows QA status on task cards with color coding:
- pending: gray
- preparing: yellow
- ready: blue
- testing: purple
- passed: green
- failed: red

#### TaskDetailQAPanel Component
Tabbed panel showing:
- Acceptance criteria list with check marks
- Test results with pass/fail status
- Screenshot gallery with lightbox view
- Failure details with expected vs actual

#### QASettingsPanel Component
Settings page section for:
- Global QA toggle
- Auto-QA for UI/API tasks
- QA phases toggles (prep, evaluation, browser testing)
- Browser testing URL configuration
- Start command and wait time

### Visual Verification Patterns

```bash
# Pattern 1: Component Renders
agent-browser open http://localhost:1420
agent-browser wait --load
agent-browser snapshot -i -c
agent-browser is visible "[data-testid='task-board']"
agent-browser screenshot screenshots/task-board-renders.png
agent-browser close

# Pattern 2: Drag-Drop
agent-browser open http://localhost:1420
agent-browser wait --load
agent-browser snapshot -i -c
agent-browser drag @e5 @e8
agent-browser screenshot screenshots/drag-drop.png
agent-browser get text @e8
agent-browser close

# Pattern 3: Form Submission
agent-browser open http://localhost:1420/new-task
agent-browser wait --load
agent-browser fill "[name='title']" "Test Task"
agent-browser click "[type='submit']"
agent-browser wait 1000
agent-browser screenshot screenshots/form-submitted.png
agent-browser is visible "[data-testid='success-message']"
agent-browser close

# Pattern 4: Status Change
agent-browser open http://localhost:1420
agent-browser wait --load
agent-browser click "[data-testid='task-123-move-executing']"
agent-browser wait 2000
agent-browser screenshot screenshots/status-executing.png
agent-browser is visible "[data-testid='agent-activity-stream']"
agent-browser close
```

## Implementation Notes

### Key Design Decisions

1. **Parallel Execution Model**: QA Prep runs concurrently with task execution - no blocking. If execution finishes before QA Prep, QA Testing waits for the plan.

2. **Refinement Step**: QA Executor first refines test steps based on actual git diff, ensuring tests match what was actually implemented, not just original intent.

3. **Per-Task Override**: Tasks can override global QA settings with `needs_qa` boolean. NULL means inherit from global.

4. **Screenshots Directory**: All screenshots stored in `screenshots/` with task-specific naming.

5. **Cost-Optimized Testing**: For testing QA agents themselves, use minimal echo prompts:
   ```typescript
   QA_PREP_TEST: 'QA_PREP_TEST_OK'
   QA_REFINE_TEST: 'QA_REFINE_TEST_OK'
   QA_TEST_TEST: 'QA_TEST_TEST_OK'
   ```

### File Size Limits

- Agent definitions: 100 lines max
- Skills: 150 lines max
- UI components: 100 lines max
- Hooks: 50 lines max

### TDD Requirements

All tasks require TDD:
1. Write failing tests first
2. Implement to pass tests
3. Refactor if needed
4. Run full test suite before marking complete

### Anti-AI-Slop Guardrails

- No generic error messages - be specific
- No placeholder screenshots - capture actual UI
- No mock data in production code
- Use real agent-browser commands, not simulated

## Task List

```json
[
  {
    "category": "setup",
    "description": "Create screenshots directory and gitkeep",
    "steps": [
      "Create screenshots/ directory at project root",
      "Add .gitkeep to preserve directory in git",
      "Add screenshots/*.png to .gitignore (keep .gitkeep)",
      "Verify directory structure"
    ],
    "passes": true
  },
  {
    "category": "setup",
    "description": "Install agent-browser globally and create skill",
    "steps": [
      "Document agent-browser installation in README or setup script",
      "Create .claude/skills/agent-browser/SKILL.md with all commands documented",
      "Verify skill structure follows Claude Code skill format"
    ],
    "passes": true
  },
  {
    "category": "setup",
    "description": "Update Claude Code settings for agent-browser permissions",
    "steps": [
      "Read current .claude/settings.json",
      "Add agent-browser permission patterns for all command categories",
      "Include: open, close, snapshot, screenshot, click, fill, get, is, wait, drag",
      "Verify JSON is valid"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create QA configuration types in Rust",
    "steps": [
      "Write tests for QASettings serialization/deserialization",
      "Create QASettings struct with all fields (qa_enabled, auto_qa_for_ui_tasks, etc.)",
      "Create TaskQAConfig struct (needs_qa, qa_prep_status, qa_test_status)",
      "Implement Default trait for QASettings",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create QA configuration types in TypeScript",
    "steps": [
      "Write tests for Zod schema validation",
      "Create QASettings Zod schema matching Rust struct",
      "Create TaskQAConfig Zod schema",
      "Create QAPrepStatus and QATestStatus enums",
      "Export from types module",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create task_qa table migration",
    "steps": [
      "Write integration test for task_qa table creation",
      "Create migration file for task_qa table",
      "Include all columns: acceptance_criteria, qa_test_steps, prep_agent_id, etc.",
      "Create index on task_id",
      "Run migration and verify schema"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Add QA columns to tasks table migration",
    "steps": [
      "Write test for task QA columns",
      "Create migration adding needs_qa, qa_prep_status, qa_test_status to tasks",
      "Run migration and verify columns exist",
      "Test nullable behavior (needs_qa can be NULL)"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create AcceptanceCriteria and QATestStep types",
    "steps": [
      "Write tests for JSON serialization",
      "Create AcceptanceCriterion struct (id, description, testable, type)",
      "Create AcceptanceCriteriaType enum (visual, behavior, data, accessibility)",
      "Create QATestStep struct (id, criteria_id, description, commands, expected)",
      "Implement JSON parsing for storage",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create QAResult types",
    "steps": [
      "Write tests for result serialization",
      "Create QAStepResult struct (step_id, status, screenshot, actual, expected, error)",
      "Create QAResults struct (task_id, overall_status, steps, totals)",
      "Create QAStepStatus enum (pending, running, passed, failed, skipped)",
      "Implement from_json and to_json",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskQA entity and repository trait",
    "steps": [
      "Write tests for TaskQA CRUD operations",
      "Create TaskQA entity struct with all fields from schema",
      "Create TaskQARepository trait with methods: create, get_by_task_id, update_prep, update_refinement, update_results",
      "Add get_pending_prep method for finding tasks needing QA prep",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteTaskQARepository",
    "steps": [
      "Write integration tests with real SQLite",
      "Implement create method",
      "Implement get_by_task_id method",
      "Implement update_prep method",
      "Implement update_refinement method",
      "Implement update_results method",
      "Test JSON column handling",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create QA Prep Agent definition",
    "steps": [
      "Create .claude/agents/qa-prep.md",
      "Add frontmatter: name, description, tools (Read, Grep, Glob only)",
      "Add disallowedTools: Write, Edit, Bash",
      "Write system prompt for acceptance criteria generation",
      "Document output format (JSON with acceptance_criteria and qa_steps)",
      "Include guidelines for testability and specificity"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create QA Executor Agent definition",
    "steps": [
      "Create .claude/agents/qa-executor.md",
      "Add frontmatter: name, description, tools (Read, Grep, Glob, Bash)",
      "Add skills: agent-browser",
      "Write system prompt for Phase 2A (evaluation via git diff)",
      "Write system prompt for Phase 2B (browser test execution)",
      "Document output format (JSON with qa_results)",
      "Include error handling guidelines"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create QA-related skills",
    "steps": [
      "Create .claude/skills/acceptance-criteria-writing/SKILL.md",
      "Document criteria format and best practices",
      "Create .claude/skills/qa-step-generation/SKILL.md",
      "Document test step format with agent-browser commands",
      "Create .claude/skills/qa-evaluation/SKILL.md",
      "Document git diff analysis and refinement process"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement QAService for orchestrating QA flow",
    "steps": [
      "Write tests for QA service methods",
      "Create QAService struct with repository and agent client dependencies",
      "Implement start_qa_prep method (spawns background agent)",
      "Implement check_prep_complete method",
      "Implement wait_for_prep method",
      "Implement start_qa_testing method",
      "Implement record_results method",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Integrate QA with state machine transitions",
    "steps": [
      "Write tests for QA-enabled state transitions",
      "Update Ready state onEnter to spawn QA prep if enabled",
      "Update ExecutionDone auto-transition to check qaEnabled",
      "Implement QaRefining state with wait-for-prep logic",
      "Implement QaTesting state with agent spawn",
      "Implement QaPassed auto-transition to PendingReview",
      "Implement QaFailed notification",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for QA operations",
    "steps": [
      "Write tests for each Tauri command",
      "Create get_qa_settings command",
      "Create update_qa_settings command",
      "Create get_task_qa command (returns TaskQA for a task)",
      "Create get_qa_results command",
      "Create retry_qa command (re-run QA tests)",
      "Create skip_qa command (bypass QA failure)",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TypeScript QA types and Zod schemas",
    "steps": [
      "Write tests for schema validation",
      "Create AcceptanceCriterion schema",
      "Create QATestStep schema",
      "Create QAStepResult schema",
      "Create QAResults schema",
      "Create TaskQA schema",
      "Export all from types/qa.ts",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrappers for QA",
    "steps": [
      "Write tests for API wrapper functions",
      "Create getQASettings wrapper",
      "Create updateQASettings wrapper",
      "Create getTaskQA wrapper",
      "Create getQAResults wrapper",
      "Create retryQA wrapper",
      "Create skipQA wrapper",
      "Export from api/qa.ts",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create qaStore with Zustand",
    "steps": [
      "Write tests for store actions",
      "Create qaStore with settings state",
      "Add taskQA map for per-task QA data",
      "Implement loadSettings action",
      "Implement updateSettings action",
      "Implement loadTaskQA action",
      "Implement updateTaskQA action",
      "Use immer middleware for immutable updates",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create useQA hook",
    "steps": [
      "Write tests for hook behavior",
      "Create useQASettings hook (reads/updates global settings)",
      "Create useTaskQA hook (reads QA data for specific task)",
      "Create useQAResults hook (reads results with polling for active tests)",
      "Handle loading and error states",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskQABadge component",
    "steps": [
      "Write unit tests with React Testing Library",
      "Create TaskQABadge component",
      "Implement status-based color mapping",
      "Show only if task.needs_qa is true",
      "Display abbreviated status text",
      "Style with Tailwind classes (no inline styles)",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskDetailQAPanel component",
    "steps": [
      "Write unit tests for panel rendering",
      "Create tabbed panel component",
      "Implement Acceptance Criteria tab with checkmarks",
      "Implement Test Results tab with pass/fail icons",
      "Implement Screenshots tab with thumbnail gallery",
      "Add lightbox for full-size screenshot viewing",
      "Show failure details with expected vs actual",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create QASettingsPanel component",
    "steps": [
      "Write unit tests for settings panel",
      "Create settings panel with all QA toggles",
      "Add global QA toggle",
      "Add auto-QA checkboxes (UI tasks, API tasks)",
      "Add QA phases toggles (prep, evaluation, browser testing)",
      "Add browser testing URL input",
      "Add start command input",
      "Add wait time number input",
      "Wire to qaStore for state management",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Add QA toggle to task creation form",
    "steps": [
      "Write tests for QA checkbox in form",
      "Add 'Enable QA for this task' checkbox to task form",
      "Wire checkbox to form state",
      "Submit needs_qa with task creation",
      "Show info text explaining what QA does",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Integrate TaskQABadge with TaskCard",
    "steps": [
      "Update TaskCard tests to include QA badge",
      "Import TaskQABadge into TaskCard",
      "Conditionally render badge based on task.needs_qa",
      "Position badge appropriately in card layout",
      "Verify badge updates when QA status changes",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create QA event handlers",
    "steps": [
      "Write tests for event handling",
      "Create useQAEvents hook",
      "Handle qa_prep_started event",
      "Handle qa_prep_completed event",
      "Handle qa_testing_started event",
      "Handle qa_passed event",
      "Handle qa_failed event",
      "Update qaStore on events",
      "Run npm test"
    ],
    "passes": true
  },
  {
    "category": "integration",
    "description": "Integration test: QA Prep runs in parallel with execution",
    "steps": [
      "Create integration test with MockAgenticClient",
      "Start task with QA enabled",
      "Verify QA Prep agent spawned as background task",
      "Verify worker agent also spawned (parallel)",
      "Complete worker task",
      "Verify state waits for QA prep if not done",
      "Complete QA prep",
      "Verify transition to QA_REFINING",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: QA Testing flow with pass",
    "steps": [
      "Create integration test with mock responses",
      "Setup task in QA_REFINING state with prep data",
      "Mock QA executor agent for refinement",
      "Verify refined_test_steps stored",
      "Transition to QA_TESTING",
      "Mock successful test execution",
      "Verify QA_PASSED state reached",
      "Verify auto-transition to PENDING_REVIEW",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: QA Testing flow with failure",
    "steps": [
      "Create integration test with mock failures",
      "Setup task in QA_TESTING state",
      "Mock agent returning failed test results",
      "Verify QA_FAILED state reached",
      "Verify notification sent",
      "Test skip_qa command bypasses to review",
      "Test retry triggers re-test",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: End-to-end QA UI flow",
    "steps": [
      "Create end-to-end test with test data",
      "Create task with needs_qa=true",
      "Verify TaskQABadge shows on card",
      "Simulate QA events",
      "Verify badge updates through states",
      "Open task detail, verify QA panel renders",
      "Mock QA results, verify display",
      "Run npm test"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Add cost-optimized test prompts for QA agents",
    "steps": [
      "Create test_prompts module for QA",
      "Add QA_PREP_TEST_PROMPT (minimal echo)",
      "Add QA_REFINE_TEST_PROMPT (minimal echo)",
      "Add QA_TEST_TEST_PROMPT (minimal echo)",
      "Document expected responses",
      "Verify ~98% cost savings vs real prompts",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Visual verification of QA UI components",
    "steps": [
      "Start dev server",
      "Open browser with agent-browser",
      "Navigate to task with QA enabled",
      "Capture screenshot of TaskQABadge states",
      "Open task detail panel",
      "Capture screenshot of QA panel",
      "Navigate to settings",
      "Capture screenshot of QA settings",
      "Verify no anti-AI-slop violations",
      "Save screenshots to screenshots/"
    ],
    "passes": false
  }
]
```
