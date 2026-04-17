import { describe, expect, it, vi } from "vitest";
import { completePlanVerificationWithSettlement } from "../verification-completion.js";
describe("completePlanVerificationWithSettlement", () => {
    it("rejects actionable needs_revision cleanup when settled delegates still require bounded revision", async () => {
        const callInfraFailure = vi.fn();
        const callCompletion = vi.fn();
        await expect(completePlanVerificationWithSettlement({
            sessionId: "parent-session",
            body: {
                generation: 1,
                status: "needs_revision",
                convergence_reason: "agent_error",
                round: 2,
            },
            requiredDelegates: [
                {
                    job_id: "job-1",
                    critic: "completeness",
                    label: "completeness",
                    required: true,
                },
            ],
            createdAfter: "2026-04-15T03:47:10.000Z",
            rescueBudgetExhausted: true,
            includeFullContent: true,
            includeMessages: true,
            messageLimit: 5,
            maxWaitMs: 120000,
            pollIntervalMs: 750,
            awaitVerificationRoundSettlement: async () => ({
                classification: "complete",
                missing_required_critics: [],
                recommended_next_action: "continue_round_analysis",
                verification_findings: [
                    {
                        artifact_id: "finding-1",
                        title: "Completeness: Round 2",
                        created_at: "2026-04-15T03:47:12.000Z",
                        critic: "completeness",
                        round: 2,
                        status: "partial",
                        summary: "Need one more revision pass.",
                        gaps: [
                            {
                                severity: "high",
                                category: "scope",
                                description: "The plan still misses a required path.",
                            },
                        ],
                    },
                ],
            }),
            callInfraFailure,
            callCompletion,
        })).rejects.toThrow("Cannot complete verification as needs_revision while backend settlement still requires bounded revision");
        expect(callInfraFailure).not.toHaveBeenCalled();
        expect(callCompletion).not.toHaveBeenCalled();
    });
    it("routes infra-failure settlement to infra-failure cleanup instead of persisting needs_revision", async () => {
        const callInfraFailure = vi.fn(async () => ({
            session_id: "parent-session",
            status: "unverified",
            in_progress: false,
            convergence_reason: "agent_error",
        }));
        const callCompletion = vi.fn(async () => ({
            session_id: "parent-session",
            status: "needs_revision",
            in_progress: false,
        }));
        const result = await completePlanVerificationWithSettlement({
            sessionId: "parent-session",
            body: {
                generation: 1,
                status: "needs_revision",
                convergence_reason: "agent_error",
            },
            requiredDelegates: [
                {
                    job_id: "job-1",
                    critic: "completeness",
                    label: "completeness",
                    required: true,
                },
                {
                    job_id: "job-2",
                    critic: "feasibility",
                    label: "feasibility",
                    required: true,
                },
            ],
            createdAfter: "2026-04-13T16:08:25.075Z",
            rescueBudgetExhausted: true,
            includeFullContent: true,
            includeMessages: true,
            messageLimit: 5,
            maxWaitMs: 120000,
            pollIntervalMs: 750,
            awaitVerificationRoundSettlement: async () => ({
                classification: "infra_failure",
                missing_required_critics: ["completeness"],
            }),
            callInfraFailure,
            callCompletion,
        });
        expect(callInfraFailure).toHaveBeenCalledTimes(1);
        expect(callCompletion).not.toHaveBeenCalled();
        expect(result).toMatchObject({
            session_id: "parent-session",
            status: "unverified",
            in_progress: false,
            settlement: {
                classification: "infra_failure",
            },
        });
    });
});
//# sourceMappingURL=verification-completion.test.js.map