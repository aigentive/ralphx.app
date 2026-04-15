---
paths:
  - "agents/ralphx-ideation/**"
  - "agents/ralphx-ideation-team-lead/**"
  - "agents/ralphx-plan-verifier/**"
  - "agents/ralphx-plan-critic-*/**"
  - "agents/ralphx-ideation-specialist-*/**"
  - "frontend/src/components/Ideation/Verification*.tsx"
  - "frontend/src/components/Chat/AutoVerificationCard.tsx"
  - "frontend/src/components/Chat/VerificationResultCard.tsx"
  - "frontend/src/components/Chat/verification-tool-calls.ts"
  - "frontend/src/components/Chat/tool-widgets/VerificationWidget.tsx"
  - "frontend/src/hooks/useVerification*.ts"
  - "frontend/src/api/verification.ts"
  - "plugins/app/ralphx-mcp-server/src/verification-*.ts"
  - "src-tauri/src/http_server/handlers/ideation/verification/**"
  - "src-tauri/src/http_server/handlers/verification/**"
  - "src-tauri/src/application/reconciliation/verification_*.rs"
  - "src-tauri/src/application/chat_service/verification_child_process_registry.rs"
  - "src-tauri/src/infrastructure/sqlite/sqlite_ideation_session_repo.rs"
  - "src-tauri/src/infrastructure/sqlite/migrations/*verification*"
  - "src-tauri/crates/ralphx-domain/src/entities/ideation/types.rs"
  - "src-tauri/src/domain/services/verification_*.rs"
  - "docs/features/plan-verification.md"
  - "docs/external-mcp/**"
---

# Ideation Verification Architecture

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to | ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

**Required Context:** `agent-mcp-tools.md` | `delegation-topology.md` | `multi-harness.md` | `docs/features/plan-verification.md`

## Fast Lookup

| Need | Read First | Why |
|---|---|---|
| Full feature map | `.claude/rules/ideation-verification-architecture.md` | Canonical architecture + file registry for the whole feature |
| Product behavior / public semantics | `docs/features/plan-verification.md` | User-facing workflow, statuses, convergence, gates |
| Parent verification HTTP flow | `src-tauri/src/http_server/handlers/ideation/verification/{query,update,lifecycle}.rs` | Authoritative parent read/update/stop/revert path |
| Start / confirmation path | `src-tauri/src/http_server/handlers/verification/{confirm,helpers}.rs` + `src-tauri/src/http_server/handlers/artifacts/create.rs` | Manual confirm, auto-start, child spawn |
| Native verification store | `src-tauri/crates/ralphx-domain/src/entities/ideation/types.rs` + `src-tauri/src/infrastructure/sqlite/sqlite_ideation_session_repo.rs` | Parent summary fields vs native run snapshots |
| Verifier MCP runtime | `plugins/app/ralphx-mcp-server/src/{verification-runtime,verification-orchestration,verification-round-assessment,verification-completion}.ts` | Delegate dispatch, waits, settlement, completion gating |
| Verifier MCP build artifact | `plugins/app/ralphx-mcp-server/build/{index,verification-runtime}.js` | The desktop app executes the built MCP bundle, so verifier runtime source changes are not live until `cd plugins/app/ralphx-mcp-server && npm run build` refreshes `build/*` |
| Verifier prompt / tool contract | `agents/ralphx-plan-verifier/agent.yaml` + `agents/ralphx-plan-verifier/{claude,codex}/prompt.md` | Live verifier surface and workflow |
| Verification tab | `frontend/src/components/Ideation/VerificationPanel.tsx` + `frontend/src/components/Ideation/VerificationHistory.tsx` | Left-panel lineage/history/gaps |
| Verification chat widgets | `frontend/src/components/Chat/tool-widgets/VerificationWidget.tsx` + `frontend/src/components/Chat/verification-tool-calls.ts` | Right-chat progress/result cards and stale-card collapse |
| Live cache / event hydration | `frontend/src/hooks/useVerificationStatus.ts` + `frontend/src/hooks/useVerificationEvents.ts` | Authoritative query + event invalidation/fast-path updates |
| Recovery / orphan cleanup | `src-tauri/src/application/reconciliation/verification_reconciliation.rs` + `src-tauri/src/application/reconciliation/verification_handoff.rs` | Auto-continue, stale reset, orphan archival |
| External/public MCP verification surface | `docs/external-mcp/*` + `src-tauri/src/http_server/handlers/external/ideation_runtime/verification.rs` | Public polling/start semantics; do not treat these docs as stale internal ballast |

## Non-Negotiables

| Rule | Detail |
|---|---|
| Parent session owns verification truth | Hidden verification children execute the loop; parent ideation session owns `verification_status`, `verification_generation`, native run history, and user-visible result state. |
| Native run store is authoritative | `verification_runs` + `verification_rounds` + `verification_round_gaps` + `verification_run_current_gaps` hold the real run. `ideation_sessions.verification_*` fields are summary / gate fields only. |
| One normal verification flow | Verifier chooses lenses and optional specialists; backend/MCP runtime dispatches delegates, waits, settles, rescues, and blocks terminal completion until required critics settle. |
| Verification tab != child transcript | Left Verification tab shows parent-owned status, gaps, round lineage, and run history. Child/delegate transcript drill-down is supplemental, not the tab’s primary content. |
| Chat widgets are derived UI | Verification chat cards are secondary projections over parent state + delegate truth. If a widget disagrees with parent verification state, fix hydration/normalization instead of trusting raw tool payloads. |
| Child ids auto-remap to parent | Reading or updating verification via a verification child session id should remap to the parent in backend handlers. Do not build separate child-owned verification state. |
| Verifier plan-edit bypass is transport-owned | Verification-child plan edits must derive caller session identity from runtime/transport context, not from a model-supplied `caller_session_id` tool argument. |
| Verifier revisions happen in place | An actionable verifier round should revise the parent plan from the live verification child context; do not wait for child shutdown or terminal cleanup to release the artifact. |
| No verifier self-nudges | Verifier/critics/specialists must not send chat nudges to self, delegates, or parent to keep the loop alive. Runtime/reconciliation own continuation and rescue. |
| Prompt/tool surfaces stay live-only | Prompts, MCP hints, docs, and widgets must describe only the current verification helper surface. No migration diary or removed-tool prose. |

## One-Screen Architecture

```text
Plan create/update or explicit confirm
  → trigger_auto_verify_sync() seeds parent summary + native run snapshot
  → spawn_verification_agent() creates hidden verification child session
  → ralphx-plan-verifier reads parent plan/status
  → run_verification_enrichment() optional specialists
  → run_verification_round() required critics + optional specialists
  → report_verification_round() persists parent round snapshot + emits event
  → verifier revises parent plan if backend says continue
  → complete_plan_verification() finalizes only after settlement barrier
  → reconciliation/handoff handles stale children, auto-continue, archival, restart cleanup
```

## Ownership Model

| Thing | Authority | Files |
|---|---|---|
| Parent summary / gates | `ideation_sessions.verification_status`, `verification_in_progress`, `verification_generation`, `verification_confirmation_status`, summary counters | `src-tauri/src/infrastructure/sqlite/sqlite_ideation_session_repo.rs`, `src-tauri/crates/ralphx-domain/src/entities/ideation/types.rs` |
| Full run lineage | `VerificationRunSnapshot` + `VerificationRoundSnapshot` persisted in native verification tables keyed by `session_id + generation` | `types.rs`, `sqlite_ideation_session_repo.rs`, `v20260414060000_verification_run_store.rs` |
| Execution host | Hidden ideation child session with `SessionPurpose::Verification`; used for agent runtime/process ownership, not authoritative status | `src-tauri/src/http_server/handlers/verification/helpers.rs`, `src-tauri/src/application/chat_service/verification_child_process_registry.rs` |
| Verifier orchestration | Verifier agent prompt + MCP runtime helpers; backend-owned settlement/completion | `agents/ralphx-plan-verifier/**`, `plugins/app/ralphx-mcp-server/src/verification-*.ts` |
| Required critics | Completeness + implementation-feasibility critics; required for terminal completion | `agents/ralphx-plan-critic-*/**`, `plugins/app/ralphx-mcp-server/src/verification-runtime.ts` |
| Optional specialists | Model-selected lenses; backend dispatches and bounds waits | `agents/ralphx-ideation-specialist-*/**`, `src-tauri/src/http_server/handlers/verification/specialist_registry.rs`, `plugins/app/ralphx-mcp-server/src/verification-orchestration.ts` |
| Left Verification tab | Parent snapshot/history/lineage surface | `frontend/src/components/Ideation/VerificationPanel.tsx`, `VerificationHistory.tsx`, `VerificationGapList.tsx` |
| Right chat transcript | Child/verifier narrative + structured verification cards + result cards | `frontend/src/components/Chat/tool-widgets/VerificationWidget.tsx`, `frontend/src/components/Chat/verification-tool-calls.ts`, `VerificationResultCard.tsx`, `AutoVerificationCard.tsx` |

## Backend File Map

| Layer | Primary Files | Responsibility |
|---|---|---|
| Domain types | `src-tauri/crates/ralphx-domain/src/entities/ideation/types.rs` | `VerificationStatus`, `VerificationGap`, `VerificationRunSnapshot`, `VerificationRoundSnapshot` |
| Domain services | `src-tauri/src/domain/services/verification_gate.rs`, `src-tauri/src/domain/services/verification_events.rs` | Gate semantics and emitted payload shaping |
| Persistence | `src-tauri/src/infrastructure/sqlite/sqlite_ideation_session_repo.rs` | Parent summary mutations, native run snapshot load/save, child lookup, trigger/reset paths |
| Schema | `src-tauri/src/infrastructure/sqlite/migrations/v20260414060000_verification_run_store.rs` | Native verification tables |
| Start / confirmation | `src-tauri/src/http_server/handlers/verification/{confirm,helpers,auto_accept,dismiss,pending_confirmations,specialist_registry}.rs` | Manual confirmation flow, spawn, specialist registry, confirmation queue |
| Auto-start from plan create | `src-tauri/src/http_server/handlers/artifacts/create.rs` | Creates plan artifact, decides auto-verify, seeds run, spawns verifier or emits pending confirmation |
| Authoritative read | `src-tauri/src/http_server/handlers/ideation/verification/query.rs` | Parent snapshot read, child→parent remap, child continuity metadata |
| Authoritative update | `src-tauri/src/http_server/handlers/ideation/verification/update.rs` | Parent status machine, generation guard, native snapshot persistence, re-verify path |
| Lifecycle controls | `src-tauri/src/http_server/handlers/ideation/verification/lifecycle.rs` | Stop, revert-and-skip, infra-failure routing, child stop/archive |
| Recovery / reconciliation | `src-tauri/src/application/reconciliation/{verification_reconciliation,verification_handoff}.rs` | Stale reset, orphan cleanup, child-complete handling, auto-continue decisions |
| Runtime process tracking | `src-tauri/src/application/chat_service/verification_child_process_registry.rs` | Tracks live verification child ownership/state |
| External/public verification | `src-tauri/src/http_server/handlers/external/ideation_runtime/verification.rs` | External MCP read/start contract |

## Verifier Runtime Map

| Concern | Files | Notes |
|---|---|---|
| Live tool contract | `agents/ralphx-plan-verifier/agent.yaml` | Canonical MCP surface + allowed delegation targets |
| Claude/Codex prompts | `agents/ralphx-plan-verifier/{claude,codex}/prompt.md` | Workflow contract; should describe runtime-owned helpers, not manual bookkeeping |
| Required/optional delegate dispatch | `plugins/app/ralphx-mcp-server/src/verification-orchestration.ts` | Starts critics/specialists and gathers delegate snapshots |
| Settlement barrier | `plugins/app/ralphx-mcp-server/src/verification-runtime.ts`, `src-tauri/src/http_server/handlers/ideation/verification/update.rs`, `src-tauri/src/application/reconciliation/verification_reconciliation.rs` | Runtime waits on critics, but the active round barrier is also durably mirrored into the native verification snapshot before waiting so reconciliation/orphan cleanup do not depend on verifier-process memory alone |
| Typed finding aggregation | `plugins/app/ralphx-mcp-server/src/verification-round-assessment.ts` | Merges typed findings, classifies delegate/finding coverage |
| Terminal completion guard | `plugins/app/ralphx-mcp-server/src/verification-completion.ts` | Rejects premature `needs_revision` / `verified` terminalization, routes infra failures |

## Frontend Surface Map

| Surface | Primary Files | Contract |
|---|---|---|
| Verification tab | `frontend/src/components/Ideation/VerificationPanel.tsx` | Reads parent verification query and renders status, actions, active run picker, history |
| Round lineage | `frontend/src/components/Ideation/VerificationHistory.tsx` | Gap score trend + addressed/remaining round lineage |
| Gap list / badge | `frontend/src/components/Ideation/{VerificationGapList,VerificationBadge}.tsx` | Current gap/status presentation |
| Confirmation UX | `frontend/src/api/verification.ts`, `frontend/src/hooks/useVerificationBootstrap.ts`, `frontend/src/components/Ideation/VerificationConfirmDialog.tsx` | Pending confirmation queue + specialist opt-out UI |
| Authoritative data hook | `frontend/src/hooks/useVerificationStatus.ts` | Parent session query key = `["verification", sessionId]` |
| Live event hydration | `frontend/src/hooks/useVerificationEvents.ts` | Consumes `plan_verification:status_changed` + pending confirmation events; updates cache/store |
| Chat widget rendering | `frontend/src/components/Chat/tool-widgets/VerificationWidget.tsx` | Structured cards for enrichment/round/report/completion/get-status tools |
| Chat stale-card collapse | `frontend/src/components/Chat/verification-tool-calls.ts` | Removes obsolete enrichment/round cards when later report/completion blocks arrive |
| Result summary cards | `frontend/src/components/Chat/{AutoVerificationCard,VerificationResultCard}.tsx` | User-facing result summaries in chat |
| Child transcript drill-down | `frontend/src/components/Ideation/VerificationChildTranscript.tsx` | Supplemental transcript viewer only; not the primary Verification tab content |

## Start Paths

| Trigger | Files | Notes |
|---|---|---|
| Auto-start after plan create/update | `handlers/artifacts/create.rs` | Plan creation can call `trigger_auto_verify_sync()` and spawn verifier immediately |
| Explicit user confirm | `handlers/verification/confirm.rs` | Sets `verification_confirmation_status = accepted`, triggers auto-verify, spawns verifier |
| Re-verify existing plan | `handlers/ideation/verification/update.rs` | Terminal → `reviewing` path resets stale metadata, bumps generation, persists cleared snapshot |
| Skip / revert | `handlers/ideation/verification/lifecycle.rs` | Parent-owned lifecycle controls; stop/archive verification children |

## State Model

| State/Data | Meaning | Notes |
|---|---|---|
| `verification_status` | Parent summary state machine: `unverified → reviewing → verified / needs_revision / skipped` | UI gates and accept/proposal guards read this |
| `verification_in_progress` | Parent active flag | Do not infer activeness only from chat widgets |
| `verification_generation` | Zombie guard / run id on parent | Child/verifier writes must match current generation |
| `verification_confirmation_status` | Pending/accepted/rejected confirm queue state | Separate from verification outcome |
| `VerificationRunSnapshot` | Full current run state | One row per parent session + generation |
| `VerificationRoundSnapshot` | Per-round gap lineage | Chronological within a run |
| Verification child session | Hidden execution session | Execution host only; status reads/updates should remap to parent |

## Event + Hydration Rules

| Rule | Why |
|---|---|
| Emit / consume `plan_verification:status_changed` as the authoritative live signal | Keeps left tab and parent summary in sync with backend state |
| Fast-path cache updates may win over an immediate refetch when persistence lags | Prevents same-turn stale overwrites in the Verification tab |
| Chat tool cards must be normalized against later report/completion blocks | Prevents stale `Running` enrichment/round cards from pinning the transcript |
| Child/delegate runtime truth should come from backend session/delegate state, not old tool payloads | Avoids stale `Working...` / `unknown` cards |

## Common Failure Modes

| Symptom | First Places To Check |
|---|---|
| Verification tab shows transcript or loses lineage | `VerificationPanel.tsx`, `VerificationHistory.tsx`, `useVerificationStatus.ts` |
| Chat shows stale `Verification enrichment` / `Verification round` as running | `verification-tool-calls.ts`, `VerificationWidget.tsx`, `useVerificationEvents.ts` |
| Parent state looks right but widget/result card looks wrong | `VerificationWidget.tsx` should prefer live `useVerificationStatus()` over stale raw tool payloads |
| Child archived or reset unexpectedly | `verification_reconciliation.rs`, `verification_handoff.rs`, backend logs for child→parent remap / orphan archival |
| Verifier tries to self-message / nudge delegates | `agents/ralphx-plan-verifier/**` plus `agent.yaml`; verifier surface should not expose chat-nudge tools |
| Required critics finish late or terminal result races them | `verification-runtime.ts`, `verification-completion.ts`, `update.rs` |
| External/public API behavior diverges from internal assumptions | `docs/external-mcp/*` + `handlers/external/ideation_runtime/verification.rs` |

## Tests By Layer

| Layer | Primary Tests |
|---|---|
| Domain / event shaping | `src-tauri/src/domain/services/verification_gate_tests.rs`, `verification_events_tests.rs` |
| Backend handlers / integration | `src-tauri/tests/ideation_runtime_handlers.rs`, `external_handlers.rs`, `src-tauri/src/tests/verification_loop_integration_tests.rs` |
| Reconciliation / handoff | `src-tauri/src/application/reconciliation/verification_reconciliation_tests.rs`, `verification_handoff_tests.rs` |
| MCP runtime | `plugins/app/ralphx-mcp-server/src/__tests__/verification-runtime.test.ts`, `verification-round-assessment.test.ts`, `verification-completion.test.ts`, `tools.test.ts` |
| Frontend tab / widgets | `frontend/src/components/Ideation/VerificationPanel.test.tsx`, `VerificationHistory.test.tsx`, `frontend/src/components/Chat/tool-widgets/VerificationWidget.test.tsx`, `frontend/src/components/Chat/verification-tool-calls.test.ts`, `frontend/src/hooks/useVerificationEvents.test.ts` |

## Change Checklist

| If You Change | Also Update |
|---|---|
| Verifier MCP tools or prompt workflow | `agents/ralphx-plan-verifier/**`, `agent-mcp-tools.md`, `plugins/app/ralphx-mcp-server/src/tools.ts`, MCP tests |
| `plugins/app/ralphx-mcp-server/src/verification-*.ts` | Rebuild `plugins/app/ralphx-mcp-server/build/*` with `npm run build` before smoke/commit; the app executes the compiled bundle, not the TypeScript source |
| Parent verification payload shape | Rust handler/response types + frontend query/event consumers + widget tests |
| Native snapshot schema | Migration + SQLite repo load/save + domain types + integration tests |
| Verification tab behavior | `VerificationPanel.tsx` + `VerificationHistory.tsx` + chat/widget surfaces if the same state also appears in transcript |
| Chat widget semantics | `VerificationWidget.tsx` + `verification-tool-calls.ts` + live status hooks/tests |
| External/public verification semantics | `docs/external-mcp/*` + external handler tests/docs |
