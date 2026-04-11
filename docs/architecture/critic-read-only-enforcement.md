> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Critic Agent Read-Only Enforcement

## Problem

User reported critics (`ralphx-plan-critic-completeness`, `ralphx-plan-critic-implementation-feasibility`) making code changes during auto-verification, crashing the app. Critics have `disallowedTools: [Write, Edit, NotebookEdit, Bash]` in their YAML definitions, but this wasn't enforced.

## Root Cause

### Spawn chain
```
ClaudeChatService.send_message()
  → spawns ralphx-ideation (main Claude process)
    → orchestrator calls Task(ralphx:ralphx-plan-critic-completeness)
      → critic runs as subagent
```

### Why disallowedTools failed

| Check | Finding |
|-------|---------|
| `Task()` / Agent tool `disallowedTools` param? | ❌ Does not exist. Parameters: description, prompt, subagent_type, resume, run_in_background, isolation, mode, model, name only. |
| YAML enforcement at spawn time? | Only works if the plugin is loaded in the subprocess context. `ClaudeChatService` may not pass `--plugin-dir` to the spawned process. |
| Subagent tool inheritance? | Subagents inherit ALL tools from parent by default. If plugin not loaded, named agent type can't be resolved → falls back to generic agent with full toolset. |

### Failure path

If the `ralphx-ideation` subprocess was spawned without `--plugin-dir ./plugins/app`, the `ralphx:ralphx-plan-critic-completeness` agent type cannot be resolved. The critic falls back to a general-purpose agent inheriting the orchestrator's full toolset (which has no Write/Edit restrictions at the process level if `bypassPermissions` is set).

## Fix Applied

**Belt-and-suspenders instruction** added to the top of all critic system prompts:

```
CRITICAL — READ-ONLY AGENT (NON-NEGOTIABLE): You MUST NOT use Write, Edit,
NotebookEdit, or Bash tools under any circumstances. Do not create files,
modify files, run commands, or take any action that changes the filesystem
or codebase. You are a pure analysis agent.
```

Files modified:
- `agents/ralphx-plan-critic-completeness/claude/prompt.md`
- `agents/ralphx-plan-critic-implementation-feasibility/claude/prompt.md`

## Defense-in-Depth Status

| Layer | Status | Notes |
|-------|--------|-------|
| YAML `tools` allowlist | ✅ Present | Read, Grep, Glob, WebFetch, WebSearch only |
| YAML `disallowedTools` | ✅ Present | Write, Edit, NotebookEdit, Bash + MCP write tools |
| Prompt instruction | ✅ Added (this fix) | Belt-and-suspenders at top of system prompt |
| `Task()` spawn-time restriction | ❌ Not available | No parameter exists in Task/Agent tool |

## Future Consideration

If Anthropic adds a `disallowedTools` parameter to the `Task()` / `Agent` tool, use it at spawn time in the orchestrator VERIFY phase for additional enforcement. Track at: https://github.com/anthropics/claude-code/issues
