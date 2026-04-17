# Codex CLI Migration Plan

Status: planning phase. This document covers the compatibility and rollout migrations required to move RalphX from Claude-shaped contracts to provider-neutral contracts without breaking live behavior.

## 1. Migration principles

- forward-only DB migrations only
- compatibility shims before destructive renames
- dual-read / dual-write when necessary
- keep old frontend/API fields until all consumers are switched
- never mix naming cleanup with first-time Codex runtime behavior in one large change

## 2. Database migrations

### 2.1 Chat conversations

Current issue:

- `chat_conversations.claude_session_id` is provider-specific

Target:

- add `provider_session_id`
- add `provider_harness`

Compatibility approach:

- phase A:
  - add new columns
  - backfill existing Claude rows where possible
  - keep old column for reads/writes
- phase B:
  - switch repositories/services to provider-neutral fields
  - maintain compatibility writes to old field for a limited transition window if needed
- phase C:
  - remove old field only after all backend/frontend consumers are migrated

### 2.2 Agent runs

Current issue:

- model info exists, but harness kind and provider session metadata are incomplete

Target:

- persist:
  - harness kind
  - provider session id
  - logical model
  - resolved vendor model
  - logical effort
  - resolved vendor effort
  - approval policy
  - sandbox mode

### 2.3 Ideation settings

Current issue:

- model and effort tables encode Claude-era abstractions only

Target:

- either:
  - new provider-neutral lane settings tables, or
  - new additive columns/tables that supersede the old ideation model/effort tables

Preferred direction:

- additive new lane-settings tables keyed by lane and scope

Reason:

- trying to stretch `primary_model`, `verifier_model`, and `ModelLevel` into multi-provider support will create a long compatibility burden

### 2.4 Ideation sessions

Current issue:

- `last_effective_model` exists but is model-only and harness-blind

Target:

- add last effective harness and resolved effort if the UI/recovery layer needs them

## 3. Rust/domain/API migrations

### 3.1 Conversation entity and repository

Current issue:

- `claude_session_id`, `set_claude_session_id`, `has_claude_session`

Migration:

- add provider-neutral fields/methods first
- deprecate Claude-specific methods behind compatibility wrappers
- migrate call sites incrementally

### 3.2 Agent profile surface

Current issue:

- `ClaudeCodeConfig`
- `AgentProfile.claude_code`
- `Model = opus|sonnet|haiku`

Migration:

- introduce harness-neutral profile/config surface
- keep a Claude-specific nested block only as one renderer/variant, not the top-level contract

### 3.3 Ideation settings types

Current issue:

- `ModelLevel` and `EffortLevel` are Claude-shaped

Migration:

- add provider-neutral lane config model
- keep legacy ideation settings readers only until the UI and runtime move over

### 3.4 Event payloads

Current issue:

- event payloads expose `claude_session_id`

Migration:

- add provider-neutral run completion/start payloads
- keep old field temporarily if a frontend compatibility window is needed

## 4. Frontend migrations

### 4.1 Chat types

Current issue:

- `claudeSessionId`
- `claude_session_id`

Migration:

- add provider-neutral fields:
  - `providerSessionId`
  - `harness`
- adapt consumers
- remove Claude naming only after all uses are gone

### 4.2 Agent profile types

Current issue:

- `claudeCode`
- `ModelSchema = opus|sonnet|haiku`

Migration:

- add harness-neutral config and richer model selection types

### 4.3 Settings UI

Current issue:

- ideation settings are model-only and Claude-alias-based
- project settings expose only execution `model` plus `allow_opus_upgrade`

Migration:

- introduce explicit per-lane harness/model/effort controls
- keep default Claude selections where no user override exists

## 5. Runtime migration order

1. add DB/schema support
2. add Rust compatibility fields and repositories
3. add frontend compatibility parsing
4. move shared orchestration to provider-neutral contracts
5. add Codex runtime path
6. switch settings UI to new lane model
7. retire old Claude-shaped paths only after no callers remain

## 6. Rollback strategy

If Codex rollout needs to be disabled after partial landing:

- Claude remains default harness
- Codex lanes can be feature-gated off
- provider-neutral schema stays valid because Claude is just one harness value
- old compatibility fields keep existing data readable

## 7. Migration review checklist

Every migration PR should answer:

- what new additive schema/API is introduced
- what legacy field/path still exists
- whether reads are dual-source
- whether writes are dual-target
- what test coverage locks the compatibility behavior
- what future commit removes the compatibility layer
