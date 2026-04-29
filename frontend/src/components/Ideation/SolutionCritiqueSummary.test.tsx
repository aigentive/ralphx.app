import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { solutionCriticApi } from "@/api/solution-critic";
import { SolutionCritiqueSummary } from "./SolutionCritiqueSummary";

vi.mock("@/api/solution-critic", () => ({
  solutionCriticApi: {
    getLatestCompiledContext: vi.fn(),
    getLatestSolutionCritique: vi.fn(),
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
        openQuestions: [],
        staleAssumptions: [],
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
        claims: [],
        recommendations: [],
        risks: [],
        verificationPlan: [],
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
    expect(screen.getByText("Investigate")).toBeInTheDocument();
    expect(screen.getByText("2 sources")).toBeInTheDocument();
    expect(screen.getByText("1 claim")).toBeInTheDocument();
    expect(screen.getAllByText("2 gaps")).toHaveLength(2);
    expect(screen.getByText("Inspect projected gaps.")).toBeInTheDocument();
    expect(
      screen.getByText("Required verification: prove the migration exists.")
    ).toBeInTheDocument();
  });
});
