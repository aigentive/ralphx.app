import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { solutionCriticApi } from "@/api/solution-critic";
import { SolutionCritiqueRecord } from "./SolutionCritiqueRecord";

vi.mock("@/api/solution-critic", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/api/solution-critic")>();
  return {
    ...actual,
    solutionCriticApi: {
      getLatestTargetCompiledContext: vi.fn(),
      getLatestTargetSolutionCritique: vi.fn(),
    },
  };
});

const critiqueResponse = {
  artifactId: "critique-1",
  solutionCritique: {
    id: "critique-1",
    artifactId: "task-1",
    contextArtifactId: "context-1",
    verdict: "revise",
    confidence: "medium",
    claims: [],
    recommendations: [],
    risks: [
      {
        id: "risk-1",
        risk: "The accepted result still needs a manual diff spot check.",
        severity: "medium",
        evidence: [],
      },
    ],
    verificationPlan: [],
    safeNextAction: "Review the accepted diff.",
    generatedAt: "2026-04-30T12:00:00Z",
  },
  projectedGaps: [],
};

function renderRecord() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return render(
    <QueryClientProvider client={client}>
      <SolutionCritiqueRecord
        sessionId="session-1"
        target={{
          targetType: "task_execution",
          id: "task-1",
          label: "Task execution: Build feature",
        }}
      />
    </QueryClientProvider>,
  );
}

describe("SolutionCritiqueRecord", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(solutionCriticApi.getLatestTargetCompiledContext).mockResolvedValue({
      artifactId: "context-1",
      compiledContext: {
        id: "context-1",
        target: { targetType: "task_execution", id: "task-1", label: "Task execution" },
        sources: [],
        claims: [],
        openQuestions: [],
        staleAssumptions: [],
        generatedAt: "2026-04-30T11:59:00Z",
      },
    });
    vi.mocked(solutionCriticApi.getLatestTargetSolutionCritique).mockResolvedValue(critiqueResponse);
  });

  it("opens a saved critique record without offering refresh model work", async () => {
    renderRecord();

    expect(await screen.findByTestId("solution-critique-record")).toHaveTextContent(
      "Revise - Medium confidence - 1 risk"
    );

    await userEvent.click(screen.getByRole("button", { name: "Open critique" }));

    expect(await screen.findByText("Review the accepted diff.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Refresh critique" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Run critique" })).not.toBeInTheDocument();
  });
});
