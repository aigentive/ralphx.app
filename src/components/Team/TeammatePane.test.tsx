import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TeammatePane } from "./TeammatePane";
import { useSplitPaneStore } from "@/stores/splitPaneStore";
import type { TeammateState } from "@/stores/teamStore";

// Mock teamStore to avoid infinite re-render from factory selectors
const mockMate = vi.fn();

vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (selector: (s: unknown) => unknown) => {
    return selector({});
  },
  selectTeammateByName: () => () => mockMate(),
}));

function makeMate(overrides?: Partial<TeammateState>): TeammateState {
  return {
    name: "worker-1",
    color: "#4ade80",
    model: "sonnet",
    roleDescription: "coder",
    status: "running",
    currentActivity: null,
    tokensUsed: 0,
    estimatedCostUsd: 0,
    conversationId: null,
    ...overrides,
  };
}

describe("TeammatePane", () => {
  beforeEach(() => {
    mockMate.mockReturnValue(null);
    useSplitPaneStore.setState({
      focusedPane: null,
      paneOrder: [],
      panes: {},
    });
  });

  it("renders nothing when teammate does not exist", () => {
    const { container } = render(
      <TeammatePane contextKey="test" teammateName="nonexistent" />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders teammate name via PaneHeader", () => {
    mockMate.mockReturnValue(makeMate());
    render(<TeammatePane contextKey="test" teammateName="worker-1" />);
    expect(screen.getByText("worker-1")).toBeInTheDocument();
  });

  it("sets focused pane on click", () => {
    mockMate.mockReturnValue(makeMate());
    const { container } = render(
      <TeammatePane contextKey="test" teammateName="worker-1" />,
    );
    // The outer div has role="button", click it
    const paneEl = container.firstChild as HTMLElement;
    fireEvent.click(paneEl);
    expect(useSplitPaneStore.getState().focusedPane).toBe("worker-1");
  });

  it("shows orange border when focused", () => {
    mockMate.mockReturnValue(makeMate());
    useSplitPaneStore.setState({ focusedPane: "worker-1" });
    const { container } = render(
      <TeammatePane contextKey="test" teammateName="worker-1" />,
    );
    const pane = container.firstChild as HTMLElement;
    // jsdom converts HSL to RGB: hsl(14 100% 60%) → rgb(255, 99, 51)
    expect(pane.style.border).toContain("rgb(255, 99, 51)");
  });

  it("shows dim border when not focused", () => {
    mockMate.mockReturnValue(makeMate());
    useSplitPaneStore.setState({ focusedPane: "other" });
    const { container } = render(
      <TeammatePane contextKey="test" teammateName="worker-1" />,
    );
    const pane = container.firstChild as HTMLElement;
    // jsdom converts HSL to RGB: hsl(220 10% 14%) → rgb(32, 35, 39)
    expect(pane.style.border).toContain("rgb(32, 35, 39)");
  });

  it("calls onStop with teammate name", () => {
    mockMate.mockReturnValue(makeMate());
    const onStop = vi.fn();
    render(
      <TeammatePane contextKey="test" teammateName="worker-1" onStop={onStop} />,
    );
    fireEvent.click(screen.getByLabelText("Stop worker-1"));
    expect(onStop).toHaveBeenCalledWith("worker-1");
  });
});
