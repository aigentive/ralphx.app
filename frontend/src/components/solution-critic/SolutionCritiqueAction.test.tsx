import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { solutionCriticApi } from "@/api/solution-critic";
import { SolutionCritiqueAction } from "./SolutionCritiqueAction";

vi.mock("@/api/solution-critic", () => ({
  solutionCriticApi: {
    getLatestTargetCompiledContext: vi.fn(),
    getLatestTargetSolutionCritique: vi.fn(),
    compileTargetContext: vi.fn(),
    critiqueTarget: vi.fn(),
  },
}));

const critiqueResponse = {
  artifactId: "critique-1",
  solutionCritique: {
    id: "critique-1",
    artifactId: "message-1",
    contextArtifactId: "context-1",
    verdict: "investigate",
    confidence: "medium",
    claims: [
      {
        id: "claim-1",
        claim: "The assistant claims implementation is complete.",
        status: "unsupported",
        confidence: "medium",
        evidence: [],
        notes: "No diff source backs the claim.",
      },
    ],
    recommendations: [
      {
        id: "recommendation-1",
        recommendation: "Treat the completion claim as unproven.",
        status: "revise",
        evidence: [],
        rationale: "The critique did not collect a diff source.",
      },
    ],
    risks: [
      {
        id: "risk-1",
        risk: "Unsupported completion claims can lead to approving broken work.",
        severity: "high",
        evidence: [],
        mitigation: "Inspect the worker diff.",
      },
    ],
    verificationPlan: [
      {
        id: "verify-1",
        requirement: "Check the worker diff against the stated acceptance criteria.",
        priority: "high",
        evidence: [],
        suggestedTest: "Review changed files manually.",
      },
    ],
    safeNextAction: "Inspect the worker diff.",
    generatedAt: "2026-04-30T12:00:10Z",
  },
  projectedGaps: [
    {
      severity: "high",
      category: "solution_critique_risk",
      description: "Unsupported completion claim.",
      whyItMatters: "Approval could merge broken work.",
    },
  ],
};

function renderAction() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return render(
    <QueryClientProvider client={client}>
      <SolutionCritiqueAction
        sessionId="session-1"
        target={{
          targetType: "chat_message",
          id: "message-1",
          label: "Assistant message",
        }}
      />
    </QueryClientProvider>,
  );
}

describe("SolutionCritiqueAction", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(solutionCriticApi.getLatestTargetCompiledContext).mockResolvedValue(null);
    vi.mocked(solutionCriticApi.getLatestTargetSolutionCritique).mockResolvedValue(null);
  });

  it("runs compile then critique for the selected target and shows the result", async () => {
    vi.mocked(solutionCriticApi.compileTargetContext).mockResolvedValue({
      artifactId: "context-1",
      compiledContext: {
        id: "context-1",
        target: { targetType: "chat_message", id: "message-1", label: "Assistant message" },
        sources: [],
        claims: [],
        openQuestions: [],
        staleAssumptions: [],
        generatedAt: "2026-04-30T12:00:00Z",
      },
    });
    vi.mocked(solutionCriticApi.critiqueTarget).mockResolvedValue(critiqueResponse);

    renderAction();
    await userEvent.click(screen.getByRole("button", { name: "Critique this" }));

    await waitFor(() => {
      expect(solutionCriticApi.compileTargetContext).toHaveBeenCalledWith(
        "session-1",
        { targetType: "chat_message", id: "message-1", label: "Assistant message" },
      );
      expect(solutionCriticApi.critiqueTarget).toHaveBeenCalledWith(
        "session-1",
        { targetType: "chat_message", id: "message-1", label: "Assistant message" },
        "context-1",
      );
    });

    expect(await screen.findByText("Investigate")).toBeInTheDocument();
    expect(screen.getAllByText(/Unsupported/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("Inspect the worker diff.").length).toBeGreaterThan(0);
    expect(screen.getByText("Unsupported completion claims can lead to approving broken work.")).toBeInTheDocument();
    expect(screen.getByText("from critique: risk")).toBeInTheDocument();
  });

  it("reopens the cached critique and reruns only from the explicit action", async () => {
    vi.mocked(solutionCriticApi.compileTargetContext).mockResolvedValue({
      artifactId: "context-1",
      compiledContext: {
        id: "context-1",
        target: { targetType: "chat_message", id: "message-1", label: "Assistant message" },
        sources: [],
        claims: [],
        openQuestions: [],
        staleAssumptions: [],
        generatedAt: "2026-04-30T12:00:00Z",
      },
    });
    vi.mocked(solutionCriticApi.critiqueTarget).mockResolvedValue(critiqueResponse);

    renderAction();
    await userEvent.click(screen.getByRole("button", { name: "Critique this" }));
    expect((await screen.findAllByText("Inspect the worker diff.")).length).toBeGreaterThan(0);
    expect(solutionCriticApi.compileTargetContext).toHaveBeenCalledTimes(1);
    expect(solutionCriticApi.critiqueTarget).toHaveBeenCalledTimes(1);

    await userEvent.keyboard("{Escape}");
    await waitFor(() => {
      expect(screen.queryAllByText("Inspect the worker diff.")).toHaveLength(0);
    });

    await userEvent.click(screen.getByTestId("solution-critique-action"));
    expect((await screen.findAllByText("Inspect the worker diff.")).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Refresh critique" })).toBeInTheDocument();
    expect(solutionCriticApi.compileTargetContext).toHaveBeenCalledTimes(1);
    expect(solutionCriticApi.critiqueTarget).toHaveBeenCalledTimes(1);

    await userEvent.click(screen.getByRole("button", { name: "Refresh critique" }));
    await waitFor(() => {
      expect(solutionCriticApi.compileTargetContext).toHaveBeenCalledTimes(2);
      expect(solutionCriticApi.critiqueTarget).toHaveBeenCalledTimes(2);
    });
  });

  it("opens a persisted target critique without starting a new model run", async () => {
    vi.mocked(solutionCriticApi.getLatestTargetCompiledContext).mockResolvedValue({
      artifactId: "context-1",
      compiledContext: {
        id: "context-1",
        target: { targetType: "chat_message", id: "message-1", label: "Assistant message" },
        sources: [],
        claims: [],
        openQuestions: [],
        staleAssumptions: [],
        generatedAt: "2026-04-30T12:00:00Z",
      },
    });
    vi.mocked(solutionCriticApi.getLatestTargetSolutionCritique).mockResolvedValue(critiqueResponse);

    renderAction();

    expect(await screen.findByRole("button", { name: "Open critique: Investigate" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Open critique: Investigate" }));

    expect(await screen.findByText("Unsupported completion claim.")).toBeInTheDocument();
    expect(solutionCriticApi.compileTargetContext).not.toHaveBeenCalled();
    expect(solutionCriticApi.critiqueTarget).not.toHaveBeenCalled();
  });
});
