import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { TeamSplitGrid } from "./TeamSplitGrid";
import { useSplitPaneStore } from "@/stores/splitPaneStore";

describe("TeamSplitGrid", () => {
  beforeEach(() => {
    useSplitPaneStore.setState({ coordinatorWidth: 40 });
  });

  it("renders default placeholder text when no slots provided", () => {
    render(<TeamSplitGrid />);
    expect(screen.getByText("Coordinator")).toBeInTheDocument();
    expect(screen.getByText("Teammates")).toBeInTheDocument();
  });

  it("renders custom coordinator and teammates slots", () => {
    render(
      <TeamSplitGrid
        coordinatorSlot={<div>Custom Coordinator</div>}
        teammatesSlot={<div>Custom Teammates</div>}
      />,
    );
    expect(screen.getByText("Custom Coordinator")).toBeInTheDocument();
    expect(screen.getByText("Custom Teammates")).toBeInTheDocument();
  });

  it("applies coordinatorWidth from store as grid-template-columns", () => {
    useSplitPaneStore.setState({ coordinatorWidth: 60 });
    const { container } = render(<TeamSplitGrid />);
    const grid = container.querySelector(".team-split-grid") as HTMLElement;
    expect(grid.style.gridTemplateColumns).toBe("60% 1fr");
  });

  it("uses default 40% width from store initial state", () => {
    const { container } = render(<TeamSplitGrid />);
    const grid = container.querySelector(".team-split-grid") as HTMLElement;
    expect(grid.style.gridTemplateColumns).toBe("40% 1fr");
  });
});
