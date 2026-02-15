import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { TeamOverviewHeader } from "./TeamOverviewHeader";
import type { TeammateState } from "@/stores/teamStore";

// Mock teamStore to avoid infinite re-render from factory selectors
const mockTeam = vi.fn();
const mockTeammates = vi.fn();

vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (selector: (s: unknown) => unknown) => {
    return selector({});
  },
  selectActiveTeam: () => () => mockTeam(),
  selectTeammates: () => () => mockTeammates(),
}));

function makeMate(name: string, status: TeammateState["status"], color = "#4ade80"): TeammateState {
  return {
    name,
    color,
    model: "sonnet",
    roleDescription: "coder",
    status,
    currentActivity: null,
    tokensUsed: 0,
    estimatedCostUsd: 0,
    streamingText: "",
  };
}

describe("TeamOverviewHeader", () => {
  beforeEach(() => {
    mockTeam.mockReturnValue(null);
    mockTeammates.mockReturnValue([]);
  });

  it("renders nothing when team does not exist", () => {
    const { container } = render(<TeamOverviewHeader contextKey="nonexistent" />);
    expect(container.firstChild).toBeNull();
  });

  it("shows active count excluding shutdown and completed", () => {
    mockTeam.mockReturnValue({ teamName: "T", totalEstimatedCostUsd: 0 });
    mockTeammates.mockReturnValue([
      makeMate("a", "running"),
      makeMate("b", "idle"),
      makeMate("c", "shutdown"),
      makeMate("d", "completed"),
    ]);
    render(<TeamOverviewHeader contextKey="test" />);
    expect(screen.getByText("2 active")).toBeInTheDocument();
  });

  it("shows teammate count as tasks label", () => {
    mockTeam.mockReturnValue({ teamName: "T", totalEstimatedCostUsd: 0 });
    mockTeammates.mockReturnValue([
      makeMate("a", "running"),
      makeMate("b", "idle"),
    ]);
    render(<TeamOverviewHeader contextKey="test" />);
    expect(screen.getByText("2 tasks")).toBeInTheDocument();
  });

  it("formats cost as <$0.01 when below threshold", () => {
    mockTeam.mockReturnValue({ teamName: "T", totalEstimatedCostUsd: 0 });
    mockTeammates.mockReturnValue([makeMate("a", "running")]);
    render(<TeamOverviewHeader contextKey="test" />);
    expect(screen.getByText("<$0.01")).toBeInTheDocument();
  });

  it("renders avatar dots for each teammate", () => {
    mockTeam.mockReturnValue({ teamName: "T", totalEstimatedCostUsd: 0 });
    mockTeammates.mockReturnValue([
      makeMate("worker-1", "running", "#ff0000"),
      makeMate("worker-2", "shutdown", "#00ff00"),
    ]);
    render(<TeamOverviewHeader contextKey="test" />);
    const dots = screen.getAllByTitle(/worker-/);
    expect(dots).toHaveLength(2);
    expect(dots[0]).toHaveAttribute("title", "worker-1 (running)");
    expect(dots[1]).toHaveAttribute("title", "worker-2 (shutdown)");
  });
});
