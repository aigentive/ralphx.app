/**
 * Tests for PlanSelectorInline component
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PlanSelectorInline } from "./PlanSelectorInline";
import { usePlanStore, type PlanCandidate } from "@/stores/planStore";

vi.mock("@/stores/planStore", () => ({
  usePlanStore: vi.fn(),
}));

const createTestCandidate = (
  overrides: Partial<PlanCandidate> = {}
): PlanCandidate => ({
  sessionId: "session-1",
  title: "Test Plan",
  acceptedAt: "2026-01-24T12:00:00Z",
  taskStats: {
    total: 10,
    incomplete: 5,
    activeNow: 2,
  },
  interactionStats: {
    selectedCount: 3,
    lastSelectedAt: "2026-01-24T12:00:00Z",
  },
  score: 0.75,
  ...overrides,
});

describe("PlanSelectorInline", () => {
  const mockOnOpenPalette = vi.fn();

  const defaultStoreState = {
    activePlanByProject: {},
    planCandidates: [],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: typeof defaultStoreState) => unknown) =>
        selector(defaultStoreState)
    );
  });

  it("shows 'Select plan' when no plan is active", () => {
    render(
      <PlanSelectorInline
        projectId="project-1"
        source="kanban_inline"
        onOpenPalette={mockOnOpenPalette}
      />
    );

    expect(screen.getByRole("button")).toHaveTextContent("Select plan");
  });

  it("shows active plan title and task count", () => {
    const activePlan = createTestCandidate({
      sessionId: "session-1",
      title: "My Active Plan",
      taskStats: { total: 20, incomplete: 8, activeNow: 3 },
    });

    (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: typeof defaultStoreState) => unknown) =>
        selector({
          ...defaultStoreState,
          activePlanByProject: { "project-1": "session-1" },
          planCandidates: [activePlan],
        })
    );

    render(
      <PlanSelectorInline
        projectId="project-1"
        source="kanban_inline"
        onOpenPalette={mockOnOpenPalette}
      />
    );

    const button = screen.getByRole("button");
    expect(button).toHaveTextContent("My Active Plan");
    expect(button).toHaveTextContent("8/20");
  });

  it("shows 'Untitled Plan' when active plan has no title", () => {
    const activePlan = createTestCandidate({
      sessionId: "session-1",
      title: null,
    });

    (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: typeof defaultStoreState) => unknown) =>
        selector({
          ...defaultStoreState,
          activePlanByProject: { "project-1": "session-1" },
          planCandidates: [activePlan],
        })
    );

    render(
      <PlanSelectorInline
        projectId="project-1"
        source="graph_inline"
        onOpenPalette={mockOnOpenPalette}
      />
    );

    expect(screen.getByRole("button")).toHaveTextContent("Untitled Plan");
  });

  it("hides labels in compact mode", () => {
    const activePlan = createTestCandidate({
      sessionId: "session-1",
      title: "My Plan",
      taskStats: { total: 20, incomplete: 8, activeNow: 3 },
    });

    (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: typeof defaultStoreState) => unknown) =>
        selector({
          ...defaultStoreState,
          activePlanByProject: { "project-1": "session-1" },
          planCandidates: [activePlan],
        })
    );

    render(
      <PlanSelectorInline
        projectId="project-1"
        source="kanban_inline"
        onOpenPalette={mockOnOpenPalette}
        compact
      />
    );

    const button = screen.getByRole("button");
    expect(button).not.toHaveTextContent("My Plan");
    expect(button).not.toHaveTextContent("8/20");
  });

  it("opens palette with the inline source when clicked", async () => {
    const user = userEvent.setup();

    render(
      <PlanSelectorInline
        projectId="project-1"
        source="graph_inline"
        onOpenPalette={mockOnOpenPalette}
      />
    );

    await user.click(screen.getByRole("button"));

    expect(mockOnOpenPalette).toHaveBeenCalledWith("graph_inline");
    expect(screen.queryByPlaceholderText("Search plans...")).not.toBeInTheDocument();
  });
});
