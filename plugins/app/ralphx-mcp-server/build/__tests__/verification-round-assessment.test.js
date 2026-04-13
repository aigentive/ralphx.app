import { describe, expect, it } from "vitest";
import { assessVerificationRound } from "../verification-round-assessment.js";
describe("assessVerificationRound", () => {
    it("classifies the round as complete when all required artifacts are present", () => {
        const result = assessVerificationRound({
            delegates: [
                { job_id: "job-1", artifact_prefix: "Completeness: ", required: true },
                { job_id: "job-2", artifact_prefix: "Feasibility: ", required: true },
            ],
            artifactsByPrefix: [
                { prefix: "Completeness: ", found: true, total_matches: 1, artifact: { id: "a1" } },
                { prefix: "Feasibility: ", found: true, total_matches: 1, artifact: { id: "a2" } },
            ],
            delegateSnapshots: [
                { job_id: "job-1", status: "completed" },
                { job_id: "job-2", status: "completed" },
            ],
        });
        expect(result.classification).toBe("complete");
        expect(result.recommended_next_action).toBe("continue_round_analysis");
        expect(result.missing_required_prefixes).toEqual([]);
    });
    it("classifies the round as pending before rescue budget is exhausted when a required delegate is still running", () => {
        const result = assessVerificationRound({
            delegates: [
                { job_id: "job-1", artifact_prefix: "Completeness: ", required: true },
            ],
            artifactsByPrefix: [
                { prefix: "Completeness: ", found: false, total_matches: 0 },
            ],
            delegateSnapshots: [
                {
                    job_id: "job-1",
                    status: "running",
                    delegated_status: {
                        agent_state: {
                            estimated_status: "likely_generating",
                        },
                    },
                },
            ],
            rescueBudgetExhausted: false,
        });
        expect(result.classification).toBe("pending");
        expect(result.recommended_next_action).toBe("perform_single_rescue_or_wait");
        expect(result.missing_required_prefixes).toEqual(["Completeness: "]);
    });
    it("classifies the round as infra failure when rescue budget is exhausted and a required artifact is still missing", () => {
        const result = assessVerificationRound({
            delegates: [
                { job_id: "job-1", artifact_prefix: "Completeness: ", required: true, label: "completeness" },
                { job_id: "job-2", artifact_prefix: "Feasibility: ", required: true, label: "feasibility" },
            ],
            artifactsByPrefix: [
                { prefix: "Completeness: ", found: false, total_matches: 0 },
                { prefix: "Feasibility: ", found: true, total_matches: 1, artifact: { id: "a2" } },
            ],
            delegateSnapshots: [
                {
                    job_id: "job-1",
                    status: "completed",
                },
                {
                    job_id: "job-2",
                    status: "completed",
                },
            ],
            rescueBudgetExhausted: true,
        });
        expect(result.classification).toBe("infra_failure");
        expect(result.recommended_next_action).toBe("complete_verification_with_infra_failure");
        expect(result.missing_required_prefixes).toEqual(["Completeness: "]);
        expect(result.delegate_assessments[0]?.reason).toContain("terminal state");
    });
});
//# sourceMappingURL=verification-round-assessment.test.js.map