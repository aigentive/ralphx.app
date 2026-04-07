# Codex CLI Implementation Backlog

Status: execution backlog derived from the frozen phase-1 contract. This document breaks the work into PR-sized slices with owned file surfaces and validation expectations.

Parent references:

- `specs/codex-cli-contract.md`
- `specs/codex-cli-implementation-plan.md`
- `specs/codex-cli-migrations.md`
- `specs/codex-cli-test-plan.md`

## Slice 1. Harness core types

Goal:

- add the provider-neutral type system without changing live behavior

Primary backend files:

- `src-tauri/crates/ralphx-domain/src/agents/types.rs`
- `src-tauri/crates/ralphx-domain/src/agents/capabilities.rs`
- `src-tauri/crates/ralphx-domain/src/entities/agent_run.rs`
- `src-tauri/crates/ralphx-domain/src/entities/chat_conversation.rs`
- `src-tauri/crates/ralphx-domain/src/repositories/agent_run_repository.rs`
- `src-tauri/crates/ralphx-domain/src/repositories/chat_conversation_repository.rs`

Expected changes:

- add `AgentHarnessKind`
- add provider-neutral session/run metadata fields to domain entities
- add capability/model metadata types needed by both harnesses
- keep Claude compatibility methods temporarily

Required tests:

- domain type tests
- serialization/deserialization tests
- repository trait mock updates

Out of scope:

- no Codex runtime yet
- no DB migrations yet

## Slice 2. Additive DB migrations and repository compatibility

Goal:

- persist the new provider-neutral fields without breaking old rows

Primary backend files:

- `src-tauri/src/infrastructure/sqlite/migrations/**`
- `src-tauri/src/infrastructure/sqlite/sqlite_chat_conversation_repo.rs`
- `src-tauri/src/infrastructure/sqlite/sqlite_agent_run_repo.rs`
- `src-tauri/src/infrastructure/memory/memory_chat_conversation_repo.rs`

Expected changes:

- add `provider_session_id` and `provider_harness` to `chat_conversations`
- add run metadata columns to `agent_runs`
- dual-read / compatibility behavior for `claude_session_id`

Required tests:

- migration tests
- sqlite repo tests
- memory repo tests

Out of scope:

- no event normalization yet
- no new frontend types yet

## Slice 3. New lane settings storage and commands

Goal:

- add the provider-neutral lane settings model alongside current settings

Primary backend files:

- new migration and repo files for `agent_lane_settings`
- `src-tauri/src/application/app_state.rs`
- new domain/repository definitions under `src-tauri/crates/ralphx-domain/src/**`
- new command surfaces under `src-tauri/src/commands/**`

Expected changes:

- add lane enum and lane settings structs
- add global/project lane settings repo
- add Tauri commands:
  - `get_agent_lane_settings`
  - `update_agent_lane_settings`
- implement legacy fallback resolution from current ideation model/effort settings

Required tests:

- repo tests
- command tests
- resolution-order tests

Out of scope:

- no settings UI yet
- no Codex routing yet

## Slice 4. Normalize Claude event pipeline

Goal:

- move current Claude paths onto the normalized event contract first

Primary backend files:

- `src-tauri/src/application/chat_service/chat_service_types.rs`
- `src-tauri/src/application/chat_service/chat_service_streaming.rs`
- `src-tauri/src/application/chat_service/chat_service_send_background.rs`
- `src-tauri/src/application/chat_service/chat_service_handlers.rs`
- `src-tauri/src/infrastructure/agents/claude/stream_processor/types.rs`
- `src-tauri/src/application/team_stream_processor.rs`

Expected changes:

- add normalized event mapper for Claude
- emit provider-neutral payloads on current event names
- carry harness/session/model metadata through run lifecycle events

Required tests:

- Claude parser replay tests
- chat service handler tests
- streaming payload serialization tests

Out of scope:

- no Codex parser yet

## Slice 5. Frontend provider-neutral types and adapters

Goal:

- make the frontend able to consume provider-neutral fields before Codex reaches it

Primary frontend files:

- `frontend/src/types/chat-conversation.ts`
- `frontend/src/types/agent-profile.ts`
- `frontend/src/types/settings.ts`
- `frontend/src/api/chat.ts`
- `frontend/src/api/ideation-model.ts`
- `frontend/src/api/ideation-effort.ts`
- new lane-settings API files

Expected changes:

- add `harness` and `providerSessionId` support
- add lane-settings frontend types
- keep compatibility parsing for old Claude-shaped fields during transition

Required tests:

- `frontend/src/types/agent-profile.test.ts`
- `frontend/src/api/chat.test.ts`
- new lane-settings parser tests

Out of scope:

- no UI redesign yet

## Slice 6. Codex capability detection and spawn config

Goal:

- detect supported local Codex capability surface and build safe spawn configs

Primary backend files:

- new `src-tauri/src/infrastructure/agents/codex/**`
- `src-tauri/src/lib.rs`
- settings/diagnostics command surfaces

Expected changes:

- Codex binary discovery
- version/help-surface capability detection
- runtime config translation for:
  - model
  - reasoning effort
  - approval policy
  - sandbox mode
  - MCP server injection

Required tests:

- capability detection tests
- config translation tests
- missing-binary / degraded-capability tests

Out of scope:

- no user-facing Codex ideation flow yet

## Slice 7. Codex raw logging and parser

Goal:

- ingest Codex raw events and normalize them

Primary backend files:

- new Codex parser/logging modules
- shared event persistence surface
- chat-service integration boundary

Expected changes:

- raw Codex JSONL/event logging
- parser for envelope + nested items
- normalized events for:
  - assistant text
  - tool calls
  - command execution
  - errors
  - delegation/subagents
  - usage

Required tests:

- replay tests using representative Codex fixtures
- parser normalization tests shared with Claude path

Out of scope:

- no settings UI routing yet

## Slice 8. Lane settings UI and API adoption

Goal:

- expose the new per-lane harness/model/effort settings to users

Primary frontend files:

- settings registry/views
- new lane-settings hooks
- ideation settings components

Likely frontend files:

- `frontend/src/components/settings/IdeationModelSection.tsx`
- `frontend/src/components/settings/SettingsDialog.tsx`
- `frontend/src/components/settings/SettingsView.tsx`
- `frontend/src/hooks/useIdeationModelSettings.ts`
- new `useAgentLaneSettings.ts`

Expected changes:

- new lane-based harness/model/effort controls
- explicit Codex capability warnings
- keep old ideation settings hidden or read-only during transition if necessary

Required tests:

- hook tests
- settings view tests
- transform tests

Out of scope:

- no execution-lane Codex activation yet

## Slice 9. Codex ideation + verification routing

Goal:

- first functional milestone requested by the user

Primary backend files:

- `src-tauri/src/application/chat_service/mod.rs`
- `src-tauri/src/application/chat_service/chat_service_context.rs`
- `src-tauri/src/commands/ideation_commands/**`
- `src-tauri/src/http_server/handlers/ideation/**`
- `src-tauri/src/http_server/handlers/session_linking/**`

Expected changes:

- resolve selected harness per ideation lane
- route Claude or Codex harness accordingly
- Codex solo-mode ideation and verifier runs
- Codex-native subagent support for those flows

Required tests:

- ideation runtime tests
- verification lifecycle tests
- queue/continuation tests
- frontend chat widget tests for Codex-backed runs

Out of scope:

- execution/review/merge Codex lanes

## Slice 10. Recovery and reconciliation parity

Goal:

- make Codex-backed ideation safe across pause/stop/restart/recovery flows

Primary backend files:

- `src-tauri/src/application/chat_resumption.rs`
- `src-tauri/src/application/startup_jobs.rs`
- `src-tauri/src/application/reconciliation/**`
- `src-tauri/src/application/chat_service/chat_service_errors.rs`
- `src-tauri/src/commands/execution_commands/**`

Expected changes:

- provider-aware stale session handling
- provider-aware resume repair
- provider-aware reconciliation classification
- pause/stop/queued-message parity

Required tests:

- chat resumption tests
- startup recovery tests
- reconciliation tests

Out of scope:

- no execution-lane Codex rollout

## Slice 11. Cleanup and compatibility retirement plan

Goal:

- define, but do not immediately execute, the retirement of old Claude-shaped surfaces

Primary surfaces:

- `claude_session_id`
- `claudeSessionId`
- `claudeCode`
- old ideation model/effort endpoints

Expected output:

- a removal checklist only after Claude and Codex both run cleanly through the new contracts

## Suggested implementation order

1. Slice 1
2. Slice 2
3. Slice 3
4. Slice 4
5. Slice 5
6. Slice 6
7. Slice 7
8. Slice 8
9. Slice 9
10. Slice 10
11. Slice 11

## Commit guidance

Recommended commit granularity:

- one commit per completed slice where practical
- if a slice is too large, split into:
  - schema/repo
  - runtime/backend
  - frontend/tests
