# RalphX Agent Catalog

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

Complete catalog of the 19 live agent definitions in `config/ralphx.yaml`. Canonical prompt/config source lives under `agents/`; `config/ralphx.yaml` remains the shared runtime/tool wiring layer for the fields that have not been extracted yet, even though execution may resolve to Claude or Codex depending on lane settings and harness availability.

---

## Agent Summary Table

| # | Agent Name | Model | Category | Purpose | Can Write Files? |
|---|-----------|-------|----------|---------|-----------------|
| 1 | ralphx-ideation | opus | Ideation | Facilitates ideation sessions → task proposals | No |
| 2 | ralphx-ideation-readonly | sonnet | Ideation | Read-only assistant for accepted sessions | No |
| 3 | ralphx-utility-session-namer | haiku | Ideation (utility) | Generates 2-word session titles | No (MCP only) |
| 4 | ralphx-chat-task | sonnet | Chat | Task-scoped conversational assistant | No |
| 5 | ralphx-chat-project | sonnet | Chat | Project-level conversational assistant | No |
| 6 | ralphx-review-chat | sonnet | Review | Interactive review discussion with user | No |
| 7 | ralphx-review-history | sonnet | Review | Read-only historical review discussion | No |
| 8 | ralphx-execution-worker | sonnet | Execution | Orchestrates task implementation, delegates to coders | Yes |
| 9 | ralphx-execution-coder | sonnet | Execution | Focused code implementation (worker's sub-agent) | Yes |
| 10 | ralphx-execution-reviewer | sonnet | Review | Automated code review with structured issues | No |
| 11 | ralphx-qa-prep | sonnet | QA | Generates acceptance criteria + test steps | No |
| 12 | ralphx-qa-executor | sonnet | QA | Executes browser-based QA tests | Yes |
| 13 | ralphx-execution-orchestrator | opus | Orchestration | Plans and coordinates complex multi-step tasks | Yes |
| 15 | ralphx-research-deep-researcher | opus | Research | Conducts thorough multi-source research | Yes (Write only) |
| 16 | ralphx-project-analyzer | haiku | Infrastructure | Scans project for build systems, generates validation commands | No |
| 17 | ralphx-execution-merger | opus | Git | Resolves merge conflicts that programmatic merge couldn't handle | Yes (Edit only) |
| 18 | ralphx-memory-maintainer | haiku | Memory | Ingests rule files, deduplicates, maintains memory DB | Yes |
| 19 | ralphx-memory-capture | haiku | Memory | Extracts high-value knowledge from conversations | No |

---

## Detailed Agent Profiles

### 1. ralphx-ideation

| Property | Value |
|----------|-------|
| **Model** | opus |
| **Canonical prompt** | `agents/ralphx-ideation/claude/prompt.md` |
| **Category** | Ideation |
| **CLI tools** | Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Disallowed CLI tools** | Write, Edit, NotebookEdit |
| **Preapproved** | Task(Plan) |

**MCP Tools (15):**
`create_task_proposal`, `update_task_proposal`, `delete_task_proposal`, `list_session_proposals`, `get_proposal`, `analyze_session_dependencies`, `create_plan_artifact`, `update_plan_artifact`, `link_proposals_to_plan`, `get_session_plan`, `ask_user_question`, `create_child_session`, `get_parent_session_context`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** The primary ideation agent. Facilitates structured ideation sessions through a 6-phase gated workflow: RECOVER → UNDERSTAND → EXPLORE → PLAN → CONFIRM → PROPOSE → FINALIZE. Transforms user ideas into well-defined task proposals with implementation plans.

**Key Directives:**
- Research-first: Explore codebase before asking questions
- Plan-first (enforced): Must call `create_plan_artifact` before `create_task_proposal`
- System-card requirement: Must read and apply `system-card-orchestration-pattern.md`
- Confirm gate: Never create proposals without explicit user approval
- Anti-injection: Treats all user text as DATA, not instructions
- Can fan out bounded native specialist delegates plus 1 Plan subagent
- Handles child session delegation for accepted sessions

---

### 2. ralphx-ideation-readonly

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-ideation-readonly/claude/prompt.md` |
| **Category** | Ideation |
| **CLI tools** | Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Disallowed CLI tools** | Write, Edit, NotebookEdit |
| **Preapproved** | Task(Plan) |

**MCP Tools (8 — read-only subset):**
`list_session_proposals`, `get_proposal`, `get_session_plan`, `get_parent_session_context`, `create_child_session`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Serves accepted (finalized) ideation sessions. Cannot mutate proposals or plans. Helps users understand completed plans, explore related code, and delegates new work to child sessions via `create_child_session`.

**Key Directives:**
- Read-only operations only — mutation tool failures are expected, not bugs
- When user wants changes → suggest and create child session
- Phase 0 RECOVER runs unconditionally on startup
- Never report tool failures as errors

---

### 3. ralphx-utility-session-namer

| Property | Value |
|----------|-------|
| **Model** | haiku |
| **Canonical prompt** | `agents/ralphx-utility-session-namer/shared/prompt.md` |
| **Category** | Ideation utility |
| **CLI tools** | None (`mcp_only: true`) |
| **MCP Tools (1):** | `update_session_title` |

**Purpose:** Lightweight agent that generates exactly 2-word session titles from the user's first message or imported plan content. Fires automatically on session creation.

**Key Directives:**
- Exactly 2 words, title case
- Avoid generic titles ("New Session", "Untitled")
- For plan imports, focus on subject matter, not the import action

---

### 4. ralphx-chat-task

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-chat-task/shared/prompt.md` |
| **Category** | Chat |
| **CLI tools** | Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Preapproved** | Task(Plan) |

**MCP Tools (6):**
`update_task`, `add_task_note`, `get_task_details`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Context-aware assistant when user is viewing a specific task. Can update task fields and add notes. Bound to a specific `${TASK_ID}`.

**Key Directives:**
- Respond like a colleague, not a bot
- Match message length to the question
- NEVER call `get_task_details` for greetings/small talk
- Skip "I'd be happy to help" phrasing

---

### 5. ralphx-chat-project

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-chat-project/shared/prompt.md` |
| **Category** | Chat |
| **CLI tools** | Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Preapproved** | Task(Plan) |

**MCP Tools (6):**
`suggest_task`, `list_tasks`, `search_memories`, `get_memory`, `get_memories_for_paths`, `get_conversation_transcript`

**Purpose:** General project-level assistant. Answers project questions, suggests tasks, explores codebase. Used in the project-level chat panel.

---

### 6. ralphx-review-chat

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-review-chat/claude/prompt.md` |
| **Category** | Review |
| **CLI tools** | Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Preapproved** | Task(Plan) |

**MCP Tools (12):**
`approve_task`, `request_task_changes`, `get_review_notes`, `get_task_context`, `get_artifact`, `get_artifact_version`, `get_related_artifacts`, `search_project_artifacts`, `get_task_steps`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Interactive review discussion agent. Helps users understand AI review findings and take action (approve or request changes). Spawned when task is in `review_passed` status.

**Key Directives:**
- Conversational discussion, not a form
- Help users decide: summarize findings, explain implications
- Never act without explicit consent for approve/request_changes
- Execute user's decision immediately once confirmed

---

### 7. ralphx-review-history

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-review-history/shared/prompt.md` |
| **Category** | Review |
| **CLI tools** | Read, Grep, Glob, Task |
| **Preapproved** | Task(Plan) |

**MCP Tools (14):**
`get_review_notes`, `get_task_context`, `get_task_issues`, `get_task_steps`, `get_step_progress`, `get_issue_progress`, `get_artifact`, `get_artifact_version`, `get_related_artifacts`, `search_project_artifacts`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Read-only retrospective view of completed reviews. Helps users understand what happened during review cycles — what was found, how issues were resolved, why the reviewer approved. No mutation tools.

---

### 8. ralphx-execution-worker

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-execution-worker/claude/prompt.md` |
| **Category** | Execution |
| **CLI tools** | Read, Write, Edit, Bash, Grep, Glob, WebFetch, WebSearch, Skill, Task |
| **All preapproved** | Read, Grep, Glob, WebFetch, WebSearch, Skill, Write, Edit, Bash, Task, Task(Plan) |

**MCP Tools (20):**
`start_step`, `complete_step`, `skip_step`, `fail_step`, `add_step`, `get_step_progress`, `get_step_context`, `get_sub_steps`, `get_task_context`, `get_artifact`, `get_artifact_version`, `get_related_artifacts`, `search_project_artifacts`, `get_review_notes`, `get_task_steps`, `get_task_issues`, `mark_issue_in_progress`, `mark_issue_addressed`, `get_project_analysis`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** The primary task execution agent. Orchestrates implementation by reading the system-card, decomposing work into sub-scopes, and delegating to parallel `ralphx-execution-coder` instances. Handles step/issue tracking, wave-gated validation, and re-execution after review feedback.

**Key Directives:**
- CRITICAL: Task-scoped — only execute YOUR task, not the whole plan
- System-card + delegation requirement (MANDATORY): Read `system-card-worker-execution-pattern.md`
- Parallel orchestration: Up to 3 concurrent coders per wave
- Parallel dispatch: Multiple Task calls in SINGLE response = parallel execution
- Sub-step dispatch pattern: Create sub-steps with `scope_context` for each coder
- Pre-completion validation: Run ALL validate commands before completing
- Re-execution: Fetch review notes + issues, prioritize by severity, track issue progress

---

### 9. ralphx-execution-coder

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-execution-coder/claude/prompt.md` |
| **Category** | Execution |
| **CLI tools** | Read, Write, Edit, Bash, Grep, Glob, WebFetch, WebSearch, Skill, Task |
| **All preapproved** | Read, Grep, Glob, WebFetch, WebSearch, Skill, Write, Edit, Bash, Task, Task(Plan) |

**MCP Tools (18):**
`start_step`, `complete_step`, `skip_step`, `fail_step`, `add_step`, `get_step_progress`, `get_step_context`, `get_task_context`, `get_artifact`, `get_artifact_version`, `get_related_artifacts`, `search_project_artifacts`, `get_review_notes`, `get_task_steps`, `get_task_issues`, `mark_issue_in_progress`, `mark_issue_addressed`, `get_project_analysis`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Focused developer agent dispatched by `ralphx-execution-worker`. Executes a single task or scoped sub-task. When dispatched with a sub-step ID, calls `get_step_context` first to get strict scope boundaries.

**Key Directives:**
- Task-scoped: Execute ONLY work within assigned scope
- If dispatched with STRICT SCOPE from worker, that scope is absolute
- Step 0: `get_step_context` if dispatched with sub-step ID
- TDD: Write tests before implementation
- Pre-completion validation mandatory
- Re-execution workflow identical to worker

**Difference from Worker:** Coder does NOT have `get_sub_steps` (cannot orchestrate sub-coders). Worker orchestrates; coder executes.

---

### 10. ralphx-execution-reviewer

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-execution-reviewer/claude/prompt.md` |
| **Category** | Review |
| **CLI tools** | Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Preapproved** | Bash |

**MCP Tools (14):**
`complete_review`, `get_task_context`, `get_artifact`, `get_artifact_version`, `get_related_artifacts`, `search_project_artifacts`, `get_review_notes`, `get_task_steps`, `get_task_issues`, `get_step_progress`, `get_issue_progress`, `get_project_analysis`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Automated code review agent. Reviews code quality, test coverage, security, and performance. MUST always call `complete_review` before exiting.

**Key Directives:**
- CRITICAL: MUST call `complete_review` — no exceptions
- Outcomes: `approved`, `needs_changes` (requires `issues[]`), `escalate` (requires `escalation_reason`)
- Re-review workflow: Fetch prior issues → check resolution → verify fixes → check regressions
- Structured issues with severity, step linkage, file paths, code snippets
- Uses `get_project_analysis` for validation commands

---

### 11. ralphx-qa-prep

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-qa-prep/shared/prompt.md` |
| **Category** | QA |
| **CLI tools** | Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Disallowed CLI tools** | Write, Edit, Bash, NotebookEdit |
| **Preapproved** | Task(Plan) |
| **MCP Tools** | None |

**Purpose:** Read-only QA preparation agent. Analyzes task specs and generates testable acceptance criteria with agent-browser test commands. Outputs structured JSON with criteria types (visual, behavior, data, accessibility).

---

### 12. ralphx-qa-executor

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Canonical prompt** | `agents/ralphx-qa-executor/shared/prompt.md` |
| **Category** | QA |
| **CLI tools** | Read, Write, Edit, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Preapproved** | Write, Edit, Bash, Task(Plan) |
| **MCP Tools** | None |

**Purpose:** Executes browser-based QA tests using `agent-browser`. Two phases: (2A) Refinement — analyzes git diff to update test steps; (2B) Testing — executes tests, captures screenshots, reports pass/fail results.

---

### 13. ralphx-execution-orchestrator

| Property | Value |
|----------|-------|
| **Model** | opus |
| **Canonical prompt** | `agents/ralphx-execution-orchestrator/shared/prompt.md` |
| **Category** | Orchestration |
| **CLI tools** | Read, Write, Edit, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Preapproved** | Write, Edit, Bash, Task |

**MCP Tools (3):**
`search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** General-purpose orchestrator for complex multi-step tasks. Decomposes work into atomic subtasks, orders by dependencies, and delegates to specialized agents (worker, reviewer, deep-researcher).

---

### 15. ralphx-research-deep-researcher

| Property | Value |
|----------|-------|
| **Model** | opus |
| **Canonical prompt** | `agents/ralphx-research-deep-researcher/shared/prompt.md` |
| **Category** | Research |
| **CLI tools** | Read, Write, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Preapproved** | Write, WebFetch, WebSearch, Task(Plan) |

**MCP Tools (3):**
`search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Thorough research agent with configurable depth presets: quick-scan (10 iterations), standard (50), deep-dive (200), exhaustive (500). Verifies information from multiple sources, tracks provenance, distinguishes facts from opinions.

---

### 16. ralphx-project-analyzer

| Property | Value |
|----------|-------|
| **Model** | haiku |
| **Canonical prompt** | `agents/ralphx-project-analyzer/shared/prompt.md` |
| **Category** | Infrastructure |
| **CLI tools** | Read, Glob, Bash, Grep |
| **Preapproved** | Read, Glob, Bash, Grep |

**MCP Tools (2):**
`save_project_analysis`, `get_project_analysis`

**Purpose:** Scans project directory to detect build systems (Node.js, Rust, Python, Go, Maven, Gradle) and generates path-scoped install/validate/worktree_setup commands. Results consumed by worker, coder, reviewer, and merger agents.

**Template Variables:** `{project_root}`, `{worktree_path}`, `{task_branch}`

---

### 17. ralphx-execution-merger

| Property | Value |
|----------|-------|
| **Model** | opus |
| **Canonical prompt** | `agents/ralphx-execution-merger/claude/prompt.md` |
| **Category** | Git |
| **CLI tools** | Read, Edit, Grep, Glob, Bash, WebFetch, WebSearch, Skill, Task |
| **Preapproved** | Read, Edit, Bash |

**MCP Tools (8):**
`complete_merge`, `report_conflict`, `report_incomplete`, `get_merge_target`, `get_task_context`, `get_project_analysis`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Resolves git merge conflicts that programmatic rebase couldn't handle. Also handles validation recovery mode (post-merge build failures).

**Key Directives:**
- Step 1: `get_merge_target` to get correct source/target branches
- Merge target may be a plan feature branch, NOT always main
- Auto-detection: System checks git state on exit for completion
- `complete_merge` is optional (backwards compatible)
- MUST call `report_conflict` if cannot resolve
- Validation recovery mode: Fix build errors when merge succeeded but validation failed

---

### 18. ralphx-memory-maintainer

| Property | Value |
|----------|-------|
| **Model** | haiku |
| **Canonical prompt** | `agents/ralphx-memory-maintainer/shared/prompt.md` |
| **Category** | Memory |
| **CLI tools** | Read, Write, Edit, Grep, Glob, Bash, WebFetch, WebSearch, Skill |
| **All preapproved** | Read, Grep, Glob, WebFetch, WebSearch, Bash, Write, Edit |

**MCP Tools (9):**
`search_memories`, `get_memory`, `get_memories_for_paths`, `get_conversation_transcript`, `upsert_memories`, `mark_memory_obsolete`, `refresh_memory_rule_index`, `ingest_rule_file`, `rebuild_archive_snapshots`

**Purpose:** Background agent that maintains the project memory system. Ingests `.claude/rules/` files, parses into semantic chunks, classifies into buckets (architecture_patterns, implementation_discoveries, operational_playbooks), deduplicates, and maintains archive snapshots.

**Workflow:** Detection → Parsing → Classification → Database upsert → Rule file rewrite → Archive snapshots

---

### 19. ralphx-memory-capture

| Property | Value |
|----------|-------|
| **Model** | haiku |
| **Canonical prompt** | `agents/ralphx-memory-capture/shared/prompt.md` |
| **Category** | Memory |
| **CLI tools** | Read, Write, Edit, Grep, Glob, Bash, WebFetch, WebSearch, Skill |
| **All preapproved** | Read, Grep, Glob, WebFetch, WebSearch, Bash, Write, Edit |

**MCP Tools (5):**
`get_conversation_transcript`, `upsert_memories`, `search_memories`, `get_memory`, `get_memories_for_paths`

**Purpose:** Background agent that extracts high-value knowledge from completed agent conversations. Applies strict quality gates (non-obvious, reusable, time-saving >15min, novel, actionable). Context-specific behavior varies for planning vs execution vs review vs merge sessions.

**Quality Targets:** >90% precision, 80% recall, 60-80% no-capture rate, <5% duplicate rate

---

## Tool Scoping Architecture

### Three-Layer Allowlist

Every MCP tool must be registered in three places (see `agent-mcp-tools.md`):

| Layer | File | Controls |
|-------|------|----------|
| 1. Rust spawn config | `src-tauri/src/infrastructure/agents/claude/agent_config/mod.rs` | `--allowedTools` flag at spawn |
| 2. MCP server filter | `plugins/app/ralphx-mcp-server/src/tools.ts` | Server-side tool filtering |
| 3. Canonical prompt contract | `agents/<name>/...` + generated Claude frontmatter | Subagent spawning + docs |

### Shared Tool Sets

Defined in `config/ralphx.yaml`:
```yaml
tool_sets:
  base_tools: [Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill]
```

Most agents extend `base_tools` and add Write, Edit, or Task as needed.

### MCP Tool Distribution

| MCP Tool | Agents |
|----------|--------|
| `get_task_context` | worker, coder, reviewer, review-chat, review-history, merger |
| `start_step` / `complete_step` / `skip_step` / `fail_step` | worker, coder |
| `add_step` | worker, coder |
| `get_step_progress` | worker, coder, reviewer, review-history |
| `get_step_context` | worker, coder |
| `get_sub_steps` | worker only |
| `complete_review` | reviewer only |
| `approve_task` / `request_task_changes` | review-chat only |
| `complete_merge` / `report_conflict` / `report_incomplete` | merger only |
| `get_merge_target` | merger only |
| `create_task_proposal` / `update_task_proposal` / `delete_task_proposal` | ralphx-ideation only |
| `create_plan_artifact` / `update_plan_artifact` | ralphx-ideation only |
| `list_session_proposals` / `get_proposal` | ralphx-ideation, ralphx-ideation-readonly |
| `get_session_plan` | ralphx-ideation, ralphx-ideation-readonly |
| `create_child_session` | ralphx-ideation, ralphx-ideation-readonly |
| `analyze_session_dependencies` | ralphx-ideation only |
| `update_session_title` | ralphx-utility-session-namer only |
| `update_task` / `add_task_note` / `get_task_details` | ralphx-chat-task only |
| `suggest_task` / `list_tasks` | ralphx-chat-project only |
| `save_project_analysis` | ralphx-project-analyzer only |
| `get_project_analysis` | worker, coder, reviewer, merger, ralphx-project-analyzer |
| `upsert_memories` | ralphx-memory-maintainer, ralphx-memory-capture |
| `mark_memory_obsolete` | ralphx-memory-maintainer, ralphx-memory-capture |
| `search_memories` / `get_memory` / `get_memories_for_paths` | 12 agents (all except ralphx-utility-session-namer, qa-prep, qa-executor, ralphx-project-analyzer) |
| `get_conversation_transcript` | ralphx-memory-maintainer, ralphx-memory-capture, ralphx-chat-project |

---

## Workflow Diagrams

### Primary Task Lifecycle Flow

```
                    ┌──────────────────┐
                    │   USER / UI      │
                    └────────┬─────────┘
                             │ idea
                             ▼
            ┌─────────────────────────────────┐
            │    IDEATION PHASE               │
            │                                 │
            │  ralphx-ideation (opus)   │
            │    ├── ralphx-utility-session-namer (haiku)    │  ← auto-fires on session create
            │    ├── delegate_start × 3      │  ← bounded specialist research
            │    ├── Task(Plan) × 1          │  ← architectural design
            │         (haiku)                │
            │                                 │
            │  Outputs: Plan artifact +      │
            │           Task proposals        │
            └────────────┬────────────────────┘
                         │ accept proposals → tasks on Kanban
                         ▼
            ┌─────────────────────────────────┐
            │    EXECUTION PHASE              │
            │                                 │
            │  ralphx-project-analyzer (haiku)       │  ← scans build systems first
            │         │                       │
            │         ▼                       │
            │  ralphx-execution-worker (sonnet)         │  ← orchestrates implementation
            │    ├── ralphx-execution-coder (sonnet)    │
            │    ├── ralphx-execution-coder (sonnet)    │  ← up to 3 parallel coders
            │    └── ralphx-execution-coder (sonnet)    │
            │                                 │
            └────────────┬────────────────────┘
                         │ execution complete
                         ▼
            ┌─────────────────────────────────┐
            │    REVIEW PHASE                 │
            │                                 │
            │  ralphx-execution-reviewer (sonnet)       │  ← automated code review
            │    │                            │
            │    ├── approved ──────────────┐ │
            │    │                          │ │
            │    ├── needs_changes ─────┐   │ │
            │    │                      │   │ │
            │    └── escalate ──► USER  │   │ │
            │                      │    │   │ │
            │                      ▼    │   │ │
            │  ralphx-review-chat ◄─────┘   │ │  ← human reviews AI feedback
            │    (sonnet)                   │ │
            │    ├── approve_task ──────────┤ │
            │    └── request_task_changes   │ │
            │         │                     │ │
            │         ▼                     │ │
            │    Back to EXECUTION ─────────┘ │  ← revision cycle
            │                                 │
            │  ralphx-review-history          │  ← retrospective (read-only)
            │    (sonnet)                     │
            └────────────┬────────────────────┘
                         │ approved
                         ▼
            ┌─────────────────────────────────┐
            │    QA PHASE (optional)          │
            │                                 │
            │  ralphx-qa-prep (sonnet)        │  ← generates test criteria
            │         │                       │
            │         ▼                       │
            │  ralphx-qa-executor (sonnet)    │  ← executes browser tests
            │                                 │
            └────────────┬────────────────────┘
                         │ QA passed
                         ▼
            ┌─────────────────────────────────┐
            │    MERGE PHASE                  │
            │                                 │
            │  Programmatic rebase+merge      │  ← automatic attempt first
            │    │                            │
            │    ├── success → DONE           │
            │    │                            │
            │    └── conflict ──►             │
            │       ralphx-execution-merger (opus)      │  ← resolves conflicts
            │         │                       │
            │         ├── resolved → DONE     │
            │         └── report_conflict     │
            │              → USER             │
            └─────────────────────────────────┘
```

### Chat Agents (User-Facing)

```
USER in RalphX UI
    │
    ├── Viewing specific task ──► ralphx-chat-task (sonnet)
    │     Can: update_task, add_task_note, get_task_details
    │
    ├── Project-level chat ──► ralphx-chat-project (sonnet)
    │     Can: suggest_task, list_tasks
    │
    ├── Active ideation session ──► ralphx-ideation (opus)
    │     Can: full CRUD on proposals/plans
    │
    └── Accepted ideation session ──► ralphx-ideation-readonly (sonnet)
          Can: read-only + create_child_session
```

### Memory System (Background)

```
Any agent conversation ends
    │
    ▼
ralphx-memory-capture (haiku)         ← extracts high-value knowledge
    │
    ▼
ralphx-memory-maintainer (haiku)      ← ingests rules, deduplicates, archives
    │
    ▼
Memory DB (SQLite)
    │
    ▼
search_memories / get_memory   ← consumed by 13 agents at runtime
```

### Supporting Agents (On-Demand)

```
ralphx-execution-orchestrator (opus)     ← general complex task coordination
ralphx-research-deep-researcher (opus)  ← thorough multi-source research
ralphx-project-analyzer (haiku)       ← scans build systems for validation commands
```

---

## Model Distribution

| Model | Count | Agents |
|-------|-------|--------|
| **opus** | 4 | ralphx-ideation, ralphx-execution-orchestrator, ralphx-research-deep-researcher, ralphx-execution-merger |
| **sonnet** | 10 | ralphx-ideation-readonly, ralphx-chat-task, ralphx-chat-project, ralphx-review-chat, ralphx-review-history, ralphx-execution-worker, ralphx-execution-coder, ralphx-execution-reviewer, ralphx-qa-prep, ralphx-qa-executor |
| **haiku** | 4 | ralphx-utility-session-namer, ralphx-project-analyzer, ralphx-memory-maintainer, ralphx-memory-capture |

**Pattern:** opus for high-stakes decisions (ideation, orchestration, merge, research), sonnet for implementation/review, haiku for lightweight utilities.

---

## Agent Spawning

All agents are spawned by the Rust backend via the Claude CLI:
```
Claude Agent → --model <model> --append-system-prompt-file <prompt.md>
             → --tools <cli_tools> --allowedTools <mcp_tools>
             → --mcp-config <ralphx-mcp-server> --permission-mode default
```

Configuration source: `config/ralphx.yaml` → parsed at compile time → `agent_config/mod.rs`

MCP Architecture: `Claude Agent → MCP Protocol → ralphx-mcp-server (TS) → HTTP :3847 → Tauri Backend`
