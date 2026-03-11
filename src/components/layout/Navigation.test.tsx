/**
 * Navigation component tests — team pill visibility + teammate count
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { Navigation } from "./Navigation";

// Dynamic mock state
let mockState = {
  activeTeams: {} as Record<string, { teammates: Record<string, unknown> }>,
};

vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (selector: (state: typeof mockState) => unknown) => selector(mockState),
  selectHasAnyActiveTeam: (state: typeof mockState) =>
    Object.keys(state.activeTeams).length > 0,
  selectTotalTeammateCount: (state: typeof mockState) =>
    Object.values(state.activeTeams).reduce(
      (sum: number, team) => sum + Object.keys(team.teammates).length,
      0,
    ),
}));

// Mock useProjectStats — avoids needing QueryClientProvider in Navigation tests
vi.mock("@/hooks/useProjectStats", () => ({
  useProjectStats: vi.fn(() => ({ data: undefined, isLoading: false, isError: false })),
}));

// Mock tooltip components to avoid portal issues
vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  TooltipTrigger: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  TooltipContent: ({ children }: { children: React.ReactNode }) => <span>{children}</span>,
}));

describe("Navigation", () => {
  const defaultProps = {
    currentView: "kanban" as const,
    onViewChange: vi.fn(),
  };

  beforeEach(() => {
    mockState = { activeTeams: {} };
  });

  it("renders all nav items", () => {
    render(<Navigation {...defaultProps} />);

    expect(screen.getByTestId("nav-ideation")).toBeInTheDocument();
    expect(screen.getByTestId("nav-kanban")).toBeInTheDocument();
    expect(screen.getByTestId("nav-graph")).toBeInTheDocument();
    expect(screen.getByTestId("nav-activity")).toBeInTheDocument();
  });

  it("shows team pill when hasActiveTeam is true", () => {
    mockState = {
      activeTeams: {
        "ctx-1": { teammates: { "t-1": {}, "t-2": {}, "t-3": {}, "t-4": {} } },
      },
    };

    render(<Navigation {...defaultProps} />);

    expect(screen.getByText("4")).toBeInTheDocument();
  });

  it("displays correct teammate count in pill", () => {
    mockState = {
      activeTeams: {
        "ctx-1": { teammates: { "t-1": {}, "t-2": {}, "t-3": {} } },
        "ctx-2": { teammates: { "t-4": {}, "t-5": {}, "t-6": {}, "t-7": {} } },
      },
    };

    render(<Navigation {...defaultProps} />);

    expect(screen.getByText("7")).toBeInTheDocument();
  });

  it("hides team pill when no active teams", () => {
    mockState = { activeTeams: {} };

    render(<Navigation {...defaultProps} />);

    // No teammate count element should exist
    const nav = screen.getByRole("navigation");
    expect(nav.querySelector(".rounded-full")).toBeNull();
  });
});
