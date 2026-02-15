import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { TeammatePaneGrid } from "./TeammatePaneGrid";
import { useSplitPaneStore } from "@/stores/splitPaneStore";
import type { TeammateState } from "@/stores/teamStore";

// Track the mates list so selectTeammateByName can look up by name
let currentMates: TeammateState[] = [];

vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (selector: (s: unknown) => unknown) => {
    return selector({});
  },
  selectTeammates: () => () => currentMates,
  selectTeammateByName: (_ctx: string, name: string) => () =>
    currentMates.find((m) => m.name === name) ?? null,
}));

function makeMate(name: string): TeammateState {
  return {
    name,
    color: "#4ade80",
    model: "sonnet",
    roleDescription: "coder",
    status: "running",
    currentActivity: null,
    tokensUsed: 0,
    estimatedCostUsd: 0,
    streamingText: "",
  };
}

describe("TeammatePaneGrid", () => {
  beforeEach(() => {
    currentMates = [];
    useSplitPaneStore.setState({ focusedPane: null, paneOrder: [], panes: {} });
  });

  it("shows empty state when no teammates", () => {
    render(<TeammatePaneGrid contextKey="test" />);
    expect(screen.getByText("No teammates spawned yet")).toBeInTheDocument();
  });

  it("renders a TeammatePane for each teammate", () => {
    currentMates = [makeMate("worker-1"), makeMate("worker-2"), makeMate("worker-3")];
    render(<TeammatePaneGrid contextKey="test" />);
    expect(screen.getByText("worker-1")).toBeInTheDocument();
    expect(screen.getByText("worker-2")).toBeInTheDocument();
    expect(screen.getByText("worker-3")).toBeInTheDocument();
  });

  it("sets grid-template-rows based on teammate count", () => {
    currentMates = [makeMate("a"), makeMate("b")];
    const { container } = render(<TeammatePaneGrid contextKey="test" />);
    const grid = container.firstChild as HTMLElement;
    expect(grid.style.gridTemplateRows).toBe("repeat(2, 1fr)");
  });
});
