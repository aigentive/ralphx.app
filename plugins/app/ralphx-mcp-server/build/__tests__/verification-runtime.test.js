import { afterEach, describe, expect, it, vi } from "vitest";
import { createVerificationRuntime } from "../verification-runtime.js";
afterEach(() => {
    vi.useRealTimers();
});
describe("verification runtime parent resolution", () => {
    it("remaps the active verifier child session id to the canonical parent session", async () => {
        const callTauri = vi.fn();
        const callTauriGet = vi.fn(async (endpoint) => {
            if (endpoint === "parent_session_context/child-session") {
                return {
                    parent_session: {
                        id: "parent-session",
                    },
                };
            }
            throw new Error(`unexpected endpoint ${endpoint}`);
        });
        const runtime = createVerificationRuntime({
            callTauri,
            callTauriGet,
            agentType: "ralphx-plan-verifier",
            contextType: "ideation",
            contextId: "child-session",
        });
        await expect(runtime.resolveVerifierParentSessionId("child-session", "run_verification_enrichment")).resolves.toBe("parent-session");
    });
    it("remaps delegated verification publishers to the parent ideation session", async () => {
        const callTauri = vi.fn();
        const callTauriGet = vi.fn(async (endpoint) => {
            if (endpoint === "coordination/delegated-session/delegated-session/status") {
                return {
                    session: {
                        id: "delegated-session",
                        parent_context_type: "ideation",
                        parent_context_id: "parent-session",
                    },
                };
            }
            throw new Error(`unexpected endpoint ${endpoint}`);
        });
        const runtime = createVerificationRuntime({
            callTauri,
            callTauriGet,
            agentType: "ralphx-plan-critic-completeness",
            contextType: "delegation",
            contextId: "delegated-session",
        });
        await expect(runtime.resolveVerificationFindingSessionId(undefined, "publish_verification_finding")).resolves.toBe("parent-session");
    });
});
describe("verification runtime settlement and terminal cleanup", () => {
    it("keeps timed-out required critics pending when they are still running and clamps the wait budget to the tool-safe cap", async () => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date("2026-04-13T16:35:53.000Z"));
        const callTauri = vi.fn(async (endpoint) => {
            if (endpoint === "coordination/delegate/wait") {
                return {
                    job_id: "job-1",
                    status: "running",
                    delegated_status: {
                        agent_state: {
                            estimated_status: "running",
                        },
                        latest_run: {
                            status: "running",
                        },
                    },
                };
            }
            throw new Error(`unexpected endpoint ${endpoint}`);
        });
        const callTauriGet = vi.fn(async (endpoint) => {
            if (endpoint.startsWith("team/verification-findings/")) {
                return {
                    findings: [],
                    count: 0,
                };
            }
            throw new Error(`unexpected endpoint ${endpoint}`);
        });
        const runtime = createVerificationRuntime({
            callTauri,
            callTauriGet,
            agentType: "ralphx-plan-verifier",
            contextType: "ideation",
            contextId: "child-session",
        });
        const settlementPromise = runtime.awaitVerificationRoundSettlement({
            session_id: "parent-session",
            delegates: [
                {
                    job_id: "job-1",
                    artifact_prefix: "Completeness: ",
                    label: "completeness",
                    required: true,
                },
            ],
            created_after: "2026-04-13T16:35:54.802Z",
            rescue_budget_exhausted: true,
            include_full_content: false,
            include_messages: false,
            message_limit: 1,
            max_wait_ms: 600000,
            poll_interval_ms: 1000,
        });
        await vi.advanceTimersByTimeAsync(91_000);
        const result = await settlementPromise;
        expect(result).toMatchObject({
            classification: "pending",
            timed_out: true,
            settled: false,
            max_wait_ms: 90000,
            recommended_next_action: "perform_single_rescue_or_wait",
            missing_required_prefixes: ["Completeness: "],
        });
    });
    it("routes verifier terminal cleanup with missing round context to infra-failure instead of persisting a zero-gap verdict", async () => {
        const callTauri = vi.fn(async (endpoint, payload) => ({
            endpoint,
            payload,
        }));
        const callTauriGet = vi.fn(async (endpoint) => {
            if (endpoint === "parent_session_context/child-session") {
                return {
                    parent_session: {
                        id: "parent-session",
                    },
                };
            }
            throw new Error(`unexpected endpoint ${endpoint}`);
        });
        const runtime = createVerificationRuntime({
            callTauri,
            callTauriGet,
            agentType: "ralphx-plan-verifier",
            contextType: "ideation",
            contextId: "child-session",
        });
        const result = await runtime.completePlanVerificationForTool({
            session_id: "child-session",
            status: "needs_revision",
            convergence_reason: "agent_error",
            generation: 6,
            required_delegates: [],
            created_after: "2026-04-13T16:35:54.802Z",
        });
        expect(callTauri).toHaveBeenCalledTimes(1);
        expect(callTauri).toHaveBeenCalledWith("ideation/sessions/parent-session/verification/infra-failure", {
            generation: 6,
            convergence_reason: "agent_error",
            round: undefined,
        });
        expect(result).toMatchObject({
            endpoint: "ideation/sessions/parent-session/verification/infra-failure",
        });
    });
});
//# sourceMappingURL=verification-runtime.test.js.map