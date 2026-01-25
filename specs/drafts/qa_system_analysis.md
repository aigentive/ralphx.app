# QA System Analysis

**Created:** 2026-01-25
**Status:** Draft - For future implementation
**Related:** [Context-Aware Chat Implementation Plan](../plans/context_aware_chat_implementation.md)

---

## Overview

This document captures the analysis of the QA system in RalphX, including how QA agents work, how results are captured, and gaps identified for future improvement.

---

## QA Flow Summary

The QA system has 3 phases, each handled by a different agent:

| Phase | Agent | State | Purpose |
|-------|-------|-------|---------|
| **1. QA Prep** | `qa-prep` | `Ready` (background) | Generate acceptance criteria and test steps |
| **2. QA Refining** | `qa-refiner` | `QaRefining` | Refine test steps based on actual git diff |
| **3. QA Testing** | `qa-tester` | `QaTesting` | Execute browser tests via agent-browser |

### State Machine Flow

```
Ready (spawn qa-prep in background)
   │
   ▼
Executing (worker agent)
   │
   ▼
ExecutionDone
   │
   ├─── [QA disabled] ──────────────────────► PendingReview
   │
   └─── [QA enabled] ───► QaRefining ───► QaTesting ───┬──► QaPassed ──► PendingReview
                                                        │
                                                        └──► QaFailed ──► RevisionNeeded
```

---

## How QA Results Are Currently Captured

### Current Approach: JSON Output Parsing

QA agents output **structured JSON** that the Rust backend **parses after agent completion**:

1. **qa-prep agent** outputs:
   ```json
   {
     "acceptance_criteria": [...],
     "qa_steps": [...]
   }
   ```

2. **Backend** (`qa_service.rs`) calls:
   - `wait_for_prep()` → `parse_qa_prep_output()` → extracts JSON
   - `repository.update_prep()` → stores criteria and steps

3. **qa-tester agent** outputs:
   ```json
   {
     "qa_results": {
       "task_id": "...",
       "overall_status": "passed|failed",
       "steps": [...]
     }
   }
   ```

4. **Backend** parses and calls:
   - `repository.update_results()` → stores test results and screenshots

### Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/application/qa_service.rs` | QA orchestration, agent spawning, result parsing |
| `src-tauri/src/commands/qa_commands.rs` | Tauri commands for QA operations |
| `src-tauri/src/domain/entities/task_qa.rs` | TaskQA entity |
| `src-tauri/src/domain/qa/results.rs` | QAResults, QAStepResult types |
| `src-tauri/src/domain/qa/criteria.rs` | AcceptanceCriteria, AcceptanceCriterion types |
| `ralphx-plugin/agents/qa-prep.md` | QA prep agent definition |
| `ralphx-plugin/agents/qa-executor.md` | QA executor agent definition (refinement + testing) |

---

## Gaps Identified

### 1. Missing `qa-refiner` Agent File

**Problem:** The state machine transition handler (`transition_handler.rs`) spawns `"qa-refiner"` in the `QaRefining` state:

```rust
// transition_handler.rs, line ~123
agent_spawner.spawn("qa-refiner", &task_id)
```

But there is **no `qa-refiner.md`** agent file in `ralphx-plugin/agents/`.

**What exists:**
- `qa-prep.md` - Generates acceptance criteria and test steps
- `qa-executor.md` - Combined agent that handles **both** refinement (Phase 2A) AND testing (Phase 2B)

**Resolution options:**
1. **Option A:** Change transition handler to spawn `"qa-executor"` instead of `"qa-refiner"`
2. **Option B:** Split `qa-executor.md` into two agents: `qa-refiner.md` and `qa-tester.md`
3. **Option C:** Create a `qa-refiner.md` that's a subset of `qa-executor.md` (just Phase 2A)

### 2. JSON Parsing vs MCP Tools

**Current approach limitations:**
- Relies on agent outputting well-formed JSON
- Backend must parse agent output text to extract JSON
- Error handling is fragile (what if JSON is malformed?)
- No real-time result submission (must wait for agent to complete)

**MCP-based approach benefits:**
- Agent calls `submit_qa_prep_results` or `submit_qa_test_results` tool
- Structured input validation via MCP schema
- Results persisted immediately when tool is called
- More reliable than text parsing
- Agent can call multiple times (incremental results)

---

## Future Recommendation: Migrate QA Agents to MCP

When implementing improvements to the QA system, consider transitioning QA agents to use MCP tools instead of JSON output parsing.

### Proposed MCP Tools for QA

| Tool | Agent | Purpose |
|------|-------|---------|
| `submit_qa_prep` | qa-prep | Submit acceptance criteria and test steps |
| `submit_qa_refinement` | qa-refiner | Submit refined test steps after git diff analysis |
| `submit_qa_step_result` | qa-tester | Submit result for individual test step (allows incremental) |
| `complete_qa_testing` | qa-tester | Signal all tests complete with overall status |

### Benefits

1. **More reliable** - Structured MCP schema validation vs text parsing
2. **Incremental results** - Submit step results as tests run, not all at end
3. **Consistent pattern** - Matches reviewer (`complete_review`) and ideation (`create_task_proposal`) patterns
4. **Better error handling** - MCP errors are structured, not buried in output text
5. **Real-time UI updates** - Results can be emitted as events when MCP tool is called

### Implementation Notes

When migrating:
1. Add tools to MCP server (`ralphx-mcp-server/src/tools.ts`)
2. Add HTTP endpoints in Rust (`src-tauri/src/http_server.rs`)
3. Update TOOL_ALLOWLIST:
   ```typescript
   "qa-prep": ["submit_qa_prep"],
   "qa-refiner": ["submit_qa_refinement"],
   "qa-tester": ["submit_qa_step_result", "complete_qa_testing"],
   ```
4. Update agent `.md` files (no explicit tool docs needed - MCP auto-exposes)
5. Remove JSON parsing logic from `qa_service.rs`

---

## References

- **State Machine:** `src-tauri/src/domain/state_machine/machine.rs`
- **Transition Handler:** `src-tauri/src/domain/state_machine/transition_handler.rs`
- **QA Service:** `src-tauri/src/application/qa_service.rs`
- **Master Plan:** `specs/plan.md` (search for "qa_refining")
- **Context-Aware Chat Plan:** `specs/plans/context_aware_chat_implementation.md` (MCP pattern reference)
