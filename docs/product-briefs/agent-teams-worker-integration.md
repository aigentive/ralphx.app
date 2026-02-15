# Product Brief: Agent Teams at Worker Level for Parallel Task Execution

**Status:** DRAFT v3
**Date:** 2026-02-15
**Author:** system-card-writer (agent-teams-research team)
**Depends on:** Agent Teams System Card (`docs/agent-teams-system-card.md`)
**Revision:** v3 — Resolves: worker teammates can document decisions via team artifacts; per-teammate cost display; consistent with ideation brief v5 decisions

---

## 1. Problem Statement

### Current State

RalphX's `ralphx-worker` agent orchestrates task implementation by delegating to `ralphx-coder` subagents. This works but has fundamental constraints:

| Constraint | Current Behavior | Impact |
|-----------|-----------------|--------|
| **Parent-child only** | Coders report results back to worker only | No peer-to-peer knowledge sharing |
| **No cross-coder awareness** | Coder A doesn't know what Coder B found | Duplicate investigation, missed integration issues |
| **Worker bottleneck** | Worker must process all results sequentially | Wave gates block on slowest coder |
| **Limited coordination** | Worker assigns scope upfront, no dynamic re-scoping | Can't adapt when coder discovers complexity |
| **No shared discovery** | Coder finds a pattern, but other coders don't learn it | Repeated mistakes across coders |

### Wave-Based Execution Today

```
Worker decomposes task → dependency graph
    ↓
Wave 1: Coder A (scope 1) + Coder B (scope 2) + Coder C (scope 3)
    ↓ All complete → Worker validates (typecheck + tests + lint)
    ↓ Gate passes
Wave 2: Coder D (scope 4) + Coder E (scope 5)
    ↓ All complete → Worker validates
    ↓ Gate passes
Worker marks task complete
```

**Pain points:**
- If Coder A discovers a shared interface change needed by Coder B, there's no way to communicate this. Worker must detect and fix the mismatch during wave validation.
- If Coder C finishes early, it can't help Coder A or B.
- Workers can't dynamically re-assign scope when complexity is uneven.

---

## 2. Proposed Enhancement

### Team-Based Worker Execution

Replace the subagent-based coder delegation with an **agent team** where coders are teammates that can communicate with each other directly while still being coordinated by the worker (team lead).

```
Worker (Team Lead, delegate mode)
    ├── Creates team + tasks from dependency graph
    ├── Spawns coder teammates
    ├── Assigns initial scopes
    │
    ├── Coder A (teammate) ←→ Coder B (teammate)
    │     ↕                      ↕
    │   Coder C (teammate) ←→ Coder D (teammate)
    │
    ├── Monitors progress via TaskList
    ├── Re-assigns work dynamically
    ├── Runs wave validation gates
    └── Synthesizes results, shuts down team
```

### Key Properties

| Property | Value |
|----------|-------|
| **Additive** | New capability alongside existing sequential worker |
| **Dynamic composition** | Worker-lead defines teammate roles, prompts, and models based on task (default) |
| **Configurable constraints** | `ralphx.yaml` can optionally constrain team composition (opt-in) |
| **Opt-in per task** | Worker decides based on task complexity |
| **MCP-compatible** | All teammates use RalphX MCP tools |
| **Git-safe** | File ownership prevents conflicts |

---

## 3. Architecture Design

### 3.1 New Agent Variant: `ralphx-worker-team`

| Property | Value |
|----------|-------|
| **Model** | sonnet |
| **Category** | Execution |
| **Based on** | `ralphx-worker` (extends with team capabilities) |
| **Additional tools** | TeamCreate, TeamDelete, SendMessage, TaskCreate, TaskUpdate, TaskGet, TaskList |
| **Permission mode** | delegate (coordination-only) |
| **MCP tools** | Same as worker + team coordination tools |
| **Team composition** | Dynamic (default) — worker-lead chooses teammate roles/models based on task analysis |

### 3.1.1 Dynamic Team Composition (Default Mode)

The worker-lead **dynamically determines** teammate roles, prompts, and models based on the specific task:

| Decision | Made By | Based On |
|----------|---------|----------|
| Number of teammates | Worker-lead | Task complexity, parallelizable scopes |
| Teammate model | Worker-lead | Scope difficulty (haiku for simple, sonnet for complex, opus for architecture) |
| Teammate prompt | Worker-lead | Specific sub-scope requirements |
| Teammate tools | Worker-lead | What the scope requires (e.g., WebSearch for research tasks) |

**Why dynamic over predefined:** Tasks vary wildly in structure. A UI-heavy task might need 2 frontend coders + 1 test writer, while an API task might need 1 backend coder + 1 integration tester. Rigid YAML configs can't anticipate this variety.

### 3.1.2 Constrained Mode (Opt-in)

For teams that want guardrails, `ralphx.yaml` can constrain what the worker-lead can spawn:

```yaml
team_constraints:
  ralphx-worker-team:
    max_teammates: 5                         # Default: 5 (consistent with ideation brief)
    allowed_models: [sonnet, haiku]          # No opus (cost control)
    allowed_agent_types: [general-purpose]    # Restrict agent types
    required_mcp_tools: [start_step, complete_step]  # All teammates must have these
    budget_limit: null                       # No budget cap by default; configurable
```

The worker-lead operates freely within these constraints. Configuration provides **boundaries**, not rigid definitions.

**System prompt additions:**
```markdown
You are a team-based worker that coordinates parallel execution.

1. Analyze the task and determine optimal team composition:
   - How many teammates? (based on parallelizable scopes)
   - What model for each? (haiku=simple, sonnet=complex, opus=architecture)
   - What tools does each need?
2. Create agent team with TeamCreate
3. Create tasks for each sub-scope with TaskCreate (include file ownership)
4. Spawn teammates with appropriate roles/models/prompts
5. Monitor progress via TaskList
6. Facilitate cross-coder communication (relay discoveries)
7. Run validation gates between waves
8. Shut down team and report results
```

### 3.2 ralphx.yaml Configuration

```yaml
agents:
  # Existing sequential worker (unchanged)
  - name: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write, Edit, Task] }
    mcp_tools: [start_step, complete_step, ...]

  # NEW: Team-based parallel worker
  - name: ralphx-worker-team
    system_prompt_file: ralphx-plugin/agents/worker-team.md
    model: sonnet
    tools: { extends: base_tools, include: [Task] }  # No Write/Edit (delegate mode)
    mcp_tools: [
      start_step, complete_step, skip_step, fail_step,
      add_step, get_step_progress, get_step_context, get_sub_steps,
      get_task_context, get_review_notes, get_task_steps,
      get_task_issues, mark_issue_in_progress, mark_issue_addressed,
      get_project_analysis, search_memories, get_memory, get_memories_for_paths,
    ]
    preapproved_cli_tools: [
      Task, Task(Explore), Task(Plan),
      # Agent team tools auto-available
    ]
    env:
      CLAUDECODE: "1"
      CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS: "1"

# Optional: Constrain what worker-lead can spawn (default: unconstrained)
team_constraints:
  ralphx-worker-team:
    max_teammates: 5                         # Default: 5
    allowed_models: [sonnet, haiku]
    required_mcp_tools: [start_step, complete_step, get_task_context]
    budget_limit: null                       # No budget cap by default
```

### 3.3 Process Mapping (Configurable)

```yaml
# New section in ralphx.yaml
processes:
  task_execution:
    default: ralphx-worker          # Sequential (current behavior)
    parallel: ralphx-worker-team    # Team-based (new)
    selection: auto                  # auto | manual | always_parallel
```

**Selection logic (`auto` mode):**

| Condition | Worker Selected |
|-----------|----------------|
| Task has ≤2 steps | `ralphx-worker` (sequential) |
| Task has >2 parallelizable steps | `ralphx-worker-team` (parallel) |
| Task flagged `parallel: true` | `ralphx-worker-team` |
| Task flagged `sequential: true` | `ralphx-worker` |
| Re-execution (fixing review issues) | `ralphx-worker` (focused fixes) |

### 3.4 Execution Flow

```
State Machine: Task → executing
    ↓
TransitionHandler::entering_executing()
    ↓
AgenticClientSpawner::spawn_agent()
    ↓ Check process mapping → select worker variant
    ↓
[If ralphx-worker-team selected]
    ↓
Worker-Team (Team Lead) starts:
    1. get_task_context() → decompose into sub-scopes
    2. Build dependency graph
    3. TeamCreate(name: "task-{task_id}")
    4. For each wave:
       a. TaskCreate for each sub-scope
          - Include: file ownership list, scope boundaries
          - Set addBlockedBy for dependencies
       b. Spawn coder teammates:
          Task(
            prompt: "Execute sub-scope: {scope_context}",
            subagent_type: "general-purpose",
            team_name: "task-{task_id}",
            name: "coder-{i}",
            model: "sonnet"
          )
       c. Monitor via TaskList + incoming messages
       d. Relay discoveries between coders via SendMessage
       e. All coders complete → run validation gate
       f. Gate fails → create fix tasks, re-assign
    5. All waves complete
    6. Final validation (typecheck + tests + lint)
    7. Shutdown all teammates
    8. TeamDelete
    9. complete_step() → report to RalphX state machine
```

---

## 4. Cross-Coder Coordination

### 4.1 File Ownership (Conflict Prevention)

Each coder teammate owns specific files. No overlapping writes.

```
TaskCreate(
  subject: "Implement user API endpoint",
  description: """
    FILE OWNERSHIP (exclusive write access):
    - src/api/users.ts
    - src/api/users.test.ts
    - src-tauri/src/http_server/handlers/users.rs

    SCOPE: Create CRUD handlers for users.
    DEPENDENCIES: none
    SHARED TYPES: src/types/user.ts (READ ONLY - owned by coder-1)
  """
)
```

### 4.2 Peer Messaging Patterns

| Pattern | When | Example |
|---------|------|---------|
| **Interface discovery** | Coder finds shared type needs change | Coder A → Coder B: "I need `UserResponse` to include `email` field" |
| **Dependency signal** | Coder finishes prerequisite | Coder A → Lead: "API types ready, coders B+C can start" |
| **Conflict warning** | Coder needs file owned by another | Coder B → Lead: "Need to modify users.rs, currently owned by Coder A" |
| **Help request** | Coder stuck on implementation detail | Coder C → Coder A: "How did you handle auth in the users endpoint?" |
| **Lead relay** | Lead relays pattern to all coders | Lead → broadcast: "Pattern: use `AppResult<T>` for all handlers" |

### 4.3 Dynamic Re-Scoping

The team lead can re-assign work mid-execution:

```
Coder A finishes early + Coder B struggling
    ↓
Lead detects via TaskList (A completed, B still in_progress)
    ↓
Lead messages Coder A: "Help Coder B with the complex query logic"
    ↓
OR Lead creates new task from B's remaining scope, assigns to A
```

---

## 5. Git Considerations

### 5.1 Branch Strategy

| Approach | Details |
|----------|---------|
| **Single branch** (recommended) | All coders work on same task branch. File ownership prevents conflicts |
| **Sub-branches** (complex) | Each coder gets sub-branch, merged by lead. Adds complexity |

**Recommended: Single task branch with file ownership.**

All coders write to the same working directory (worktree). Since file ownership is exclusive, there are no git conflicts. The commit lock protocol (`.commit-lock`) ensures atomic commits.

### 5.2 Commit Strategy

| Strategy | When |
|----------|------|
| **Per-coder commits** | Each coder commits after completing their scope |
| **Wave commits** | Lead commits after wave validation passes |
| **Final commit** | Lead makes single commit after all waves |

**Recommended: Wave commits** — Lead runs validation, then commits all changes for that wave as a single `feat:` commit.

### 5.3 Commit Lock Integration

RalphX's existing `.commit-lock` protocol applies:

```
Coder finishes work → signals Lead
Lead acquires .commit-lock
Lead commits wave changes
Lead releases .commit-lock
```

Only the lead commits (coders write files, lead commits). This prevents commit races.

---

## 6. MCP Tool Integration

### 6.1 Step Management Mapping

| Current (Sequential) | Team-Based (Parallel) |
|----------------------|----------------------|
| Worker calls `start_step()` | Lead calls `start_step()` for wave |
| Worker delegates to coder subagent | Lead creates TaskCreate + spawns teammate |
| Coder calls `get_step_context()` | Teammate calls `get_step_context()` (same) |
| Coder calls `complete_step()` | Teammate calls `complete_step()` (same) |
| Worker validates wave | Lead validates wave (same gate logic) |

### 6.2 MCP Tools per Team Role

| Tool | Team Lead | Coder Teammate |
|------|----------|---------------|
| `start_step` | ✅ | ✅ |
| `complete_step` | ✅ | ✅ |
| `get_step_context` | ✅ | ✅ |
| `get_sub_steps` | ✅ (orchestration) | ❌ (not needed) |
| `get_task_context` | ✅ | ✅ |
| `mark_issue_in_progress` | ✅ | ✅ |
| `mark_issue_addressed` | ✅ | ✅ |
| `get_project_analysis` | ✅ | ✅ |
| `search_memories` | ✅ | ✅ |

**Note:** MCP tools use stdio and work in both foreground and background modes. The limitation for background teammates is user-interactive permission prompts — which is not an issue for RalphX agents using `acceptEdits` or `--dangerously-skip-permissions`.

---

## 7. Failure Handling

### 7.1 Failure Scenarios

| Scenario | Detection | Response |
|----------|-----------|----------|
| **Coder fails (error)** | TaskList shows task failed, or teammate messages lead | Lead re-assigns task or creates fix task |
| **Coder stuck (loop)** | TeammateIdle hook, or lead checks progress | Lead messages coder with guidance, or replaces |
| **Validation gate fails** | Lead runs typecheck/tests, gets errors | Lead creates fix tasks for specific errors |
| **Git conflict** | Should not happen with file ownership | Lead mediates ownership, re-assigns files |
| **Coder exceeds max_turns** | Agent exits automatically | Lead detects via idle notification, re-spawns |

### 7.2 Rollback Strategy

| Level | Strategy |
|-------|----------|
| **Coder failure** | `git checkout -- <coder-owned-files>` to revert coder's changes |
| **Wave failure** | `git stash` entire wave, fix, then reapply |
| **Full task failure** | Task branch is isolated; just delete branch |

### 7.3 Graceful Degradation

If team execution fails repeatedly (e.g., teammates can't coordinate):

```
Worker-team fails (3 attempts)
    ↓
State machine marks task failed
    ↓
User retries → system selects ralphx-worker (sequential fallback)
    ↓
Sequential execution completes normally
```

### 7.4 Team Resume for Failed/Interrupted Worker Teams

**NOTE:** Consistent with ideation brief (Section 7.5) — team sessions support resume.

When a worker team execution is interrupted (coder crash, session expiry, app restart), the worker-lead can resume:

| State Persisted | Storage | Purpose |
|----------------|---------|---------|
| Team composition | DB: teammate names, roles, prompts, file ownership | Re-spawn coders with same scope |
| Wave progress | Existing step tracking via MCP | Which waves completed |
| File changes | Git working tree (uncommitted files survive) | Resume from last state |
| Coder findings | Team artifacts in `team-findings` bucket (if applicable) | Context for re-spawned coders |

**Resume flow:**
- Worker-lead detects missing teammates on restart
- Reads persisted team state from DB
- Re-spawns crashed coders with injected context: "You are resuming. Completed waves: [X]. Your file ownership: [Y]. Continue from where the previous coder left off."
- Since file ownership is exclusive, no conflict risk on resume

**Key difference from ideation resume:** Worker teammates produce **code artifacts** (file writes), not just research. This means resume must account for partial file changes in the working tree. The worker-lead should run a validation gate (typecheck/lint) before resuming to detect inconsistent state.

### 7.5 Artifact Model Implications for Worker Teams

**Cross-reference:** Ideation brief Section 6.4 extends the existing artifact system (NO new tables — uses existing `artifacts` table + new `team-findings` bucket + `metadata_json` for team metadata). For worker teams, the implications are:

| Aspect | Ideation Teams | Worker Teams |
|--------|---------------|-------------|
| Artifact type | Research findings, analysis docs | Code changes, implementation notes, architecture decisions |
| Read-only concern | All teammates read-only | Teammates have Write/Edit (file ownership prevents conflicts) |
| Supporting artifacts useful? | Yes — persist research for resume | **YES — workers document implementation decisions** |

**RESOLVED (v3): Worker teammates CAN create team artifacts.** Worker coders can document:
- **Architecture decisions** — "Chose X pattern over Y because..." (gives reviewers context beyond code diffs)
- **Implementation notes** — "This handler uses retry logic because the upstream API is flaky"
- **Integration observations** — "Discovered shared interface needs `email` field for Coder B's endpoint"
- **Test rationale** — "Edge case X is tested because the original issue mentioned it"

These artifacts persist indefinitely (consistent with ideation brief) and provide structured reviewer context that code comments alone cannot capture. The `create_team_artifact` MCP tool is added to the worker-team-member tool ceiling.

**MCP tool addition for worker teammates:**
```yaml
team_constraints:
  ralphx-worker-team:
    mcp_tool_ceiling:
      - get_task_context
      - start_step
      - complete_step
      - create_team_artifact   # NEW — document decisions
      - get_team_artifacts     # NEW — read team findings
```

---

## 8. UI/UX Considerations

### 8.1 Execution Panel Changes

Current `ExecutionTaskDetail` shows single-agent progress. For team execution:

| Element | Current | Team Enhancement |
|---------|---------|-----------------|
| **Progress bar** | Single step progress | Multi-track: one per coder teammate |
| **Current step** | Single current step | Active steps per teammate |
| **Status** | "Executing..." | "Team: 3 coders active, Wave 2/4" |
| **Log** | Single agent output stream | Tabbed view per teammate |

### 8.2 Running Processes Popover

| Field | Current | Team Enhancement |
|-------|---------|-----------------|
| Process list | 1 entry per task | 1 group per task, sub-entries per teammate |
| Status | "Executing step 3/8" | "Wave 2: Coder A (step 3), Coder B (step 4), Coder C (done)" |

### 8.2.1 Per-Teammate Cost Display

**Consistent with ideation brief:** Team execution shows per-teammate token usage breakdown in the execution panel. This helps users identify which coder roles consume the most tokens and whether team execution is cost-effective for the task type.

### 8.3 Intervention Capability

User should be able to:
- View each teammate's progress independently
- Send a message to specific teammate via chat panel
- Pause/stop individual teammates
- Pause/stop entire team execution
- View inter-teammate messages

### 8.4 Event Bus Integration

New events for team execution:

| Event | Data | UI Update |
|-------|------|-----------|
| `team:created` | `{taskId, teamName, memberCount}` | Show team indicator |
| `team:member_spawned` | `{taskId, memberName, scope}` | Add to progress tracker |
| `team:member_completed` | `{taskId, memberName, filesChanged}` | Update progress |
| `team:wave_validated` | `{taskId, wave, passed, errors}` | Show validation result |
| `team:disbanded` | `{taskId}` | Remove team indicator |

---

## 9. Cost/Benefit Analysis

### 9.1 Token Cost Comparison

| Scenario | Sequential Worker | Team Worker | Delta |
|----------|------------------|-------------|-------|
| 3-scope task | 4 contexts (worker + 3 coders) | 4 contexts + messaging overhead | ~+15% tokens |
| 6-scope task | 7 contexts (worker + 6 coders, 2 waves) | 7 contexts + messaging | ~+20% tokens |
| Simple 2-scope task | 3 contexts | 3 contexts + team overhead | ~+30% tokens (team overhead not worth it) |

### 9.2 Time Savings

| Scenario | Sequential | Parallel | Speedup |
|----------|-----------|---------|---------|
| 3 independent scopes | 3× coder time | 1× coder time + coord overhead | ~2.5x |
| 3 scopes with deps | 2× coder time (2 waves) | ~1.5× coder time | ~1.3x |
| 6 independent scopes | 6× coder time (2 waves of 3) | 2× coder time + coord | ~2.5x |

### 9.3 Quality Benefits

| Benefit | Description |
|---------|-------------|
| **Peer discovery** | Coders share patterns and gotchas in real-time |
| **Earlier conflict detection** | Interface mismatches caught during execution, not validation |
| **Dynamic load balancing** | Fast coders help slow ones |
| **Reduced iteration cycles** | Fewer wave validation failures due to better coordination |

### 9.4 When NOT to Use Team Execution

| Condition | Reason |
|-----------|--------|
| ≤2 task steps | Team overhead exceeds benefit |
| All steps sequential | No parallelism opportunity |
| Re-execution (review fixes) | Focused fixes, not broad implementation |
| Simple bug fixes | Single coder sufficient |
| File-heavy overlap | Can't partition ownership cleanly |

---

## 10. Implementation Roadmap

### Phase 1: Foundation (2-3 weeks)

| Task | Details |
|------|---------|
| Create `worker-team.md` agent definition | System prompt with team coordination instructions |
| Add to `ralphx.yaml` | New agent config with team tools |
| Add process mapping to config | `processes.task_execution.default/parallel` |
| Update `AgenticClientSpawner` | Support agent selection from process mapping |
| Environment variable propagation | `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` to spawned agents |

### Phase 2: Core Execution (3-4 weeks)

| Task | Details |
|------|---------|
| Worker-team execution flow | TeamCreate → TaskCreate → spawn coders → monitor → validate |
| File ownership protocol | Exclusive write lists in task descriptions |
| Wave validation integration | Same gates as sequential worker |
| Commit lock compatibility | Lead-only commits during team execution |
| Failure handling | Coder failure detection and recovery |

### Phase 3: UI Integration (2-3 weeks)

| Task | Details |
|------|---------|
| Multi-track progress display | Per-teammate progress in ExecutionTaskDetail |
| Team events on EventBus | `team:created`, `team:member_*`, `team:wave_*` |
| Running processes grouping | Group teammates under task in popover |
| Chat panel for teammates | Send messages to individual teammates |

### Phase 4: Optimization (2 weeks)

| Task | Details |
|------|---------|
| Auto-selection logic | Analyze task structure to choose sequential vs parallel |
| Token usage monitoring | Track and compare sequential vs team costs |
| Quality metrics | Compare defect rates, review pass rates |
| Graceful degradation | Fallback to sequential on team failures |

---

## 11. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| **MCP tools fail in team context** | Medium | High | Test MCP availability in foreground teammates early |
| **File conflicts despite ownership** | Low | High | Strict ownership enforcement in task descriptions |
| **Token costs too high** | Medium | Medium | Auto-selection avoids teams for simple tasks |
| **Teammates can't coordinate effectively** | Medium | Medium | Strong spawn prompts + lead relay pattern |
| **Agent teams feature changes/breaks** | High (experimental) | High | Abstract team management behind interface |
| **Performance regression** | Low | Medium | Benchmark against sequential execution |
| **Commit lock contention** | Low | Low | Only lead commits (single committer) |

---

## 12. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Execution time reduction** | >30% for 3+ scope tasks | Compare parallel vs sequential task duration |
| **Review pass rate** | ≥ current rate | Track first-time review approval |
| **Token overhead** | <25% increase | Compare token usage per task |
| **Team failure rate** | <10% of team executions | Count fallbacks to sequential |
| **User satisfaction** | Positive feedback | Manual assessment |

---

## 13. Open Questions

| # | Question | Options | Recommendation |
|---|----------|---------|----------------|
| 1 | Should coder teammates use RalphX's custom `ralphx-coder` agent type or `general-purpose`? | Custom (full MCP), General (simpler) | Custom — MCP tools are essential for step tracking |
| 2 | Should the lead be in strict delegate mode or allowed to write files? | Delegate (clean separation), Full (can fix things) | Delegate with fallback to direct fixes |
| 3 | How should team execution interact with the reviewer? | Same reviewer flow (team is transparent), Team includes reviewer | Same flow — reviewer doesn't need to know about team internals |
| 4 | Should cross-coder messages go through the lead or be direct? | Through lead (controlled), Direct (faster) | Direct for discoveries, through lead for scope changes |
| 5 | What's the max team size? | 3 (current coder limit), 5 (more parallelism), Dynamic | **RESOLVED v2:** Default 5 (consistent with ideation brief). Configurable via `team_constraints.max_teammates`. |
| 6 | How does QA phase interact with team execution? | QA after team completes (current), QA teammate in team | QA after (keep phases separate) |
| 7 | How much autonomy should the worker-lead have in choosing teammate composition? | Full dynamic (default), Constrained by YAML, Hybrid | **RESOLVED v2:** Dynamic default + constrained opt-in (consistent with ideation brief Section 4.1). |

---

## 14. Dependencies & Prerequisites

| Dependency | Status | Notes |
|-----------|--------|-------|
| Claude Code Agent Teams feature | Experimental | Must be enabled, may change |
| `CLAUDECODE` + `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` env vars | Available | Both required; needs propagation to spawned agents |
| MCP tools in foreground teammates | Needs verification | Critical path — test early |
| ralphx.yaml process mapping | New feature | Requires config parser changes |
| Frontend multi-track progress | New feature | Requires UI design |
| Agent teams in print mode (-p) | Compatible | Lead CAN use `-p`; teammates spawned without `-p` by team framework |

**Note on `-p` mode:** The team lead CAN be spawned with `-p` (print mode). Teammates are spawned internally by the lead without `-p` — they run as interactive processes managed by the team framework. This is compatible with RalphX's current spawning model.

---

## Appendix: Comparison with Current Worker Architecture

| Aspect | Current `ralphx-worker` | Proposed `ralphx-worker-team` |
|--------|------------------------|------------------------------|
| Delegation method | Subagent Task calls | Agent team teammates |
| Coder communication | None (isolated) | Direct messaging |
| Coordination | Worker processes all results | Shared task list + messages |
| Dynamic re-scoping | Not possible | Lead re-assigns tasks |
| File conflict prevention | Sub-step scope_context | Explicit file ownership in tasks |
| Validation | Worker runs gates | Lead runs gates (same logic) |
| Commit strategy | Worker commits | Lead commits (delegate mode) |
| Max coders | 3 (subagent limit) | Up to 5 (default, configurable via `team_constraints.max_teammates`) |
| Failure recovery | Worker retries scope | Lead re-assigns or re-spawns |
| Token efficiency | Results summarized to worker | Full context per teammate |
| Time efficiency | Sequential wave processing | Parallel with peer coordination |
