# Agent Teams Implementation Tracker

**Started:** 2026-02-15
**Base Branch:** `feat/agent-teams`
**Status:** IN PROGRESS

## Git Worktrees

### Phase 1A (MERGED into feat/agent-teams)
| Worktree | Branch | Worker | Commit | Status |
|----------|--------|--------|--------|--------|
| `ralphx-wt-config` | `feat/agent-teams-config` | rust-config (Opus) | `75905584` | DONE — 47 tests |
| `ralphx-wt-services` | `feat/agent-teams-services` | rust-services (Opus) | `a6dc8716` | DONE — 21 tests |
| `ralphx-wt-mcp` | `feat/agent-teams-mcp` | mcp-prompts (Sonnet) | `d83ad3de` | DONE — 37 tests |
| `ralphx-wt-frontend` | `feat/agent-teams-frontend` | frontend (Opus) | `77d1443c` | DONE — 91 tests |

### Phase 1B (IN PROGRESS — fix branches)
| Worktree | Branch | Worker | Focus | Status |
|----------|--------|--------|-------|--------|
| `ralphx-wt-rust` | `feat/agent-teams-rust-fixes` | rust-fixes (Opus) | Services gaps: ChatService, events, HTTP handler, interactive mode | IN PROGRESS |
| `ralphx-wt-front` | `feat/agent-teams-frontend-fixes` | frontend-fixes (Sonnet) | 4 wiring bugs: filter, target selector, teammate props, CSS | IN PROGRESS |
| `ralphx-wt-verify` | `feat/agent-teams-verify` | verifier (Sonnet) | MCP build + Rust check/clippy/test + Frontend lint/test | BLOCKED by fixes |

## Phase 1A: Foundation (Parallel)

### rust-config (Opus) — DONE ✓
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

### rust-services (Opus) — PARTIAL (gaps being fixed in Phase 1B)
- [x] `TeamStateTracker` service (`team_state_tracker.rs`, 965 lines)
- [x] `TeammateHandle` with child process, stream task
- [x] Artifact model: 3 new `ArtifactType` variants (TeamResearch, TeamAnalysis, TeamSummary)
- [x] Artifact model: `team-findings` system bucket
- [x] `TeamStatusResponse`, `TeammateStatusResponse` types
- [x] 6 new Tauri IPC commands (`get_team_status`, `send_team_message`, etc.)
- [x] `team_sessions` DB table for resume support
- [x] `team_messages` DB table
- [x] Unit tests (21 tests)
- [ ] ChatService `send_message()` extension for team mode → **Phase 1B rust-fixes**
- [ ] `resolve_agent()` extended with `team_mode` parameter → **Phase 1B rust-fixes**
- [ ] `StreamProcessorConfig` team extensions → **Phase 1B rust-fixes**
- [ ] Interactive mode support in `ClaudeCodeClient` (stdin pipe) → **Phase 1B rust-fixes**
- [ ] Team event emission (payloads defined, no emit calls) → **Phase 1B rust-fixes**
- [ ] `request_teammate_spawn` HTTP handler → **Phase 1B rust-fixes**
- [ ] `TeamArtifactMetadata` DB persistence → **Phase 1B rust-fixes**

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

### frontend (Opus) — DONE ✓ (wiring bugs being fixed in Phase 1B)
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
- [ ] Wire TeamFilterTabs to actually filter messages → **Phase 1B frontend-fixes**
- [ ] Wire TargetSelector to send handler → **Phase 1B frontend-fixes**
- [ ] Pass teammate props to MessageItem from ChatMessageList → **Phase 1B frontend-fixes**
- [ ] Fix `focusRingColor` invalid CSS → **Phase 1B frontend-fixes**

## Phase 1B: Integration + Fixes (IN PROGRESS)

### Merge (DONE)
- [x] Merge all 4 feature branches into `feat/agent-teams`
- [x] Resolve merge conflicts (agent prompt duplicates between mcp + config)

### Services Gap Fixes (rust-fixes worker — IN PROGRESS)
- [ ] ChatService `send_message()` extension for team mode
- [ ] `resolve_agent()` with `team_mode` parameter
- [ ] Interactive mode in ClaudeCodeClient (stdin pipe management)
- [ ] Team event emission (`app_handle.emit()` for all 7 event types)
- [ ] `request_teammate_spawn` HTTP handler
- [ ] `StreamProcessorConfig` team extensions (teammate_name, teammate_color)
- [ ] `TeamArtifactMetadata` DB persistence

### Frontend Wiring Fixes (frontend-fixes worker — IN PROGRESS)
- [ ] Fix `focusRingColor` invalid CSS in TeamFilterTabs
- [ ] Wire TeamFilterTabs filter to actually filter messages
- [ ] Wire TargetSelector value into send/queue handlers
- [ ] Pass teammateName/teammateColor to MessageItem from ChatMessageList

### Verification (verifier worker — BLOCKED by fixes)
- [ ] MCP server: `npm run build` + `npm test`
- [ ] Rust: `cargo check` + `cargo clippy` + `cargo test --lib`
- [ ] Frontend: `npm run lint` + `npx vitest run`

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

## Integration Points (Cross-Worker Dependencies)

| Dependency | Producer | Consumer | Notes |
|-----------|----------|----------|-------|
| `TeamConstraints` types | rust-config | rust-services | Services needs config types for validation |
| `ArtifactType` enum | rust-services | mcp-prompts | MCP handlers reference new artifact types |
| Team event payloads | rust-services | frontend | Frontend event types must match backend emission |
| MCP tool names | mcp-prompts | rust-services | HTTP handler routing must match tool definitions |
| `TeamStatusResponse` shape | rust-services | frontend | API response types must align |
