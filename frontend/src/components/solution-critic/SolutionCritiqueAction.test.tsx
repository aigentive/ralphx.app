import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { solutionCriticApi } from "@/api/solution-critic";
import { SolutionCritiqueAction } from "./SolutionCritiqueAction";

vi.mock("@/api/solution-critic", () => ({
  solutionCriticApi: {
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
    recommendations: [],
    risks: [],
    verificationPlan: [],
    safeNextAction: "Inspect the worker diff.",
    generatedAt: "2026-04-30T12:00:10Z",
  },
  projectedGaps: [],
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
    expect(screen.getByText("Unsupported")).toBeInTheDocument();
    expect(screen.getByText("Inspect the worker diff.")).toBeInTheDocument();
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
    expect(await screen.findByText("Inspect the worker diff.")).toBeInTheDocument();
    expect(solutionCriticApi.compileTargetContext).toHaveBeenCalledTimes(1);
    expect(solutionCriticApi.critiqueTarget).toHaveBeenCalledTimes(1);

    await userEvent.keyboard("{Escape}");
    await waitFor(() => {
      expect(screen.queryByText("Inspect the worker diff.")).not.toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole("button", { name: "Critique this" }));
    expect(await screen.findByText("Inspect the worker diff.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Run again" })).toBeInTheDocument();
    expect(solutionCriticApi.compileTargetContext).toHaveBeenCalledTimes(1);
    expect(solutionCriticApi.critiqueTarget).toHaveBeenCalledTimes(1);

    await userEvent.click(screen.getByRole("button", { name: "Run again" }));
    await waitFor(() => {
      expect(solutionCriticApi.compileTargetContext).toHaveBeenCalledTimes(2);
      expect(solutionCriticApi.critiqueTarget).toHaveBeenCalledTimes(2);
    });
  });
});
