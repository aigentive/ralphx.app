> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Agent Teams: Context Relaunch Protocol Implementation Spec

**Decision:** Approved context relaunch protocol for agent teams
**Phase:** 1C (post-integration, pre-UI)

## File Modifications Table

| File | Type | Lines Added | Lines Changed | Description |
|------|------|-------------|---------------|-------------|
| `agents/ralphx-ideation-team-lead/claude/prompt.md` | Prompt | ~35 | 0 | Add relaunch protocol section for lead |
| `agents/ralphx-execution-team-lead/claude/prompt.md` | Prompt | ~35 | 0 | Add relaunch protocol section for lead |
| `agents/ideation-specialist-*/claude/prompt.md` | Prompt | ~8 | 0 | Add self-report rule |
| `agents/ralphx-ideation-critic/claude/prompt.md` | Prompt | ~8 | 0 | Add self-report rule |
| `agents/ralphx-ideation-advocate/claude/prompt.md` | Prompt | ~8 | 0 | Add self-report rule |
| `agents/ralphx-execution-coder/claude/prompt.md` | Prompt | ~8 | 0 | Add self-report rule |
| `src-tauri/src/infrastructure/agents/team_config.rs` | Rust | ~6 | 0 | Add `context_relaunch_threshold` field |
| `ralphx.yaml` | YAML | ~1 | 0 | Add threshold to defaults |
| `src-tauri/src/domain/artifacts/mod.rs` | Rust | ~15 (optional) | 0 | Add `TeamHandoff` artifact type |
| `src-tauri/src/infrastructure/agents/team_state_tracker.rs` | Rust | ~3 | 0 | Add `relaunch_count` tracking |

## Prompt Modifications

### Team Lead Prompts (`agents/ralphx-ideation-team-lead/claude/prompt.md`, `agents/ralphx-execution-team-lead/claude/prompt.md`)

**Location:** After team coordination section, before workflow phases

**Section to add:**

```markdown
## Context Window Relaunch Protocol

When teammates approach context limits, use this protocol to maintain quality:

### Lead Responsibilities

| Step | Action | Tool/Command |
|------|--------|--------------|
| 1 | **Include self-report instruction** | Add to every teammate prompt: "Self-report when reaching ~20% context remaining." |
| 2 | **Monitor handoff messages** | Teammate sends: "Context at 22%, committing current work. Handoff: {summary}" |
| 3 | **Request shutdown** | `SendMessage(type: "shutdown_request", recipient: "{name}", content: "Acknowledged handoff")` |
| 4 | **Wait for confirmation** | Teammate responds with `shutdown_response(approve: true)` |
| 5 | **Spawn fresh teammate** | `Task(...)` with same role, same worktree/branch, inject handoff summary in prompt |

### Relaunch Prompt Template

When spawning a relaunched teammate, inject this context:

```
You are {name}, continuing work from a previous teammate that hit context limits.

YOUR WORKTREE: {worktree_path}
YOUR BRANCH: {branch}

HANDOFF FROM PREVIOUS TEAMMATE:
{handoff_summary}

REMAINING WORK:
{remaining_task_items}

RELAUNCH COUNT: {relaunch_count}

Start by reading recent git diff to understand what's been implemented.
```

### Handoff Summary Format

Instruct outgoing teammate to provide:

```markdown
## Context Relaunch Handoff

**What's done:**
- {completed items}

**What's remaining:**
- {remaining items}

**Gotchas/discoveries:**
- {edge cases, integration issues, blockers}

**Files modified:**
- {list of uncommitted changes if any}
```

### Quality Checks

Before relaunching, verify:
- [ ] Outgoing teammate committed all work (no uncommitted changes)
- [ ] Handoff summary includes concrete details, not just high-level status
- [ ] Branch state is clean (no merge conflicts, no stray files)
- [ ] Remaining work items are clear and actionable

### Multiple Relaunches

- Track relaunch count in teammate prompt for context
- No limit on relaunch cycles — each starts fresh from git checkpoint
- If teammate relaunches >3 times, consider task complexity issue → message user
```

### Specialist/Advocate/Critic Prompts

**Files:**
- `ralphx-ideation-specialist.md`
- `ralphx-ideation-critic.md`
- `ralphx-ideation-advocate.md`
- `coder.md`

**Location:** In `<rules>` section, after core responsibilities, before workflow

**Section to add:**

```markdown
## Context Window Management

**Self-report threshold:** When you observe your context window approaching ~20% remaining capacity:

1. **Stop new work** — finish current analysis/implementation, do not start new subtasks
2. **Checkpoint state** — commit any code changes to your branch (even if partial)
3. **Create handoff** — document what's done, what's remaining, and gotchas discovered
4. **Message lead** — send handoff summary via SendMessage to team lead
5. **Prepare for shutdown** — wait for shutdown_request from lead, approve when received

**Handoff format:**
```
Context at {percentage}%, requesting relaunch.

Done:
- {item 1}
- {item 2}

Remaining:
- {item 3}

Gotchas:
- {edge case or blocker}

Files modified: {list or "none — all committed"}
```

**Why 20%?** Below 10%, context compaction degrades output quality. At 20%, you have runway to produce a quality handoff and commit clean state.
```

## Backend Modifications

### TeamConstraints Struct

**File:** `src-tauri/src/infrastructure/agents/team_config.rs`

**Location:** In `TeamConstraints` struct definition

**Code to add:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct TeamConstraints {
    // ... existing fields ...

    /// Context window threshold (%) for teammate self-report and relaunch
    /// Default: 20 (range: 10-40)
    #[serde(default = "default_context_relaunch_threshold")]
    pub context_relaunch_threshold: u8,
}

fn default_context_relaunch_threshold() -> u8 {
    20
}
```

### YAML Configuration

**File:** `ralphx.yaml`

**Location:** In `team_constraints._defaults` section

**Line to add:**

```yaml
team_constraints:
  _defaults:
    max_teammates: 5
    model_cap: sonnet
    mode: dynamic
    timeout_minutes: 20
    budget_limit: null
    context_relaunch_threshold: 20  # NEW: Self-report threshold (%)
```

### TeamHandoff Artifact Type (Optional)

**File:** `src-tauri/src/domain/artifacts/mod.rs`

**Location:** In `ArtifactType` enum

**Code to add:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    // ... existing variants ...

    /// Structured handoff summary from relaunched teammate
    /// Contains: work completed, work remaining, gotchas, files modified
    TeamHandoff,
}
```

**Note:** Can use existing `TeamSummary` type for handoffs if structured type not needed.

### Team State Tracker

**File:** `src-tauri/src/infrastructure/agents/team_state_tracker.rs`

**Location:** In teammate tracking struct (wherever teammate status is stored)

**Field to add:**

```rust
pub struct TeammateStatus {
    // ... existing fields ...

    /// Number of times this teammate slot has been relaunched due to context limits
    pub relaunch_count: u8,
}
```

**Increment on relaunch:**

```rust
// When spawning a relaunched teammate for the same role/slot
teammate_status.relaunch_count += 1;
```

## Implementation Order

| Phase | Task | Files Modified | Dependency |
|-------|------|----------------|------------|
| 1 | Add `context_relaunch_threshold` config field | `team_config.rs`, `ralphx.yaml` | None |
| 2 | Update team lead prompts | `ralphx-ideation-team-lead.md`, `worker-team.md` | Phase 1 |
| 3 | Update specialist prompts | 4 specialist prompt files | Phase 1 |
| 4 | Add `relaunch_count` tracking | `team_state_tracker.rs` | Phase 1 |
| 5 | (Optional) Add `TeamHandoff` artifact type | `artifacts/mod.rs` | Phase 1 |
| 6 | Integration test with long-running task | Test suite | Phases 1-4 |

## Testing Strategy

| Test Type | Scenario | Expected Behavior |
|-----------|----------|-------------------|
| **Unit** | `TeamConstraints` deserializes with threshold | Default = 20, custom values 10-40 accepted |
| **Integration** | Teammate self-reports at 20% | Lead receives handoff message with correct format |
| **Integration** | Lead performs relaunch sequence | Fresh teammate spawned with handoff context injected |
| **E2E** | Long task requiring >1 relaunch | Multiple relaunch cycles complete successfully, quality maintained |
| **Observability** | Check `relaunch_count` after session | Count increments correctly, logged for analytics |

## Migration Notes

- **Backward compatible:** Existing configs without `context_relaunch_threshold` use default (20)
- **No state machine changes:** Protocol is coordination-only, no DB schema changes
- **No UI changes:** Relaunch is invisible to user (appears as continuous progress)
- **Prompt-only enforcement:** Teammates self-report based on prompt instruction (honor system)

## Observability

| Metric | Source | Purpose |
|--------|--------|---------|
| `relaunch_count` per teammate | `TeammateStatus` | Track relaunch frequency per role/task type |
| Total relaunches per session | Aggregate teammate counts | Identify tasks with high context consumption |
| Context % at handoff | Parse from handoff message | Validate self-report accuracy (~20% expected) |
| Quality delta pre/post relaunch | Code review scores | Validate relaunch maintains quality |

## Cost Analysis

| Impact | Estimate | Notes |
|--------|----------|-------|
| **Additional tokens** | +5-10k per relaunch | Handoff summary + re-reading git diff |
| **Additional time** | +5-10 min per relaunch | Fresh agent warm-up time |
| **Quality improvement** | High | Prevents 0% context degradation (observed: markdown instead of code, no tests) |
| **Net value** | Positive | Small overhead vs. large quality gain |

## Edge Cases

| Case | Handling |
|------|----------|
| Teammate self-reports >3 times | Lead messages user: "Task may be too complex for single agent, consider splitting" |
| Teammate forgets to commit before handoff | Relaunch prompt instructs: "Check git status, commit any uncommitted work first" |
| Handoff message lacks detail | Lead asks for clarification before relaunch, or re-reads git log for context |
| Multiple teammates hit threshold simultaneously | Lead handles sequentially (one relaunch at a time to avoid coordination overhead) |
| User wants different threshold | Override via `team_constraints.{process}.context_relaunch_threshold` in YAML |
