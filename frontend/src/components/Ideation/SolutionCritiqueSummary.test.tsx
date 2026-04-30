import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { solutionCriticApi } from "@/api/solution-critic";
import { SolutionCritiqueSummary } from "./SolutionCritiqueSummary";

vi.mock("@/api/solution-critic", () => ({
  solutionCriticApi: {
    getLatestCompiledContext: vi.fn(),
    getLatestSolutionCritique: vi.fn(),
    getSolutionCritiqueRollup: vi.fn(),
    compileTargetContext: vi.fn(),
    critiqueTarget: vi.fn(),
  },
}));

function renderSummary() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return render(
    <QueryClientProvider client={client}>
      <SolutionCritiqueSummary sessionId="session-1" enabled />
    </QueryClientProvider>
  );
}

describe("SolutionCritiqueSummary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(solutionCriticApi.getSolutionCritiqueRollup).mockResolvedValue({
      sessionId: "session-1",
      generatedAt: "2026-04-29T12:00:00Z",
      targetCount: 0,
      critiqueCount: 0,
      staleCount: 0,
      promotedGapCount: 0,
      deferredGapCount: 0,
      coveredGapCount: 0,
      targets: [],
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders nothing when no persisted context or critique exists", async () => {
    vi.mocked(solutionCriticApi.getLatestCompiledContext).mockResolvedValue(null);
    vi.mocked(solutionCriticApi.getLatestSolutionCritique).mockResolvedValue(null);

    renderSummary();

    await waitFor(() => {
      expect(solutionCriticApi.getLatestCompiledContext).toHaveBeenCalledWith("session-1");
      expect(solutionCriticApi.getLatestSolutionCritique).toHaveBeenCalledWith("session-1");
    });
    expect(screen.queryByTestId("solution-critique-summary")).not.toBeInTheDocument();
  });

  it("renders compact context and projected gap data", async () => {
    vi.mocked(solutionCriticApi.getLatestCompiledContext).mockResolvedValue({
      artifactId: "context-1",
      compiledContext: {
        id: "context-1",
        target: { targetType: "plan_artifact", id: "plan-1", label: "Plan" },
        sources: [
          { sourceType: "plan_artifact", id: "plan_artifact:plan-1", label: "Plan" },
          { sourceType: "chat_message", id: "chat_message:1", label: "User note" },
        ],
        claims: [
          {
            id: "claim-1",
            text: "Claim",
            classification: "fact",
            confidence: "high",
            evidence: [],
          },
        ],
        openQuestions: [
          {
            id: "question-1",
            question: "Which migration file owns the change?",
            evidence: [],
          },
        ],
        staleAssumptions: [
          {
            id: "assumption-1",
            text: "The previous schema is still current.",
            evidence: [],
          },
        ],
        generatedAt: "2026-04-29T12:00:00Z",
      },
    });
    vi.mocked(solutionCriticApi.getLatestSolutionCritique).mockResolvedValue({
      artifactId: "critique-1",
      solutionCritique: {
        id: "critique-1",
        artifactId: "plan-1",
        contextArtifactId: "context-1",
        verdict: "investigate",
        confidence: "medium",
        claims: [
          {
            id: "claim-review-1",
            claim: "The migration exists.",
            status: "unsupported",
            confidence: "medium",
            evidence: [],
            notes: "No migration source was collected.",
          },
        ],
        recommendations: [
          {
            id: "recommendation-1",
            recommendation: "Add a migration reference.",
            status: "revise",
            evidence: [],
            rationale: "The plan needs a concrete implementation point.",
          },
        ],
        risks: [
          {
            id: "risk-1",
            risk: "Runtime may miss persisted context.",
            severity: "medium",
            evidence: [],
            mitigation: "Verify startup hydration.",
          },
        ],
        verificationPlan: [
          {
            id: "verification-1",
            requirement: "Run the migration test.",
            priority: "medium",
            evidence: [],
            suggestedTest: "cargo test migration",
          },
        ],
        safeNextAction: "Inspect projected gaps.",
        generatedAt: "2026-04-29T12:30:00Z",
      },
      projectedGaps: [
        {
          severity: "medium",
          category: "solution_critique_claim",
          description: "Unclear plan claim: evidence is partial.",
        },
        {
          severity: "high",
          category: "solution_critique_verification",
          description: "Required verification: prove the migration exists.",
        },
      ],
    });

    renderSummary();

    expect(await screen.findByTestId("solution-critique-summary")).toBeInTheDocument();
    expect(screen.getByText("Plan Solution Critique")).toBeInTheDocument();
    expect(screen.getByText("Target: Plan artifact · plan-1")).toBeInTheDocument();
    expect(screen.getAllByText("Investigate").length).toBeGreaterThan(0);
    expect(screen.getByText("2 sources")).toBeInTheDocument();
    expect(screen.getByText("1 claim")).toBeInTheDocument();
    expect(screen.getAllByText("2 gaps").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("Inspect projected gaps.")).toBeInTheDocument();
    expect(screen.getByText("Which migration file owns the change?")).toBeInTheDocument();
    expect(screen.getByText("The previous schema is still current.")).toBeInTheDocument();
    expect(screen.getByText("The migration exists.")).toBeInTheDocument();
    expect(screen.getByText("Run the migration test.")).toBeInTheDocument();
    expect(
      screen.getByText("Required verification: prove the migration exists.")
    ).toBeInTheDocument();
    expect(screen.getByText("from critique: verification")).toBeInTheDocument();
    expect(screen.queryByText(/solution_critique/i)).not.toBeInTheDocument();
  });

  it("labels compiled context without critique as pending model critique", async () => {
    vi.mocked(solutionCriticApi.getLatestCompiledContext).mockResolvedValue({
      artifactId: "context-1",
      compiledContext: {
        id: "context-1",
        target: { targetType: "plan_artifact", id: "plan-1", label: "Plan" },
        sources: [
          { sourceType: "plan_artifact", id: "plan_artifact:plan-1", label: "Plan" },
        ],
        claims: [
          {
            id: "claim-1",
            text: "The selected target is the plan.",
            classification: "fact",
            confidence: "high",
            evidence: [],
          },
        ],
        openQuestions: [],
        staleAssumptions: [],
        generatedAt: "2026-04-29T12:00:00Z",
      },
    });
    vi.mocked(solutionCriticApi.getLatestSolutionCritique).mockResolvedValue(null);

    renderSummary();

    expect(await screen.findByTestId("solution-critique-summary")).toBeInTheDocument();
    expect(screen.getByText("Plan Critique Pending")).toBeInTheDocument();
    expect(screen.getByText("Target: Plan artifact · plan-1")).toBeInTheDocument();
    expect(screen.getByText("Compiled plan context ready")).toBeInTheDocument();
    expect(
      screen.getByText("Model critique has not been persisted for this context yet.")
    ).toBeInTheDocument();
    expect(screen.getByText("No LLM critique persisted yet.")).toBeInTheDocument();
    expect(screen.queryByText("No critique")).not.toBeInTheDocument();
  });

  it("polls for the model critique while compiled context is waiting", async () => {
    vi.mocked(solutionCriticApi.getLatestCompiledContext).mockResolvedValue({
      artifactId: "context-1",
      compiledContext: {
        id: "context-1",
        target: { targetType: "plan_artifact", id: "plan-1", label: "Plan" },
        sources: [
          { sourceType: "plan_artifact", id: "plan_artifact:plan-1", label: "Plan" },
        ],
        claims: [
          {
            id: "claim-1",
            text: "The selected target is the plan.",
            classification: "fact",
            confidence: "high",
            evidence: [],
          },
        ],
        openQuestions: [],
        staleAssumptions: [],
        generatedAt: "2026-04-29T12:00:00Z",
      },
    });
    vi.mocked(solutionCriticApi.getLatestSolutionCritique)
      .mockResolvedValueOnce(null)
      .mockResolvedValue({
        artifactId: "critique-1",
        solutionCritique: {
          id: "critique-1",
          artifactId: "plan-1",
          contextArtifactId: "context-1",
          verdict: "investigate",
          confidence: "high",
          claims: [],
          recommendations: [],
          risks: [],
          verificationPlan: [],
          safeNextAction: "Review the critique.",
          generatedAt: "2026-04-29T12:30:00Z",
        },
        projectedGaps: [],
      });

    renderSummary();

    expect(await screen.findByText("Compiled plan context ready")).toBeInTheDocument();
    expect((await screen.findAllByText("Investigate", {}, { timeout: 2_500 })).length).toBeGreaterThan(0);
    expect(screen.getByText("Review the critique.")).toBeInTheDocument();
    expect(solutionCriticApi.getLatestSolutionCritique).toHaveBeenCalledTimes(2);
  });

  it("distinguishes pending plan critique from existing message critiques and can run the plan critique", async () => {
    vi.mocked(solutionCriticApi.getLatestCompiledContext).mockResolvedValue({
      artifactId: "context-1",
      compiledContext: {
        id: "context-1",
        target: { targetType: "plan_artifact", id: "plan-1", label: "Plan" },
        sources: [
          { sourceType: "plan_artifact", id: "plan_artifact:plan-1", label: "Plan" },
        ],
        claims: [
          {
            id: "claim-1",
            text: "The selected target is the plan.",
            classification: "fact",
            confidence: "high",
            evidence: [],
          },
        ],
        openQuestions: [],
        staleAssumptions: [],
        generatedAt: "2026-04-29T12:00:00Z",
      },
    });
    vi.mocked(solutionCriticApi.getLatestSolutionCritique).mockResolvedValue(null);
    vi.mocked(solutionCriticApi.getSolutionCritiqueRollup).mockResolvedValue({
      sessionId: "session-1",
      generatedAt: "2026-04-29T12:30:00Z",
      targetCount: 1,
      critiqueCount: 1,
      staleCount: 0,
      promotedGapCount: 2,
      deferredGapCount: 0,
      coveredGapCount: 0,
      targets: [
        {
          target: {
            targetType: "chat_message",
            id: "chat_message:fb59b1e9-aafe-4797-97d3-28b2e79d718f",
            label: "Assistant message",
          },
          artifactId: "message-critique-1",
          contextArtifactId: "message-context-1",
          verdict: "revise",
          confidence: "medium",
          generatedAt: "2026-04-29T12:25:00Z",
          stale: false,
          riskCount: 4,
          projectedGapCount: 5,
          promotedGapCount: 2,
          deferredGapCount: 0,
          coveredGapCount: 0,
        },
      ],
    });
    vi.mocked(solutionCriticApi.compileTargetContext).mockResolvedValue({
      artifactId: "context-2",
      compiledContext: {
        id: "context-2",
        target: { targetType: "plan_artifact", id: "plan-1", label: "Plan" },
        sources: [],
        claims: [],
        openQuestions: [],
        staleAssumptions: [],
        generatedAt: "2026-04-29T12:40:00Z",
      },
    });
    vi.mocked(solutionCriticApi.critiqueTarget).mockResolvedValue({
      artifactId: "plan-critique-1",
      solutionCritique: {
        id: "plan-critique-1",
        artifactId: "plan-1",
        contextArtifactId: "context-2",
        verdict: "revise",
        confidence: "medium",
        claims: [],
        recommendations: [],
        risks: [],
        verificationPlan: [],
        safeNextAction: "Revise the plan.",
        generatedAt: "2026-04-29T12:41:00Z",
      },
      projectedGaps: [],
    });

    renderSummary();

    expect(await screen.findByText("Plan critique not yet run")).toBeInTheDocument();
    expect(
      screen.getByText("Latest saved critique is for Assistant response · 28b2e79d718f.")
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Critique plan" }));

    await waitFor(() => {
      expect(solutionCriticApi.compileTargetContext).toHaveBeenCalledWith(
        "session-1",
        { targetType: "plan_artifact", id: "plan-1", label: "Plan" },
      );
      expect(solutionCriticApi.critiqueTarget).toHaveBeenCalledWith(
        "session-1",
        { targetType: "plan_artifact", id: "plan-1", label: "Plan" },
        "context-2",
      );
    });
    expect(await screen.findByText("Revise the plan.")).toBeInTheDocument();
  });
});
