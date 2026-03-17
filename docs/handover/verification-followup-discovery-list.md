# Verification Follow-up Discovery List

Scope: validated follow-up issues similar to the stale verification badge bug. This list now keeps only items that are reachable in current code, or explicitly marks narrower scope where the first pass was too broad.

## 1. Confirmed High — `max_rounds` resolves to different terminal statuses depending on the code path

Evidence:
- `src-tauri/src/http_server/handlers/ideation.rs:1062-1075` forces `max_rounds` to `VerificationStatus::Verified`
- `src-tauri/src/application/reconciliation/verification_reconciliation.rs:720-737` maps `max_rounds` to `VerificationStatus::NeedsRevision`
- `src-tauri/src/http_server/handlers/ideation_tests.rs:1497-1504` and `src-tauri/src/tests/verification_loop_integration_tests.rs:260-292` expect `Verified`
- `src-tauri/src/application/reconciliation/verification_reconciliation_tests.rs:498-502` expects `NeedsRevision`
- Targeted tests both pass today, proving the split is real:
  - `cargo test --manifest-path src-tauri/Cargo.toml test_max_rounds_exit_behavior --lib`
  - `cargo test --manifest-path src-tauri/Cargo.toml test_reconcile_child_complete_max_rounds_needs_revision --lib`

Why it matters:
- The same verification run can end in different terminal states depending on whether the normal round loop finishes cleanly or reconciliation owns the final transition.
- Proposal gating, badge state, and recovery behavior can disagree after restarts or child-session completion.

Follow-up:
- Pick one canonical `max_rounds` outcome, then align handler logic, reconciliation logic, tests, UI copy, and agent docs around it.

## 2. Confirmed Medium — `agent_crashed_mid_round` is user-visible but has no frontend label

Evidence:
- `src-tauri/src/application/reconciliation/verification_reconciliation.rs:469-477` stores `agent_crashed_mid_round` and sets status `NeedsRevision`
- `src/components/Ideation/VerificationBadge.tsx:79-86` only labels `zero_blocking`, `jaccard_converged`, `max_rounds`, `critic_parse_failure`, `user_skipped`, `user_reverted`
- `src/components/Ideation/VerificationBadge.tsx:112-114` falls back to the raw string when a label is missing
- `src/components/Ideation/VerificationHistory.tsx:33-40` has the same reduced label map
- `src/components/Ideation/VerificationHistory.tsx:214-216` also falls back to the raw string
- `src/components/Ideation/VerificationPanel.tsx:557-807` renders the badge/history path for non-`unverified` states, so `NeedsRevision + agent_crashed_mid_round` reaches the visible UI

Narrowing from the first pass:
- `app_restart` and `agent_completed_without_update` are not confirmed as visible badge/history bugs because `VerificationPanel` stays in the unverified empty state when status is `unverified` and there is no round history (`src/components/Ideation/VerificationPanel.tsx:319`, `src/components/Ideation/VerificationPanel.tsx:464`)

Why it matters:
- When reconciliation salvages a partially completed verification run, the user sees a raw internal code instead of a readable explanation at exactly the moment they are debugging a failed verification.

Follow-up:
- Introduce one shared reason-to-label mapping for verification UI and add a test covering `agent_crashed_mid_round`.

## 3. Confirmed Medium — verifier prompt error cleanup disagrees with the backend transition rules

Evidence:
- `ralphx-plugin/agents/plan-verifier.md:252` tells the agent to do final cleanup with `status: "reviewing"`, `in_progress: false`, `convergence_reason: "agent_error"`
- `ralphx-plugin/ralphx-mcp-server/src/plan-tools.ts:113-161` allows that tool payload shape
- `src-tauri/src/http_server/handlers/ideation.rs:835-850` does not allow `Reviewing -> Reviewing`; that transition is rejected with 422
- `src-tauri/src/http_server/handlers/ideation_tests.rs:1695` explicitly notes `reviewing -> reviewing` must not be called because it returns 422
- `src-tauri/src/application/reconciliation/verification_reconciliation.rs:559-690` shows the backend’s actual error/stop recovery path resets to `Unverified` with `agent_error` / `user_stopped`, not `reviewing`

Why it matters:
- If the verifier follows its own prompt on a non-retriable MCP error, the cleanup call is invalid against the backend state machine.
- That leaves error handling to fallback reconciliation instead of the agent’s intended terminal update path.

Follow-up:
- Make the prompt/tool contract match the backend’s real error terminalization semantics, then add a direct test for the invalid cleanup payload so this cannot drift again.

## Not confirmed from the first pass

- `low_remaining_only` drift is not confirmed as a current bug. It appears only in `src-tauri/src/application/reconciliation/verification_reconciliation.rs:724-735` and does not have an active producer elsewhere in the codebase, so it is dead/legacy contract debt rather than a proven runtime defect today.
