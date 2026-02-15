# Agent Teams Implementation Tracker

**Started:** 2026-02-15
**Base Branch:** `feat/agent-teams`
**Status:** IN PROGRESS

## Git Worktrees

| Worktree | Branch | Worker | Focus | Status |
|----------|--------|--------|-------|--------|
| `ralphx-wt-config` | `feat/agent-teams-config` | rust-config | YAML config, process_mapping, team_constraints, extends | IN PROGRESS (running clippy) |
| `ralphx-wt-services` | `feat/agent-teams-services` | rust-services | TeamStateTracker, artifact model, ChatService, IPC commands | DONE (`a6dc8716`) |
| `ralphx-wt-mcp` | `feat/agent-teams-mcp` | mcp-prompts | MCP server changes + agent prompt files | DONE (`d83ad3de`, 37 tests) |
| `ralphx-wt-frontend` | `feat/agent-teams-frontend` | frontend | Stores, events, hooks, UI components | DONE impl (`e782af77`), tests in progress (10 files) |

## Phase 1A: Foundation (Parallel)

### rust-config (Opus)
- [ ] `ProcessSlot`, `ProcessMapping` structs in `agent_config/`
- [ ] `TeamConstraints`, `TeamConstraintsConfig`, `TeamMode` structs
- [ ] `resolve_process_agent()` function
- [ ] Agent config inheritance (`extends` field + `resolve_agent_extends()`)
- [ ] `process_mapping` section in `ralphx.yaml`
- [ ] `team_constraints` section in `ralphx.yaml`
- [ ] Fallback to hardcoded constants when config absent
- [ ] `validate_team_plan()` constraint enforcement
- [ ] Environment variable overrides for constraints
- [ ] Unit tests for all new structs and functions

### rust-services (Opus)
- [ ] `TeamStateTracker` service (`team_state_tracker.rs`)
- [ ] `TeammateHandle` with child process, stdin pipe, stream task
- [ ] Artifact model: 3 new `ArtifactType` variants (TeamResearch, TeamAnalysis, TeamSummary)
- [ ] Artifact model: `team-findings` system bucket
- [ ] Artifact model: `TeamArtifactMetadata` in `metadata_json`
- [ ] `TeamStatusResponse`, `TeammateStatusResponse` types
- [ ] ChatService `send_message()` extension for team mode
- [ ] `resolve_agent()` extended with `team_mode` parameter
- [ ] `StreamProcessorConfig` team extensions
- [ ] Interactive mode support in `ClaudeCodeClient` (no `-p` flag path)
- [ ] 6 new Tauri IPC commands (`get_team_status`, `send_team_message`, etc.)
- [ ] Team event emission (`team:created`, `team:teammate_spawned`, etc.)
- [ ] `request_teammate_spawn` HTTP handler
- [ ] `team_sessions` DB table for resume support
- [ ] `team_messages` DB table
- [ ] Unit tests

### mcp-prompts (Sonnet) — DONE ✓
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

### frontend (Opus)
- [ ] `team:*` event constants in `src/lib/events.ts`
- [ ] `teamStore.ts` (Zustand + immer)
- [ ] `chatStore.ts` extension (`isTeamActive`)
- [ ] `src/api/team.ts` (Tauri invoke wrappers)
- [ ] `useTeamEvents.ts` hook
- [ ] `useTeamStatus.ts` hook (TanStack Query)
- [ ] `useTeamActions.ts` hook
- [ ] `ChatContextConfig` extension (`supportsTeamMode`)
- [ ] `TeamActivityPanel.tsx`
- [ ] `TeammateCard.tsx`
- [ ] `TeamFilterTabs.tsx`
- [ ] `TargetSelector.tsx`
- [ ] `TeamMessageBubble.tsx`
- [ ] `TeamSystemEvent.tsx`
- [ ] `TeamCostDisplay.tsx`
- [ ] `ChatPanel.tsx` team mode extension
- [ ] `IntegratedChatPanel.tsx` team mode
- [ ] `MessageItem.tsx` teammate color border
- [ ] `ExecutionTaskDetail.tsx` multi-track progress

## Phase 1B: Integration Testing
- [ ] Merge all feature branches into `feat/agent-teams`
- [ ] Resolve merge conflicts
- [ ] Cargo clippy + cargo test --lib
- [ ] npm run lint
- [ ] Manual integration testing

## Phase 2: Split-Pane UI (Future)
- [ ] `splitPaneStore.ts`
- [ ] `TeamSplitView.tsx` + child components
- [ ] `useTeamKeyboardNav.ts` (Ctrl+B prefix)
- [ ] `usePaneEvents.ts`
- [ ] Responsive breakpoints
- [ ] Display mode toggle

## Integration Points (Cross-Worker Dependencies)

| Dependency | Producer | Consumer | Notes |
|-----------|----------|----------|-------|
| `TeamConstraints` types | rust-config | rust-services | Services needs config types for validation |
| `ArtifactType` enum | rust-services | mcp-prompts | MCP handlers reference new artifact types |
| Team event payloads | rust-services | frontend | Frontend event types must match backend emission |
| MCP tool names | mcp-prompts | rust-services | HTTP handler routing must match tool definitions |
| `TeamStatusResponse` shape | rust-services | frontend | API response types must align |
