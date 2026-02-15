/**
 * TeamCostDisplay tests
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TeamCostDisplay } from "./TeamCostDisplay";
import type { TeammateState } from "@/stores/teamStore";

function makeTeammate(overrides: Partial<TeammateState> = {}): TeammateState {
  return {
    name: "coder-1",
    color: "#3b82f6",
    model: "sonnet",
    roleDescription: "Auth",
    status: "running",
    currentActivity: null,
    tokensUsed: 50000,
    estimatedCostUsd: 0.30,
    streamingText: "",
    ...overrides,
  };
}

describe("TeamCostDisplay", () => {
  it("renders aggregate totals", () => {
    render(
      <TeamCostDisplay
        totalTokens={130000}
        totalEstimatedCostUsd={0.78}
        teammates={[]}
      />
    );
    expect(screen.getByText(/~130K tokens/)).toBeInTheDocument();
    expect(screen.getByText(/\$0\.78/)).toBeInTheDocument();
  });

  it("renders per-teammate breakdown", () => {
    const teammates = [
      makeTeammate({ name: "coder-1", tokensUsed: 50000, estimatedCostUsd: 0.30 }),
      makeTeammate({ name: "coder-2", tokensUsed: 80000, estimatedCostUsd: 0.48, color: "#10b981" }),
    ];
    render(
      <TeamCostDisplay
        totalTokens={130000}
        totalEstimatedCostUsd={0.78}
        teammates={teammates}
      />
    );
    expect(screen.getByText("coder-1")).toBeInTheDocument();
    expect(screen.getByText("coder-2")).toBeInTheDocument();
  });

  it("shows <$0.01 for very small costs", () => {
    render(
      <TeamCostDisplay
        totalTokens={100}
        totalEstimatedCostUsd={0.001}
        teammates={[makeTeammate({ tokensUsed: 100, estimatedCostUsd: 0.001 })]}
      />
    );
    // Both aggregate and per-teammate should show <$0.01
    const cheapLabels = screen.getAllByText(/<\$0\.01/);
    expect(cheapLabels.length).toBeGreaterThanOrEqual(1);
  });
});
