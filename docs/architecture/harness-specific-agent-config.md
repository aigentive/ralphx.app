# Harness-Specific Agent Config And Prompt Architecture

## Goal

Replace the current Claude-shaped agent prompt/config source of truth with a canonical RalphX agent definition layer that can produce harness-specific outputs for Claude, Codex, and future harnesses.

## Status

In progress.

Landed so far:
- phase 1 pilot skeleton under `agents/` for `orchestrator-ideation`, `ideation-team-lead`, and `session-namer`
- resolver-backed canonical prompt loading for those pilot agents on the Codex path, with legacy Claude plugin prompt fallback retained

Tracker reference:
- [AGENTS.md](/Users/lazabogdan/Code/ralphx/AGENTS.md)

## Problem

Today the system mixes three concerns into one Claude-oriented source:

1. Canonical RalphX agent identity and policy
2. Claude plugin/frontmatter/runtime bootstrap details
3. Prompt body text

That coupling is now incorrect because Codex is using the same prompt bodies and some of those prompt bodies contain Claude-only semantics such as:
- `Task(...)`
- Claude frontmatter/tool assumptions
- `mcpServers` guidance
- Claude-specific subagent behavior notes

Codex currently wraps those same prompts via `compose_codex_prompt(...)`, which means Codex inherits prompt content that was authored for Claude plugin execution rather than for Codex-native runtime behavior.

## Current State

### Source Of Truth Today

- Runtime agent config is loaded from [ralphx.yaml](/Users/lazabogdan/Code/ralphx/ralphx.yaml)
- Prompt bodies live in [plugins/app/agents](/Users/lazabogdan/Code/ralphx/plugins/app/agents)
- Claude runtime resolves `system_prompt_file` from [agent_config/mod.rs](/Users/lazabogdan/Code/ralphx/src-tauri/src/infrastructure/agents/claude/agent_config/mod.rs)
- Claude runtime/plugin bootstrap is built in [claude/mod.rs](/Users/lazabogdan/Code/ralphx/src-tauri/src/infrastructure/agents/claude/mod.rs)
- Codex currently loads those same prompt files through [compose_codex_prompt](/Users/lazabogdan/Code/ralphx/src-tauri/src/infrastructure/agents/codex/mod.rs)
- Chat/runtime Codex spawn paths call that from [chat_service_context.rs](/Users/lazabogdan/Code/ralphx/src-tauri/src/application/chat_service/chat_service_context.rs)

### Why This Is Wrong

- Claude plugin markdown is being treated as universal prompt source
- Claude-only prompt syntax leaks into Codex
- Claude frontmatter is mixed with prompt content
- Harness-specific spawn semantics are not isolated
- Adding Codex-specific behavior by branching inside shared prompts will make prompts larger, harder to maintain, and less testable

## Design Principles

1. RalphX canonical config must be the source of truth.
2. Claude plugin assets must become generated outputs, not the universal source.
3. Prompt bodies must be harness-specific where semantics differ.
4. Shared policy must stay shared to avoid duplicated grants and drift.
5. Harness-specific runtime bootstrap details must not leak into the wrong harness.
6. Migration must be incremental and agent-by-agent, not a big-bang rewrite.

## Target Architecture

## Canonical Layout

Recommended initial structure:

```text
agents/
  <agent-name>/
    shared.yaml
    claude/
      prompt.md
    codex/
      prompt.md
```

Possible later extension if needed:

```text
agents/
  <agent-name>/
    claude/agent.yaml
    codex/agent.yaml
```

Do not start with duplicated per-harness YAML unless concrete requirements force it.

## Shared Versus Harness-Specific Data

### Shared

Belongs in `shared.yaml`:

- canonical agent id/name
- role classification
- lane binding intent
- shared MCP tool grants
- shared preapproved CLI tools or delegated capabilities
- shared model/effort policy defaults where truly harness-neutral
- settings profile hooks
- whether the agent is helper-only, orchestrator, team lead, specialist, critic, worker, reviewer, merger, etc.
- migration metadata and generation flags

### Harness-Specific

Belongs in harness-specific prompt/config:

- prompt body
- harness-specific delegation syntax
- Claude frontmatter
- Claude `Task(...)` instructions
- Claude `mcpServers` guidance
- Codex-native delegation or subagent instructions
- harness-specific bootstrap framing
- harness-specific tool-calling notes

## Runtime Model

### Canonical Resolver

Add a RalphX agent resolver service that loads:

1. canonical shared agent definition
2. harness-specific prompt body
3. harness-specific runtime metadata

The resolver must return a harness-ready agent definition for:
- Claude backend spawns
- Codex backend spawns
- helper agents
- team/specialist/critic agent paths

### Claude Path

Claude still requires plugin-discoverable agent assets.

Therefore:
- canonical config remains source of truth
- Claude plugin markdown/frontmatter becomes generated output
- generated assets feed the existing Claude plugin/runtime path

This keeps Claude compatible with its plugin agent discovery and subagent spawning model.

### Codex Path

Codex should stop reading Claude plugin prompt files as the canonical prompt body.

Instead:
- Codex loads the harness-specific Codex prompt body from canonical config
- Codex prompt composition no longer depends on Claude frontmatter or Claude-oriented prompt text
- Codex-specific delegation semantics become first-class instead of patched into shared prompts

## Generation Pipeline

### Required Output For Claude

Generate Claude assets from canonical config:

- plugin agent markdown
- frontmatter
- tool lists
- `mcpServers` declarations where needed

These outputs may remain in `plugins/app/agents/` for compatibility, but they should be generated artifacts.

### Required Output For Codex

Codex does not need Claude plugin frontmatter.

Codex should consume:
- resolved prompt text
- resolved MCP/tool policy
- resolved runtime model/effort/sandbox/approval settings

from the canonical resolver directly.

## Migration Plan

## Phase 0: Freeze Scope

Before behavior changes:
- document the current architecture
- define canonical schema
- define generated Claude outputs
- define Codex resolver interface

## Phase 1: Canonical Config Skeleton

Implement:
- `agents/<agent>/shared.yaml`
- `agents/<agent>/claude/prompt.md`
- `agents/<agent>/codex/prompt.md`
- resolver that can load one agent from the new structure

No runtime cutover yet.

## Phase 2: Ideation Pilot

Migrate only:
- `orchestrator-ideation`
- `ideation-team-lead`
- `session-namer`

Reason:
- they are the highest-value agents for current Codex/Claude divergence
- they directly affect the current product issues

Deliverables:
- Codex uses Codex-native prompt files for these agents
- Claude plugin assets for these agents are generated from canonical config
- no Claude-only instructions remain in the Codex versions

## Phase 3: Core Execution Agents

Migrate:
- `worker`
- `coder`
- `reviewer`
- `merger`
- `review-chat`
- `chat-task`
- `chat-project`

## Phase 4: Specialists And Critics

Migrate:
- ideation specialists
- advocate/critic
- plan critics
- plan verifier

This is where harness-specific delegation semantics matter most.

## Phase 5: Remove Legacy Source-Of-Truth Status

After all production agents are migrated:
- stop treating `plugins/app/agents/*.md` as authored source
- keep them only as generated Claude assets if still required
- update docs/rules accordingly

## Agent Cohorts To Migrate

### Pilot

- `orchestrator-ideation`
- `ideation-team-lead`
- `session-namer`

### Core User-Facing

- `chat-task`
- `chat-project`
- `review-chat`
- `review-history`

### Execution Pipeline

- `ralphx-worker`
- `ralphx-coder`
- `ralphx-reviewer`
- `ralphx-merger`
- `ralphx-worker-team`

### Verification / Debate

- `plan-verifier`
- `plan-critic-completeness`
- `plan-critic-implementation-feasibility`
- `ideation-advocate`
- `ideation-critic`

### Specialists

- `ideation-specialist-backend`
- `ideation-specialist-frontend`
- `ideation-specialist-infra`
- `ideation-specialist-ux`
- `ideation-specialist-code-quality`
- `ideation-specialist-prompt-quality`
- `ideation-specialist-intent`
- `ideation-specialist-pipeline-safety`
- `ideation-specialist-state-machine`

## Required Tests

### Resolver Tests

- canonical shared config loads
- harness-specific prompt file loads
- harness-specific override resolution works
- unknown/missing harness prompt fails clearly

### Claude Generation Tests

- generated Claude markdown/frontmatter matches expected fields
- generated tool grants match canonical MCP policy
- `mcpServers` generation is correct
- existing Claude runtime can still discover and spawn generated agents

### Codex Prompt Tests

- Codex prompt bodies contain no Claude-only frontmatter
- Codex prompt bodies contain no `Task(...)` unless explicitly intended
- Codex prompt composition still preserves RalphX bootstrap wrappers where needed

### Runtime Contract Tests

- Claude path still resolves the expected prompt/tool/runtime config
- Codex path resolves the Codex-specific prompt/tool/runtime config
- same agent id can resolve different harness prompt bodies from the same shared definition

### Integration Tests

- ideation pilot agents spawn correctly on both harnesses
- Claude subagent behavior remains intact
- Codex runtime no longer consumes Claude plugin prompt bodies for migrated agents

## Acceptance Criteria

The migration is successful when:

1. Canonical RalphX config is the source of truth for migrated agents.
2. Claude plugin markdown/frontmatter is generated for migrated agents.
3. Codex no longer loads Claude plugin prompt bodies for migrated agents.
4. Codex prompts for migrated agents contain no Claude-only semantics unless explicitly preserved by design.
5. MCP grants remain aligned between canonical config, generated Claude assets, and runtime allowlists.
6. Existing Claude plugin-based subagent behavior still works.
7. The ideation pilot works on both Claude and Codex without prompt-body leakage.

## Non-Goals

- full multi-harness subagent parity in one step
- removing Claude plugin discovery immediately
- redesigning all agent instructions during this migration
- changing lane policy/model policy unless needed for harness split

## Open Questions

1. Codex-native delegation/subagent model
Do we map RalphX specialists/critics to Codex-native subagents, Codex tools, or keep some delegation in RalphX MCP/backend instead?

2. Generated asset location
Do generated Claude plugin assets remain committed in `plugins/app/agents/`, or do we generate them into a build artifact directory and sync them at runtime/build time?

3. Shared versus harness-specific model defaults
Should model/effort defaults remain shared unless overridden, or should they move into harness sections early?

4. Agent frontmatter ownership
How much Claude frontmatter should be generated from canonical schema versus templated as raw harness-specific text?

## Recommended First Implementation Slice

Implement only:

- canonical config skeleton
- resolver service
- Claude generation for 3 agents
- Codex-native prompt loading for 3 agents
- tests for those 3 agents

Pilot agents:
- `orchestrator-ideation`
- `ideation-team-lead`
- `session-namer`

That is the smallest slice that fixes the current Codex/Claude prompt coupling without forcing a repo-wide migration in one round.
