import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  CompiledContextReadResponseSchema,
  SolutionCritiqueReadResponseSchema,
  solutionCriticApi,
} from "./solution-critic";

const mockFetch = vi.fn();

beforeEach(() => {
  vi.stubGlobal("fetch", mockFetch);
  mockFetch.mockReset();
});

const sourceRef = {
  source_type: "plan_artifact",
  id: "plan_artifact:plan-1",
  label: "Implementation Plan",
  excerpt: "Plan excerpt",
  created_at: "2026-04-29T12:00:00Z",
};

const compiledContextRaw = {
  artifact_id: "context-1",
  compiled_context: {
    id: "context-1",
    target: {
      target_type: "plan_artifact",
      id: "plan-1",
      label: "Implementation Plan",
    },
    sources: [sourceRef],
    claims: [
      {
        id: "claim-1",
        text: "The plan exists.",
        classification: "fact",
        confidence: "high",
        evidence: [sourceRef],
      },
    ],
    open_questions: [],
    stale_assumptions: [],
    generated_at: "2026-04-29T12:00:00Z",
  },
};

const solutionCritiqueRaw = {
  artifact_id: "critique-1",
  solution_critique: {
    id: "critique-1",
    artifact_id: "plan-1",
    context_artifact_id: "context-1",
    verdict: "investigate",
    confidence: "medium",
    claims: [
      {
        id: "claim-review-1",
        claim: "The plan needs stronger evidence.",
        status: "unclear",
        confidence: "medium",
        evidence: [sourceRef],
        notes: "Evidence is partial.",
      },
    ],
    recommendations: [],
    risks: [
      {
        id: "risk-1",
        risk: "Unsupported plan claims can mislead implementation.",
        severity: "medium",
        evidence: [sourceRef],
        mitigation: "Verify before implementation.",
      },
    ],
    verification_plan: [
      {
        id: "requirement-1",
        requirement: "Check each major claim against source evidence.",
        priority: "high",
        evidence: [sourceRef],
        suggested_test: "Run focused backend coverage.",
      },
    ],
    safe_next_action: "Inspect projected gaps.",
    generated_at: "2026-04-29T12:30:00Z",
  },
  projected_gaps: [
    {
      severity: "high",
      category: "solution_critique_verification",
      description: "Required verification: Check each major claim against source evidence.",
      why_it_matters: "Suggested test: Run focused backend coverage.",
    },
  ],
};

describe("solution critic API schemas", () => {
  it("parses compiled context responses into frontend shape", () => {
    const parsed = CompiledContextReadResponseSchema.parse(compiledContextRaw);

    expect(parsed.artifactId).toBe("context-1");
    expect(parsed.compiledContext.target.targetType).toBe("plan_artifact");
    expect(parsed.compiledContext.sources[0].sourceType).toBe("plan_artifact");
    expect(parsed.compiledContext.openQuestions).toEqual([]);
  });

  it("parses solution critique responses with projected gaps", () => {
    const parsed = SolutionCritiqueReadResponseSchema.parse(solutionCritiqueRaw);

    expect(parsed.artifactId).toBe("critique-1");
    expect(parsed.solutionCritique.artifactId).toBe("plan-1");
    expect(parsed.solutionCritique.verificationPlan[0].suggestedTest).toBe(
      "Run focused backend coverage."
    );
    expect(parsed.projectedGaps).toEqual([
      {
        severity: "high",
        category: "solution_critique_verification",
        description: "Required verification: Check each major claim against source evidence.",
        whyItMatters: "Suggested test: Run focused backend coverage.",
      },
    ]);
  });
});

describe("solutionCriticApi", () => {
  it("fetches the latest compiled context as nullable data", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(compiledContextRaw),
    });

    const result = await solutionCriticApi.getLatestCompiledContext("session 1");

    expect(mockFetch).toHaveBeenCalledWith(
      "http://localhost:3847/api/ideation/sessions/session%201/compiled-context",
      {}
    );
    expect(result?.artifactId).toBe("context-1");
  });

  it("returns null when no latest solution critique exists", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(null),
    });

    await expect(solutionCriticApi.getLatestSolutionCritique("session-1")).resolves.toBeNull();
  });

  it("posts compile context requests with snake_case source limits", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(compiledContextRaw),
    });

    await solutionCriticApi.compileContext("session-1", "plan-1", {
      chatMessages: 5,
      taskProposals: 3,
      relatedArtifacts: 2,
      agentRuns: 1,
    });

    expect(mockFetch).toHaveBeenCalledWith(
      "http://localhost:3847/api/ideation/sessions/session-1/compiled-context",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          target_artifact_id: "plan-1",
          source_limits: {
            chat_messages: 5,
            task_proposals: 3,
            related_artifacts: 2,
            agent_runs: 1,
          },
        }),
      }
    );
  });

  it("throws backend error messages when requests fail", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 400,
      json: () => Promise.resolve({ error: "Compiled context target mismatch" }),
    });

    await expect(
      solutionCriticApi.getSolutionCritique("session-1", "critique-1")
    ).rejects.toThrow("Compiled context target mismatch");
  });
});
