import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { TeamSplitView } from "./TeamSplitView";
import { useSplitPaneStore } from "@/stores/splitPaneStore";

describe("TeamSplitView", () => {
  beforeEach(() => {
    useSplitPaneStore.setState({
      contextKey: null,
      isActive: false,
      focusedPane: null,
      coordinatorWidth: 40,
      isPrefixKeyActive: false,
      paneOrder: [],
      panes: {},
    });
  });

  it("renders empty state when no contextKey prop or store key", () => {
    render(<TeamSplitView />);
    expect(screen.getByText("No active team")).toBeInTheDocument();
  });

  it("syncs contextKey prop to splitPaneStore", () => {
    render(<TeamSplitView contextKey="team:abc" />);
    expect(useSplitPaneStore.getState().contextKey).toBe("team:abc");
  });

  it("uses store contextKey when prop is not provided", () => {
    useSplitPaneStore.setState({ contextKey: "team:store-key" });
    render(<TeamSplitView />);
    // Should not show empty state since store key exists
    expect(screen.queryByText("No active team")).not.toBeInTheDocument();
  });

  it("prefers prop contextKey over store contextKey", () => {
    useSplitPaneStore.setState({ contextKey: "team:store-key" });
    render(<TeamSplitView contextKey="team:prop-key" />);
    expect(useSplitPaneStore.getState().contextKey).toBe("team:prop-key");
  });
});
