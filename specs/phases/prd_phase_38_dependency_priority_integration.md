# RalphX - Phase 38: Dependency Graph & Priority Assessment Integration

## Overview

Complete the dependency/priority system by wiring up priority assessment commands to actually compute scores, implementing AI-based dependency suggestions (following the session-namer pattern), and integrating dependency/priority indicators into the session view UI. This phase extends the foundation laid in earlier ideation phases with real-time computation, automated dependency analysis, and enhanced agent proactivity.

**Reference Plan:**
- `specs/plans/dependency_graph_priority_integration.md` - Complete implementation details for priority assessment, AI suggestions, UI integration, chat agent tools, and agent prompt enhancements

## Goals

1. Wire up `assess_proposal_priority` and `assess_all_priorities` to actually compute scores via `PriorityService`
2. Implement AI-based dependency suggestions using the session-namer spawn pattern with auto-apply
3. Add dependency count badges and critical path indicators to ProposalCard UI
4. Extend chat agent with `analyze_session_dependencies` tool for intelligent recommendations
5. Enhance orchestrator-ideation agent with proactive behaviors

## Dependencies

### Phase 37 (Proposal Query Tools) - Required

| Dependency | Why Needed |
|------------|------------|
| `list_session_proposals` MCP tool | Part 5 proactive behaviors need to query existing proposals |
| `get_proposal` MCP tool | Needed for proposal-aware agent recommendations |
| HTTP dispatch pattern | Same pattern used for new `analyze_session_dependencies` tool |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/dependency_graph_priority_integration.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol:**

Reference: `.claude/rules/commit-lock.md`

1. Establish project root: `PROJECT_ROOT="$(git rev-parse --show-toplevel)"`
2. Acquire lock before `git add` (see commit-lock.md § Protocol)
3. Stage and commit using `git -C "$PROJECT_ROOT"`
4. Release lock after commit: `rm -f "$PROJECT_ROOT/.commit-lock"`

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/dependency_graph_priority_integration.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Wire up priority assessment commands to actually compute scores",
    "plan_section": "Part 1: Wire Up Priority Assessment Commands",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(ideation): wire up priority assessment commands",
    "steps": [
      "Read specs/plans/dependency_graph_priority_integration.md section 'Part 1'",
      "Fix assess_proposal_priority: get proposal, build graph via DependencyService, call PriorityService::assess_priority, store result, emit event",
      "Fix assess_all_priorities: call PriorityService::assess_and_update_all_priorities, emit session:priorities_assessed",
      "Add event emissions: proposal:priority_assessed, session:priorities_assessed, dependency:added, dependency:removed",
      "Add frontend event handlers in useIdeationEvents.ts for TanStack Query invalidation",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): wire up priority assessment commands"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add dependency badges and critical path indicators to ProposalCard",
    "plan_section": "Part 3: UI Integration (3.1-3.3)",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(ideation): add dependency badges and critical path indicators",
    "steps": [
      "Read specs/plans/dependency_graph_priority_integration.md section 'Part 3' (3.1-3.3)",
      "Add props to ProposalCard: dependsOnCount, blocksCount, isOnCriticalPath",
      "Display compact badges in tags row: ←N for dependsOn, →M in orange for blocks",
      "Wire ProposalList to fetch dependency graph via useDependencyGraph(sessionId)",
      "Build counts map from graph nodes and pass to each ProposalCard",
      "Add critical path indicator: orange bottom border, tooltip on priority badge",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add dependency badges and critical path indicators"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "agent",
    "description": "Create dependency-suggester agent and MCP tool for AI-based suggestions",
    "plan_section": "Part 2: AI-Based Dependency Suggestions (2.1-2.4)",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(ideation): add AI-based dependency suggestions",
    "steps": [
      "Read specs/plans/dependency_graph_priority_integration.md section 'Part 2' (2.1-2.4)",
      "Create agent definition at ralphx-plugin/agents/dependency-suggester.md",
      "Add apply_proposal_dependencies tool definition in tools.ts",
      "Add MCP handler in index.ts gated by RALPHX_AGENT_TYPE=dependency-suggester",
      "Create backend command apply_dependency_suggestions: clear existing, add new (skip cycles), emit event",
      "Remove add_proposal_dependency from TOOL_ALLOWLIST for orchestrator-ideation (keep definition)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(ideation): add AI-based dependency suggestions"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Add spawn command and auto-trigger logic for dependency suggester",
    "plan_section": "Part 2: AI-Based Dependency Suggestions (2.5-2.7)",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(ideation): add dependency suggester spawn and auto-trigger",
    "steps": [
      "Read specs/plans/dependency_graph_priority_integration.md section 'Part 2' (2.5-2.7)",
      "Add spawn_dependency_suggester command: get proposals, build prompt, emit analysis_started, spawn agent",
      "Add auto-trigger logic after create/update/remove proposal (when count >= 2)",
      "Implement debounce (2s delay) to avoid rapid re-triggers",
      "Add spawnDependencySuggester API call in src/api/ideation.ts",
      "Add event listeners in useIdeationEvents.ts for analysis_started and suggestions_applied",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add dependency suggester spawn and auto-trigger"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add loading states and manual re-trigger button for dependency analysis",
    "plan_section": "Part 3: UI Integration (3.4-3.5)",
    "blocking": [6],
    "blockedBy": [4],
    "atomic_commit": "feat(ideation): add dependency analysis loading states",
    "steps": [
      "Read specs/plans/dependency_graph_priority_integration.md section 'Part 3' (3.4-3.5)",
      "Add isAnalyzing state to ProposalList with event listeners",
      "Show spinner in header when analyzing: 'Analyzing dependencies...'",
      "Show toast on suggestions_applied with count",
      "Add manual re-trigger button with network/link icon in proposal list header",
      "Disable button while isAnalyzing",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add dependency analysis loading states"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "mcp",
    "description": "Add analyze_session_dependencies tool for chat agent integration",
    "plan_section": "Part 4: Chat Agent Integration",
    "blocking": [7],
    "blockedBy": [5],
    "atomic_commit": "feat(mcp): add analyze_session_dependencies tool for chat agent",
    "steps": [
      "Read specs/plans/dependency_graph_priority_integration.md section 'Part 4'",
      "Add analyze_session_dependencies tool definition to tools.ts",
      "Add to TOOL_ALLOWLIST for orchestrator-ideation",
      "Add analyzing_dependencies: HashSet<IdeationSessionId> to AppState",
      "Create HTTP handler that calls DependencyService::build_graph(), includes analysis_in_progress",
      "Add GET dispatch in MCP server index.ts",
      "Add route in http_server/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(mcp): add analyze_session_dependencies tool for chat agent"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "agent",
    "description": "Enhance orchestrator-ideation agent with proactive behaviors",
    "plan_section": "Part 5: Enhance Orchestrator-Ideation Agent Prompt",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "docs(plugin): enhance orchestrator-ideation agent with proactive behaviors",
    "steps": [
      "Read specs/plans/dependency_graph_priority_integration.md section 'Part 5'",
      "Add Proactive Behaviors section after Guidelines in orchestrator-ideation.md",
      "Add documentation for query tools (list_session_proposals, get_proposal) and analysis tools (analyze_session_dependencies)",
      "Add proactive examples: Plan-Proposal Sync, Dependency Analysis, Continuation",
      "Update Do Not section with passive/stopping behaviors to avoid",
      "Commit: docs(plugin): enhance orchestrator-ideation agent with proactive behaviors"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Auto-accept all AI suggestions** | Frictionless UX - no review UI needed, badges show results |
| **Replace-all dependency approach** | Each auto-run provides clean slate based on current proposals |
| **Auto-trigger at 2+ proposals** | Runs on create/update/delete when enough proposals exist |
| **Haiku model for dependency suggester** | Fast, cheap, sufficient for semantic analysis |
| **Session-namer spawn pattern** | Proven pattern for fire-and-forget agent tasks |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] assess_proposal_priority computes scores (not just returns stored values)
- [ ] assess_all_priorities calls PriorityService correctly
- [ ] apply_dependency_suggestions clears existing and adds new
- [ ] spawn_dependency_suggester validates proposal count >= 2
- [ ] Events emitted correctly for all dependency/priority changes

### Frontend - Run `npm run test`
- [ ] ProposalCard renders dependency badges correctly
- [ ] ProposalList passes counts from dependency graph
- [ ] Critical path cards have orange indicator
- [ ] Loading state shows during analysis
- [ ] Toast appears on suggestions_applied

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Create proposals → call assess_all_priorities → verify scores computed
- [ ] Create 2 proposals → verify analysis auto-triggers
- [ ] Check loading indicator shows during analysis
- [ ] Verify dependencies auto-applied, toast appears
- [ ] Verify count badges (←N →M) appear on cards with dependencies
- [ ] Verify critical path cards have orange indicator
- [ ] Test manual re-trigger button
- [ ] Verify analyze_session_dependencies tool works in chat

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Priority assessment commands actually compute (not stub)
- [ ] Dependency suggester agent spawns and applies suggestions
- [ ] ProposalCard receives and displays dependency counts
- [ ] analyze_session_dependencies HTTP handler responds correctly
- [ ] Events trigger UI updates via TanStack Query invalidation

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
