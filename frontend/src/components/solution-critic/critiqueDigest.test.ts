import { describe, expect, it } from "vitest";
import type {
  CompiledContextReadResponse,
  SolutionCritiqueReadResponse,
} from "@/api/solution-critic";
import {
  buildCritiqueDigest,
  critiqueGapOriginLabel,
  formatCritiqueEnum,
} from "./critiqueDigest";

const context: CompiledContextReadResponse = {
  artifactId: "context-1",
  compiledContext: {
    id: "context-1",
    target: { targetType: "chat_message", id: "message-1", label: "Assistant message" },
    sources: [
      {
        sourceType: "chat_message",
        id: "chat_message:message-1",
        label: "Assistant message",
        createdAt: "2026-04-30T12:05:00Z",
      },
    ],
    claims: [
      {
        id: "claim-1",
        text: "Implementation is complete.",
        classification: "fact",
        confidence: "high",
        evidence: [],
      },
    ],
    openQuestions: [],
    staleAssumptions: [],
    generatedAt: "2026-04-30T12:05:00Z",
  },
};

const critique: SolutionCritiqueReadResponse = {
  artifactId: "critique-1",
  solutionCritique: {
    id: "critique-1",
    artifactId: "message-1",
    contextArtifactId: "context-1",
    verdict: "revise",
    confidence: "medium",
    claims: [
      {
        id: "claim-review-1",
        claim: "Implementation is complete.",
        status: "unsupported",
        confidence: "medium",
        evidence: [],
        notes: "No diff evidence was collected.",
      },
      {
        id: "claim-review-2",
        claim: "The target is scoped.",
        status: "supported",
        confidence: "high",
        evidence: [],
      },
    ],
    recommendations: [],
    risks: [
      {
        id: "risk-1",
        risk: "Unsupported completion claim.",
        severity: "high",
        evidence: [],
        mitigation: "Inspect the diff before approval.",
      },
    ],
    verificationPlan: [],
    safeNextAction: "Inspect the worker diff.",
    generatedAt: "2026-04-30T12:00:00Z",
  },
  projectedGaps: [
    {
      severity: "high",
      category: "solution_critique_risk",
      description: "Risk needs review.",
      whyItMatters: "The claim is unsupported.",
    },
  ],
};

describe("critiqueDigest", () => {
  it("formats critique enums for display", () => {
    expect(formatCritiqueEnum("task_execution")).toBe("Task Execution");
  });

  it("labels critique-derived gap origins from category", () => {
    expect(critiqueGapOriginLabel("solution_critique_claim")).toBe("from critique: claim");
    expect(critiqueGapOriginLabel("solution_critique_risk")).toBe("from critique: risk");
    expect(critiqueGapOriginLabel("other_gap")).toBeNull();
  });

  it("summarizes verdict, counts, primary action, and stale state", () => {
    const digest = buildCritiqueDigest({
      context,
      result: critique,
      isLoading: false,
      error: null,
    });

    expect(digest.state).toBe("stale");
    expect(digest.verdictLabel).toBe("Revise");
    expect(digest.confidenceLabel).toBe("Medium");
    expect(digest.riskCount).toBe(1);
    expect(digest.highestRiskSeverity).toBe("high");
    expect(digest.flaggedClaimCount).toBe(1);
    expect(digest.projectedGapCount).toBe(1);
    expect(digest.primaryAction).toBe("Open critique");
    expect(digest.pillLabel).toBe("Revise - stale");
  });

  it("summarizes missing, loading, and failed critique states", () => {
    expect(buildCritiqueDigest({ context: null, result: null, isLoading: false, error: null }))
      .toMatchObject({ state: "empty", primaryAction: "Run critique", pillLabel: "Critique" });
    expect(buildCritiqueDigest({ context: null, result: null, isLoading: true, error: null }))
      .toMatchObject({ state: "loading", primaryAction: "Critiquing", pillLabel: "Critiquing..." });
    expect(buildCritiqueDigest({ context: null, result: null, isLoading: false, error: "Failed" }))
      .toMatchObject({ state: "error", primaryAction: "Run critique", pillLabel: "Critique failed" });
  });
});
