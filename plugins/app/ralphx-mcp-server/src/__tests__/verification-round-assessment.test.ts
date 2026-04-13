import { describe, expect, it } from "vitest";

import {
  aggregateVerificationGaps,
  assessVerificationRound,
} from "../verification-round-assessment.js";

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

  it("keeps the round pending when rescue budget is exhausted but the required delegate is still running", () => {
    const result = assessVerificationRound({
      delegates: [
        { job_id: "job-1", artifact_prefix: "Completeness: ", required: true, label: "completeness" },
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
    expect(result.missing_required_prefixes).toEqual(["Completeness: "]);
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
