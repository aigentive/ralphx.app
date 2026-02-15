# Agent Teams Implementation Tracker

**Started:** 2026-02-15
**Branch:** `main` (merged 2026-02-15)
**Status:** PHASE 1 COMPLETE — 100 files, 11,847 insertions, 10,373 tests passing

## Summary

| Phase | Status | Workers | Tests |
|-------|--------|---------|-------|
| 1A: Foundation | DONE | 4 workers (3 Opus, 1 Sonnet) | 196 new tests |
| 1B: Fixes + Verification | DONE | 4 workers (2 Opus, 2 Sonnet) + 1 relaunch | 10,373 total passing |
| 1C: Context Relaunch Protocol | PLANNED | — | — |
| 2: Split-Pane UI | FUTURE | — | — |

## Phase 1A: Foundation — DONE ✓

### rust-config (Opus) — `75905584`
- [x] `ProcessSlot`, `ProcessMapping` structs in `agent_config/`
- [x] `TeamConstraints`, `TeamConstraintsConfig`, `TeamMode` structs
- [x] `resolve_process_agent()` function
- [x] Agent config inheritance (`extends` field + `resolve_agent_extends()`)
- [x] `process_mapping` section in `ralphx.yaml`
- [x] `team_constraints` section in `ralphx.yaml`
- [x] Fallback to hardcoded constants when config absent
- [x] `validate_team_plan()` constraint enforcement
- [x] Environment variable overrides for constraints
- [x] Unit tests (47 tests)

### rust-services (Opus) — `a6dc8716`
- [x] `TeamStateTracker` service (`team_state_tracker.rs`, 965 lines)
- [x] `TeammateHandle` with child process, stream task
- [x] Artifact model: 3 new `ArtifactType` variants (TeamResearch, TeamAnalysis, TeamSummary)
- [x] Artifact model: `team-findings` system bucket
- [x] `TeamStatusResponse`, `TeammateStatusResponse` types
- [x] 6 new Tauri IPC commands (`get_team_status`, `send_team_message`, etc.)
- [x] `team_sessions` DB table for resume support
- [x] `team_messages` DB table
- [x] Unit tests (21 tests)

### mcp-prompts (Sonnet) — `d83ad3de`
- [x] `ideation-team-member` allowlist in `tools.ts`
- [x] `worker-team-member` allowlist in `tools.ts`
- [x] `RALPHX_ALLOWED_MCP_TOOLS` env var support in `getToolAllowlist()`
- [x] `request_team_plan` MCP tool definition + handler
- [x] `create_team_artifact` MCP tool (thin wrapper)
- [x] `get_team_artifacts` MCP tool (thin wrapper)
- [x] `get_team_session_state` MCP tool
- [x] `save_team_session_state` MCP tool
- [x] `request_teammate_spawn` MCP tool
- [x] HTTP endpoint routing for new tools
- [x] `ideation-team-lead.md` agent prompt
- [x] `worker-team.md` agent prompt
- [x] `ideation-specialist-frontend.md` template
- [x] `ideation-specialist-backend.md` template
- [x] `ideation-specialist-infra.md` template
- [x] `ideation-advocate.md` template
- [x] `ideation-critic.md` template
- [x] 37 vitest unit tests (`src/__tests__/tools.test.ts`)

### frontend (Opus) — `77d1443c` + `e782af77`
- [x] `team:*` event constants in `src/lib/events.ts`
- [x] `teamStore.ts` (Zustand + immer, 212 lines)
- [x] `chatStore.ts` extension (`isTeamActive`)
- [x] `src/api/team.ts` (Tauri invoke wrappers + Zod schemas)
- [x] `useTeamEvents.ts` hook (219 lines, 9 event types)
- [x] `useTeamStatus.ts` hook (TanStack Query)
- [x] `useTeamActions.ts` hook
- [x] `ChatContextConfig` extension (`supportsTeamMode`)
- [x] `TeamActivityPanel.tsx`
- [x] `TeammateCard.tsx`
- [x] `TeamFilterTabs.tsx`
- [x] `TargetSelector.tsx`
- [x] `TeamMessageBubble.tsx`
- [x] `TeamSystemEvent.tsx`
- [x] `TeamCostDisplay.tsx`
- [x] `ChatPanel.tsx` team mode extension
- [x] `IntegratedChatPanel.tsx` team mode
- [x] `MessageItem.tsx` teammate color border
- [x] `ExecutionTaskDetail.tsx` multi-track progress
- [x] 91 tests across 11 test files

## Phase 1B: Fixes + Verification — DONE ✓

### Services Gap Fixes — `6af25a22` + `1dfe29da`
- [x] ChatService `send_message()` extension for team mode
- [x] `resolve_agent()` with `team_mode` parameter
- [x] Interactive mode in ClaudeCodeClient (stdin pipe management)
- [x] Team event emission (`team_events.rs`, all 7 event types)
- [x] `request_teammate_spawn` HTTP handler (`handlers/teams.rs`)
- [x] `StreamProcessorConfig` team extensions (teammate_name, teammate_color)
- [x] `TeamArtifactMetadata` DB persistence

### Frontend Wiring Fixes — `7dc5fef2`
- [x] Fix `focusRingColor` invalid CSS in TeamFilterTabs
- [x] Wire TeamFilterTabs filter to actually filter messages
- [x] Wire TargetSelector value into send/queue handlers
- [x] Pass teammateName/teammateColor to MessageItem from ChatMessageList

### Verification — `19739af6`
- [x] MCP server: `npm run build` + `npm test` (74 tests passed)
- [x] Rust: `cargo check` + `cargo clippy` + `cargo test --lib` (4837 tests passed)
- [x] Frontend: `npm run lint` + `npx vitest run` (5462 tests passed)

### Context Relaunch (applied during Phase 1B)
- rust-fixes worker hit 7% context → forced handoff → relaunched as rust-fixes-v2
- Protocol documented: `docs/architecture/agent-teams-context-relaunch.md`

## Phase 1C: Context Relaunch Protocol (PLANNED)

**Decision:** `docs/decisions/agent-teams-context-relaunch-protocol.md`
**Architecture:** `docs/architecture/agent-teams-context-relaunch.md`
**Modifications spec:** `docs/architecture/agent-teams-context-relaunch-modifications.md`

### Prompt Changes
- [ ] `ideation-team-lead.md` — Add relaunch protocol section (monitor teammates, handle handoff, respawn)
- [ ] `worker-team.md` — Add relaunch protocol for worker-team coordinator
- [ ] All specialist/advocate/critic prompts — Add self-report one-liner
- [ ] Verify prompts include threshold from config

### Config Changes
- [ ] `TeamConstraints` — Add `context_relaunch_threshold: Option<u8>` (default 20)
- [ ] `ralphx.yaml` — Add `context_relaunch_threshold` to `_defaults`
- [ ] Unit tests for new field

### Artifact Model
- [ ] New `ArtifactType::TeamHandoff` variant for structured handoff summaries
- [ ] Update `team-findings` bucket to accept TeamHandoff

### Observability
- [ ] `TeamStateTracker` — Track `relaunch_count` per teammate slot
- [ ] `TeammateStatusResponse` — Include relaunch_count in API response

## Phase 2: Split-Pane UI (Future)
- [ ] `splitPaneStore.ts`
- [ ] `TeamSplitView.tsx` + child components
- [ ] `useTeamKeyboardNav.ts` (Ctrl+B prefix)
- [ ] `usePaneEvents.ts`
- [ ] Responsive breakpoints
- [ ] Display mode toggle
