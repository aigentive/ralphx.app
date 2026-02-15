import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TeamSplitHeader } from "./TeamSplitHeader";
import { useUiStore } from "@/stores/uiStore";
import type { TeammateState } from "@/stores/teamStore";

// Mock teamStore to avoid infinite re-render from factory selectors
const mockTeam = vi.fn();
const mockTeammates = vi.fn();

vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (selector: (s: unknown) => unknown) => {
    // The component calls useTeamStore with memoized selectors from selectActiveTeam / selectTeammates
    // We detect which selector by checking what it returns when called
    const result = selector({});
    // If result is null (selectActiveTeam returns null for empty), return mockTeam
    // We use a simpler approach: return values in call order
    return result;
  },
  selectActiveTeam: () => () => mockTeam(),
  selectTeammates: () => () => mockTeammates(),
}));

function makeMate(name: string, status: TeammateState["status"]): TeammateState {
  return {
    name,
    color: "#4ade80",
    model: "sonnet",
    roleDescription: "coder",
    status,
    currentActivity: null,
    tokensUsed: 0,
    estimatedCostUsd: 0,
    streamingText: "",
  };
}

describe("TeamSplitHeader", () => {
  beforeEach(() => {
    mockTeam.mockReturnValue({
      teamName: "Test Team",
      leadName: "lead",
      totalEstimatedCostUsd: 0,
    });
    mockTeammates.mockReturnValue([
      makeMate("worker-1", "running"),
      makeMate("worker-2", "idle"),
      makeMate("worker-3", "shutdown"),
    ]);
    useUiStore.setState({ previousView: null });
  });

  it("renders team name from store", () => {
    render(<TeamSplitHeader contextKey="test" />);
    expect(screen.getByText("Test Team")).toBeInTheDocument();
  });

  it("shows active count excluding shutdown teammates", () => {
    render(<TeamSplitHeader contextKey="test" />);
    // 2 active (running + idle), 3 total
    expect(screen.getByText("2/3 active")).toBeInTheDocument();
  });

  it("formats cost as $X.XX", () => {
    mockTeam.mockReturnValue({
      teamName: "Test Team",
      leadName: "lead",
      totalEstimatedCostUsd: 1.5,
    });
    render(<TeamSplitHeader contextKey="test" />);
    expect(screen.getByText("$1.50")).toBeInTheDocument();
  });

  it("shows $0.00 when no team exists", () => {
    mockTeam.mockReturnValue(null);
    mockTeammates.mockReturnValue([]);
    render(<TeamSplitHeader contextKey="nonexistent" />);
    expect(screen.getByText("$0.00")).toBeInTheDocument();
  });

  it("navigates back to kanban by default", () => {
    const setCurrentView = vi.fn();
    const setPreviousView = vi.fn();
    useUiStore.setState({ setCurrentView, setPreviousView, previousView: null });

    render(<TeamSplitHeader contextKey="test" />);
    fireEvent.click(screen.getByText("Back"));

    expect(setPreviousView).toHaveBeenCalledWith(null);
    expect(setCurrentView).toHaveBeenCalledWith("kanban");
  });

  it("navigates back to previousView when set", () => {
    const setCurrentView = vi.fn();
    const setPreviousView = vi.fn();
    useUiStore.setState({ setCurrentView, setPreviousView, previousView: "graph" as never });

    render(<TeamSplitHeader contextKey="test" />);
    fireEvent.click(screen.getByText("Back"));

    expect(setCurrentView).toHaveBeenCalledWith("graph");
  });

  it("shows Stop All button when onStopAll provided and active count > 0", () => {
    const onStopAll = vi.fn();
    render(<TeamSplitHeader contextKey="test" onStopAll={onStopAll} />);
    expect(screen.getByText("Stop All")).toBeInTheDocument();
    fireEvent.click(screen.getByText("Stop All"));
    expect(onStopAll).toHaveBeenCalledTimes(1);
  });

  it("hides Stop All button when no onStopAll callback", () => {
    render(<TeamSplitHeader contextKey="test" />);
    expect(screen.queryByText("Stop All")).not.toBeInTheDocument();
  });
});
