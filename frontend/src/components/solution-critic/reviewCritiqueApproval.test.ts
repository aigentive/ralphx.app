import { describe, expect, it } from "vitest";
import type { SolutionCritiqueReadResponse } from "@/api/solution-critic";
import { buildCritiqueApprovalWarning } from "./reviewCritiqueApproval";

const baseCritique: SolutionCritiqueReadResponse = {
  artifactId: "critique-1",
  solutionCritique: {
    id: "critique-1",
    artifactId: "target-1",
    contextArtifactId: "context-1",
    verdict: "accept",
    confidence: "high",
    claims: [],
    recommendations: [],
    risks: [],
    verificationPlan: [],
    generatedAt: "2026-04-30T12:00:00Z",
  },
  projectedGaps: [],
};

describe("buildCritiqueApprovalWarning", () => {
  it("does not warn without a persisted critique", () => {
    expect(buildCritiqueApprovalWarning(null)).toBeNull();
  });

  it("does not warn for low and medium risk non-reject verdicts", () => {
    const warning = buildCritiqueApprovalWarning({
      ...baseCritique,
      solutionCritique: {
        ...baseCritique.solutionCritique,
        verdict: "revise",
        risks: [
          {
            id: "risk-1",
            risk: "Minor follow-up remains.",
            severity: "medium",
            evidence: [],
          },
        ],
      },
    });

    expect(warning).toBeNull();
  });

  it("warns for high-risk investigate critiques", () => {
    const warning = buildCritiqueApprovalWarning({
      ...baseCritique,
      solutionCritique: {
        ...baseCritique.solutionCritique,
        verdict: "investigate",
        risks: [
          {
            id: "risk-1",
            risk: "Diff coverage is unverified.",
            severity: "high",
            evidence: [],
          },
        ],
        safeNextAction: "Inspect the worker diff.",
      },
    });

    expect(warning).toMatchObject({
      title: "Approve despite solution critique?",
      confirmText: "Approve",
      variant: "destructive",
    });
    expect(warning?.description).toContain("Investigate");
    expect(warning?.description).toContain("high risk: Diff coverage is unverified.");
    expect(warning?.description).toContain("Safe next action: Inspect the worker diff.");
  });

  it("warns for reject critiques even without a high risk", () => {
    const warning = buildCritiqueApprovalWarning({
      ...baseCritique,
      solutionCritique: {
        ...baseCritique.solutionCritique,
        verdict: "reject",
      },
    });

    expect(warning?.description).toContain("Reject");
  });
});
