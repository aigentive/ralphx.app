import { describe, expect, it } from "vitest";
import { aggregateVerificationGaps, assessVerificationRound, } from "../verification-round-assessment.js";
describe("assessVerificationRound", () => {
    it("classifies the round as complete when all required findings are present", () => {
        const result = assessVerificationRound({
            delegates: [
                { job_id: "job-1", critic: "completeness", required: true },
                { job_id: "job-2", critic: "feasibility", required: true },
            ],
            findingsByCritic: [
                { critic: "completeness", found: true, total_matches: 1, finding: { artifact_id: "a1", title: "Completeness: Round 1", created_at: "2026-04-13T17:25:41Z", critic: "completeness", round: 1, status: "complete", summary: "done", gaps: [] } },
                { critic: "feasibility", found: true, total_matches: 1, finding: { artifact_id: "a2", title: "Feasibility: Round 1", created_at: "2026-04-13T17:25:28Z", critic: "feasibility", round: 1, status: "complete", summary: "done", gaps: [] } },
            ],
            delegateSnapshots: [
                { job_id: "job-1", status: "completed" },
                { job_id: "job-2", status: "completed" },
            ],
        });
        expect(result.classification).toBe("complete");
        expect(result.recommended_next_action).toBe("continue_round_analysis");
        expect(result.missing_required_critics).toEqual([]);
    });
    it("classifies the round as pending before rescue budget is exhausted when a required delegate is still running", () => {
        const result = assessVerificationRound({
            delegates: [
                { job_id: "job-1", critic: "completeness", required: true },
            ],
            findingsByCritic: [
                { critic: "completeness", found: false, total_matches: 0 },
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
        expect(result.missing_required_critics).toEqual(["completeness"]);
    });
    it("classifies the round as infra failure when rescue budget is exhausted and a required finding is still missing", () => {
        const result = assessVerificationRound({
            delegates: [
                { job_id: "job-1", critic: "completeness", required: true, label: "completeness" },
                { job_id: "job-2", critic: "feasibility", required: true, label: "feasibility" },
            ],
            findingsByCritic: [
                { critic: "completeness", found: false, total_matches: 0 },
                { critic: "feasibility", found: true, total_matches: 1, finding: { artifact_id: "a2", title: "Feasibility: Round 1", created_at: "2026-04-13T17:25:28Z", critic: "feasibility", round: 1, status: "complete", summary: "done", gaps: [] } },
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
        expect(result.missing_required_critics).toEqual(["completeness"]);
        expect(result.delegate_assessments[0]?.reason).toContain("terminal state");
    });
    it("keeps the round pending when rescue budget is exhausted but the required delegate is still running", () => {
        const result = assessVerificationRound({
            delegates: [
                { job_id: "job-1", critic: "completeness", required: true, label: "completeness" },
            ],
            findingsByCritic: [
                { critic: "completeness", found: false, total_matches: 0 },
            ],
            delegateSnapshots: [
                {
                    job_id: "job-1",
                    status: "running",
                    delegated_status: {
                        agent_state: {
                            estimated_status: "likely_generating",
                        },
                        latest_run: {
                            status: "running",
                        },
                    },
                },
            ],
            rescueBudgetExhausted: true,
        });
        expect(result.classification).toBe("pending");
        expect(result.recommended_next_action).toBe("perform_single_rescue_or_wait");
        expect(result.missing_required_critics).toEqual(["completeness"]);
    });
});
describe("aggregateVerificationGaps", () => {
    it("merges duplicate required-critic gaps into one backend-owned blocker list", () => {
        const result = aggregateVerificationGaps([
            {
                prefix: "Completeness: ",
                label: "completeness",
                usable: true,
                gaps: [
                    {
                        severity: "critical",
                        category: "migration",
                        description: "Existing rows are not backfilled.",
                        why_it_matters: "Already-persisted projects will keep the old default.",
                        source: "layer1",
                    },
                ],
            },
            {
                prefix: "Feasibility: ",
                label: "feasibility",
                usable: true,
                gaps: [
                    {
                        severity: "critical",
                        category: "migration",
                        description: "Existing rows are not backfilled.",
                        why_it_matters: "Already-persisted projects will keep the old default.",
                        source: "layer2",
                    },
                    {
                        severity: "high",
                        category: "tests",
                        description: "Fixture coverage does not assert the new default.",
                        source: "layer2",
                    },
                ],
            },
        ]);
        expect(result.merged_gaps).toHaveLength(2);
        expect(result.gap_counts).toEqual({
            critical: 1,
            high: 1,
            medium: 0,
            low: 0,
        });
        expect(result.merged_gaps[0]).toMatchObject({
            severity: "critical",
            category: "migration",
            source: "both",
        });
    });
});
//# sourceMappingURL=verification-round-assessment.test.js.map