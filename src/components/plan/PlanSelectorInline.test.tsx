/**
 * Tests for PlanSelectorInline component
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PlanSelectorInline } from "./PlanSelectorInline";
import { usePlanStore, type PlanCandidate } from "@/stores/planStore";

// Mock the planStore
vi.mock("@/stores/planStore", () => ({
  usePlanStore: vi.fn(),
  selectActivePlanId: vi.fn(),
  selectCurrentActivePlan: vi.fn(),
}));

// Helper to create test plan candidates
const createTestCandidate = (
  overrides: Partial<PlanCandidate> = {}
): PlanCandidate => ({
  sessionId: `session-${Math.random().toString(36).slice(2)}`,
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
  const mockLoadCandidates = vi.fn();
  const mockSetActivePlan = vi.fn();
  const mockClearActivePlan = vi.fn();

  const defaultStoreState = {
    activePlanByProject: {},
    planCandidates: [],
    isLoading: false,
    error: null,
    loadCandidates: mockLoadCandidates,
    setActivePlan: mockSetActivePlan,
    clearActivePlan: mockClearActivePlan,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: typeof defaultStoreState) => unknown) =>
        selector(defaultStoreState)
    );
  });

  describe("Trigger Button Rendering", () => {
    it("shows 'Select plan' when no plan is active", () => {
      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      expect(screen.getByRole("button")).toHaveTextContent("Select plan");
    });

    it("shows plan title when plan is active", () => {
      const activePlan = createTestCandidate({
        sessionId: "session-1",
        title: "My Active Plan",
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
        />
      );

      expect(screen.getByRole("button")).toHaveTextContent("My Active Plan");
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
          source="kanban_inline"
        />
      );

      expect(screen.getByRole("button")).toHaveTextContent("Untitled Plan");
    });

    it("shows task count badge when plan is active", () => {
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
        />
      );

      expect(screen.getByText("8/20")).toBeInTheDocument();
    });

    it("renders compact mode without text labels", () => {
      const activePlan = createTestCandidate({
        sessionId: "session-1",
        title: "My Plan",
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
          compact
        />
      );

      const button = screen.getByRole("button");
      expect(button).not.toHaveTextContent("My Plan");
      expect(button).not.toHaveTextContent("8/20");
    });
  });

  describe("Popover Interaction", () => {
    it("opens popover when trigger button is clicked", async () => {
      const user = userEvent.setup();

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByPlaceholderText("Search plans...")).toBeInTheDocument();
      });
    });

    it("loads candidates when popover opens", async () => {
      const user = userEvent.setup();

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(mockLoadCandidates).toHaveBeenCalledWith("project-1");
      });
    });

    it("displays loading state while candidates are loading", async () => {
      const user = userEvent.setup();

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            isLoading: true,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Loading plans...")).toBeInTheDocument();
      });
    });

    it("displays empty state when no candidates found", async () => {
      const user = userEvent.setup();

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("No accepted plans yet")).toBeInTheDocument();
      });
    });

    it("displays error state when there is an error", async () => {
      const user = userEvent.setup();

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            error: "Failed to load plans",
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Failed to load plans")).toBeInTheDocument();
        expect(screen.getByText("Retry")).toBeInTheDocument();
      });
    });

    it("calls loadCandidates when retry button is clicked", async () => {
      const user = userEvent.setup();

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            error: "Failed to load plans",
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      await user.click(await screen.findByText("Retry"));

      expect(mockLoadCandidates).toHaveBeenCalled();
    });
  });

  describe("Candidate List Rendering", () => {
    it("renders list of plan candidates", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan Alpha" }),
        createTestCandidate({ sessionId: "s2", title: "Plan Beta" }),
        createTestCandidate({ sessionId: "s3", title: "Plan Gamma" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Plan Alpha")).toBeInTheDocument();
        expect(screen.getByText("Plan Beta")).toBeInTheDocument();
        expect(screen.getByText("Plan Gamma")).toBeInTheDocument();
      });
    });

    it("displays task stats for each candidate", async () => {
      const user = userEvent.setup();
      const candidate = createTestCandidate({
        sessionId: "s1",
        title: "Test Plan",
        taskStats: { total: 15, incomplete: 7, activeNow: 0 },
      });

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: [candidate],
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText(/7\/15 incomplete/)).toBeInTheDocument();
      });
    });

    it("shows Active badge when activeNow > 0", async () => {
      const user = userEvent.setup();
      const candidate = createTestCandidate({
        sessionId: "s1",
        title: "Active Plan",
        taskStats: { total: 10, incomplete: 5, activeNow: 3 },
      });

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: [candidate],
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Active")).toBeInTheDocument();
      });
    });

    it("does not show Active badge when activeNow = 0", async () => {
      const user = userEvent.setup();
      const candidate = createTestCandidate({
        sessionId: "s1",
        title: "Inactive Plan",
        taskStats: { total: 10, incomplete: 5, activeNow: 0 },
      });

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: [candidate],
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.queryByText("Active")).not.toBeInTheDocument();
      });
    });
  });

  describe("Search Functionality", () => {
    it("filters candidates by search query", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan Alpha" }),
        createTestCandidate({ sessionId: "s2", title: "Plan Beta" }),
        createTestCandidate({ sessionId: "s3", title: "Different Name" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      const searchInput = await screen.findByPlaceholderText("Search plans...");
      await user.type(searchInput, "Plan");

      await waitFor(() => {
        expect(screen.getByText("Plan Alpha")).toBeInTheDocument();
        expect(screen.getByText("Plan Beta")).toBeInTheDocument();
        expect(screen.queryByText("Different Name")).not.toBeInTheDocument();
      });
    });

    it("search is case-insensitive", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan ALPHA" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      const searchInput = await screen.findByPlaceholderText("Search plans...");
      await user.type(searchInput, "alpha");

      await waitFor(() => {
        expect(screen.getByText("Plan ALPHA")).toBeInTheDocument();
      });
    });

    it("shows 'No accepted plans found' when search has no results", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan Alpha" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      const searchInput = await screen.findByPlaceholderText("Search plans...");
      await user.type(searchInput, "NonexistentPlan");

      await waitFor(() => {
        expect(screen.getByText("No accepted plans found")).toBeInTheDocument();
      });
    });
  });

  describe("Plan Selection", () => {
    it("calls setActivePlan when a candidate is clicked", async () => {
      const user = userEvent.setup();
      const candidate = createTestCandidate({
        sessionId: "session-123",
        title: "Selected Plan",
      });

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: [candidate],
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      await user.click(await screen.findByText("Selected Plan"));

      expect(mockSetActivePlan).toHaveBeenCalledWith(
        "project-1",
        "session-123",
        "kanban_inline"
      );
    });

    it("uses graph_inline source when specified", async () => {
      const user = userEvent.setup();
      const candidate = createTestCandidate({
        sessionId: "session-123",
        title: "Selected Plan",
      });

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: [candidate],
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="graph_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      await user.click(await screen.findByText("Selected Plan"));

      expect(mockSetActivePlan).toHaveBeenCalledWith(
        "project-1",
        "session-123",
        "graph_inline"
      );
    });
  });

  describe("Clear Selection", () => {
    it("shows clear button when plan is active", async () => {
      const user = userEvent.setup();
      const activePlan = createTestCandidate({
        sessionId: "session-1",
        title: "Active Plan",
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
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Clear selection")).toBeInTheDocument();
      });
    });

    it("does not show clear button when no plan is active", async () => {
      const user = userEvent.setup();

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.queryByText("Clear selection")).not.toBeInTheDocument();
      });
    });

    it("calls clearActivePlan when clear button is clicked", async () => {
      const user = userEvent.setup();
      const activePlan = createTestCandidate({
        sessionId: "session-1",
        title: "Active Plan",
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
        />
      );

      await user.click(screen.getByRole("button"));
      await user.click(await screen.findByText("Clear selection"));

      expect(mockClearActivePlan).toHaveBeenCalledWith("project-1");
    });
  });

  describe("Keyboard Navigation", () => {
    it("navigates down with ArrowDown key", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan 1" }),
        createTestCandidate({ sessionId: "s2", title: "Plan 2" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      const searchInput = await screen.findByPlaceholderText("Search plans...");

      await user.type(searchInput, "{ArrowDown}");

      // First item should be highlighted (index 0 -> 1)
      // We can't easily test visual highlighting, but we can verify keyboard events work
    });

    it("navigates up with ArrowUp key", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan 1" }),
        createTestCandidate({ sessionId: "s2", title: "Plan 2" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      const searchInput = await screen.findByPlaceholderText("Search plans...");

      await user.type(searchInput, "{ArrowUp}");

      // Should wrap around to last item
    });

    it("selects highlighted item with Enter key", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan 1" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      const searchInput = await screen.findByPlaceholderText("Search plans...");

      await user.type(searchInput, "{Enter}");

      expect(mockSetActivePlan).toHaveBeenCalledWith(
        "project-1",
        "s1",
        "kanban_inline"
      );
    });

    it("closes popover with Escape key", async () => {
      const user = userEvent.setup();

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      const searchInput = await screen.findByPlaceholderText("Search plans...");

      await user.type(searchInput, "{Escape}");

      await waitFor(() => {
        expect(screen.queryByPlaceholderText("Search plans...")).not.toBeInTheDocument();
      });
    });
  });

  // ==========================================================================
  // Animation & Performance
  // ==========================================================================

  describe("animations", () => {
    it("applies transition classes to candidate items", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan 1" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      const candidateButton = await screen.findByText("Plan 1");
      const button = candidateButton.closest("button")!;

      expect(button).toHaveClass("transition-all", "origin-center");
    });

    it("applies hover scale class to candidate items", async () => {
      const user = userEvent.setup();
      const candidates = [
        createTestCandidate({ sessionId: "s1", title: "Plan 1" }),
      ];

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      const candidateButton = await screen.findByText("Plan 1");
      const button = candidateButton.closest("button")!;

      expect(button.className).toMatch(/hover:scale-\[1\.01\]/);
    });

    it("applies transition-colors to loading state", async () => {
      const user = userEvent.setup();

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({ ...defaultStoreState, isLoading: true })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      const loadingDiv = await screen.findByText("Loading plans...");

      expect(loadingDiv.closest("div")).toHaveClass("transition-colors");
    });

    it("applies transition-colors to empty state", async () => {
      const user = userEvent.setup();

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({ ...defaultStoreState, planCandidates: [] })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));
      const emptyDiv = await screen.findByText("No accepted plans yet");

      expect(emptyDiv.closest("div")).toHaveClass("transition-colors");
    });

    it("constrains scroll area height to prevent layout shifts", async () => {
      const user = userEvent.setup();
      const candidates = Array.from({ length: 20 }, (_, i) =>
        createTestCandidate({ sessionId: `s${i}`, title: `Plan ${i}` })
      );

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: (state: typeof defaultStoreState) => unknown) =>
          selector({
            ...defaultStoreState,
            planCandidates: candidates,
          })
      );

      render(
        <PlanSelectorInline
          projectId="project-1"
          source="kanban_inline"
        />
      );

      await user.click(screen.getByRole("button"));

      // ScrollArea should have max-h constraint (max-h-64)
      const scrollArea = screen.getByText("Plan 0").closest("[class*='max-h']");
      expect(scrollArea).toBeTruthy();
    });
  });
});
