# Task Tool: Parallel Dispatch Reference

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

---

## How the Task Tool Works

`Task` launches a subagent (subprocess) that executes a prompt autonomously and returns a single result message.

| Parameter | Purpose |
|-----------|---------|
| `prompt` | The task for the subagent to perform |
| `subagent_type` | Agent type: `Bash`, `general-purpose`, `Explore`, `Plan`, etc. |
| `run_in_background` | `true` → runs concurrently, returns `output_file` path to check later |
| `resume` | Agent ID from prior invocation → continues with full context preserved |

---

## Parallel vs Sequential Dispatch

**The core mechanic**: Claude processes all tool calls in a single response concurrently. This is how you achieve parallelism.

| Dispatch Style | Mechanic | Result |
|----------------|----------|--------|
| **Parallel** | Multiple `Task` calls in ONE response | All agents run concurrently |
| **Sequential** | One `Task` call per response | Each agent blocks → next waits for result |

### ✅ Correct: Parallel Dispatch (single response)

```
Response contains:
  Task(prompt="Coder A: implement cache store", subagent_type="general-purpose")
  Task(prompt="Coder B: implement cache tests", subagent_type="general-purpose")
  Task(prompt="Coder C: implement cache types", subagent_type="general-purpose")
→ All 3 launch simultaneously, results return together
```

### ❌ Wrong: Sequential Dispatch (separate responses)

```
Response 1: Task(prompt="Coder A: implement cache store")
  → waits for result...
Response 2: Task(prompt="Coder B: implement cache tests")
  → waits for result...
Response 3: Task(prompt="Coder C: implement cache types")
  → waits for result...
→ 3x slower — each coder waits for the previous one to finish
```

---

## Foreground vs Background Agents

| Mode | How | MCP Tools Available? | Use When |
|------|-----|---------------------|----------|
| **Foreground** (default) | `Task(...)` | ✅ Yes | Coders needing `start_step`, `complete_step`, etc. |
| **Background** | `Task(..., run_in_background: true)` | ❌ No | Research/exploration agents that don't need MCP |

### Critical Constraint: MCP Tools in Background Agents

**Background subagents (`run_in_background: true`) CANNOT use MCP tools.** This means:

- `ralphx-coder` agents need MCP tools (`start_step`, `complete_step`, `get_task_context`, etc.)
- Therefore coders **MUST** run in foreground mode
- Parallel execution of coders is achieved by putting multiple foreground `Task` calls in a single response — NOT by using `run_in_background`

---

## Wave Execution Pattern

For worker agents dispatching coders in waves:

```
Wave 1:
  1. Prepare STRICT SCOPE prompts for all coders in this wave
  2. Emit ALL coder Task calls in a SINGLE response (→ parallel)
  3. Wait for all results to return
  4. Run wave gate validation (typecheck + tests + lint)
  5. Commit wave changes

Wave 2 (only if Wave 1 gate passes):
  1. Prepare prompts for next wave (can reference Wave 1 output)
  2. Emit ALL coder Task calls in a SINGLE response
  3. Wait → validate → commit
  ... repeat
```

### Wave Gate Between Dispatches

You MUST wait for all coders in a wave to complete before starting the next wave. This happens naturally with foreground parallel dispatch — all results return before your next response.

---

## Resuming Agents

| Scenario | How |
|----------|-----|
| Follow-up work on same scope | `Task(resume: "<agent_id>", prompt: "Now fix the test failure")` |
| Agent context preserved | Full previous conversation carried forward |
| When to use | Coder needs to fix a validation issue found in wave gate |

---

## When to Use Background Mode

Background mode (`run_in_background: true`) is appropriate for:

- `Explore` agents doing codebase research (no MCP tools needed)
- Long-running research tasks where you don't need results immediately
- Tasks that only use file system tools (Read, Grep, Glob)

**Never use background mode for**:
- `ralphx-coder` agents (need MCP tools)
- Any agent that must call `start_step`, `complete_step`, or other MCP endpoints

---

## Summary Rules

1. **Multiple Task calls in ONE response = parallel execution**
2. **One Task call per response = sequential execution** (anti-pattern for waves)
3. **Coders MUST run foreground** (MCP tool constraint)
4. **Background is for research only** (no MCP tools available)
5. **Wave gates happen between responses** — dispatch wave → all results return → validate → next wave
