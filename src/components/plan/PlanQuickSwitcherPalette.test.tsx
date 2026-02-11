/**
 * Tests for PlanQuickSwitcherPalette keyboard navigation polish
 *
 * These tests verify:
 * - Scroll behavior when navigating with keyboard
 * - Focus ring visibility on highlighted items
 * - Home/End key navigation
 * - Edge cases (empty list, wrapping)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PlanQuickSwitcherPalette } from "./PlanQuickSwitcherPalette";
import { usePlanStore } from "@/stores/planStore";

// Mock the store
vi.mock("@/stores/planStore");

// Mock framer-motion to avoid animation issues in tests
vi.mock("framer-motion", () => ({
  AnimatePresence: ({ children }: { children: React.ReactNode }) => children,
  motion: {
    div: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  },
}));

describe("PlanQuickSwitcherPalette - Keyboard Navigation", () => {
  const mockOnClose = vi.fn();
  const mockSetActivePlan = vi.fn();
  const mockLoadCandidates = vi.fn();

  // Generate test plans (100+ for performance testing)
  const generatePlans = (count: number) =>
    Array.from({ length: count }, (_, i) => ({
      sessionId: `session-${i}`,
      title: `Plan ${i + 1}`,
      acceptedAt: new Date(Date.now() - i * 86400000).toISOString(),
      taskStats: {
        total: 10,
        incomplete: 5,
        activeNow: i % 3 === 0 ? 1 : 0,
      },
      interactionStats: {
        selectedCount: i % 5,
        lastSelectedAt: null,
      },
      score: 0.5,
    }));

  beforeEach(() => {
    vi.clearAllMocks();

    // Setup default mock implementation
    (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: any) => {
        const state = {
          activePlanByProject: {},
          planCandidates: generatePlans(5),
          isLoading: false,
          error: null,
          loadCandidates: mockLoadCandidates,
          setActivePlan: mockSetActivePlan,
        };
        return selector(state);
      }
    );
  });

  describe("Scroll behavior", () => {
    it("should scroll highlighted item into view when navigating down", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Mock scrollIntoView
      const scrollIntoViewMock = vi.fn();
      Element.prototype.scrollIntoView = scrollIntoViewMock;

      // Navigate down
      await user.keyboard("{ArrowDown}");

      await waitFor(() => {
        expect(scrollIntoViewMock).toHaveBeenCalledWith({
          block: "nearest",
          behavior: "smooth",
        });
      });
    });

    it("should scroll highlighted item into view when navigating up", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      const scrollIntoViewMock = vi.fn();
      Element.prototype.scrollIntoView = scrollIntoViewMock;

      // Navigate up (should wrap to last item)
      await user.keyboard("{ArrowUp}");

      await waitFor(() => {
        expect(scrollIntoViewMock).toHaveBeenCalledWith({
          block: "nearest",
          behavior: "smooth",
        });
      });
    });

    it("should handle rapid keyboard navigation smoothly", async () => {
      const user = userEvent.setup();
      const longList = generatePlans(100);

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: any) => {
          const state = {
            activePlanByProject: {},
            planCandidates: longList,
            isLoading: false,
            error: null,
            loadCandidates: mockLoadCandidates,
            setActivePlan: mockSetActivePlan,
          };
          return selector(state);
        }
      );

      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      const scrollIntoViewMock = vi.fn();
      Element.prototype.scrollIntoView = scrollIntoViewMock;

      // Rapidly navigate down 10 times
      for (let i = 0; i < 10; i++) {
        await user.keyboard("{ArrowDown}");
      }

      // Should have called scrollIntoView for each navigation
      expect(scrollIntoViewMock).toHaveBeenCalledTimes(10);
    });
  });

  describe("Home/End key navigation", () => {
    it("should jump to first item when Home key is pressed", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Navigate down a few times
      await user.keyboard("{ArrowDown}");
      await user.keyboard("{ArrowDown}");

      // Press Home
      await user.keyboard("{Home}");

      // First item should be highlighted
      const firstButton = screen.getByText("Plan 1").closest("button");
      expect(firstButton).toHaveClass("bg-white/10");
    });

    it("should jump to last item when End key is pressed", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Press End
      await user.keyboard("{End}");

      // Last item should be highlighted
      const lastButton = screen.getByText("Plan 5").closest("button");
      expect(lastButton).toHaveClass("bg-white/10");
    });

    it("should scroll to last item when End is pressed with long list", async () => {
      const user = userEvent.setup();
      const longList = generatePlans(150);

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: any) => {
          const state = {
            activePlanByProject: {},
            planCandidates: longList,
            isLoading: false,
            error: null,
            loadCandidates: mockLoadCandidates,
            setActivePlan: mockSetActivePlan,
          };
          return selector(state);
        }
      );

      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      const scrollIntoViewMock = vi.fn();
      Element.prototype.scrollIntoView = scrollIntoViewMock;

      // Press End
      await user.keyboard("{End}");

      await waitFor(() => {
        expect(scrollIntoViewMock).toHaveBeenCalledWith({
          block: "nearest",
          behavior: "smooth",
        });
      });
    });
  });

  describe("Edge cases", () => {
    it("should handle empty list gracefully", async () => {
      const user = userEvent.setup();

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: any) => {
          const state = {
            activePlanByProject: {},
            planCandidates: [],
            isLoading: false,
            error: null,
            loadCandidates: mockLoadCandidates,
            setActivePlan: mockSetActivePlan,
          };
          return selector(state);
        }
      );

      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Navigation keys should not throw errors
      await user.keyboard("{ArrowDown}");
      await user.keyboard("{ArrowUp}");
      await user.keyboard("{Home}");
      await user.keyboard("{End}");

      expect(screen.getByText(/no accepted plans found/i)).toBeInTheDocument();
    });

    it("should wrap from last to first when pressing ArrowDown", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Go to last item
      await user.keyboard("{End}");

      // Press ArrowDown (should wrap to first)
      await user.keyboard("{ArrowDown}");

      const firstButton = screen.getByText("Plan 1").closest("button");
      expect(firstButton).toHaveClass("bg-white/10");
    });

    it("should wrap from first to last when pressing ArrowUp", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Start at first item (index 0)
      // Press ArrowUp (should wrap to last)
      await user.keyboard("{ArrowUp}");

      const lastButton = screen.getByText("Plan 5").closest("button");
      expect(lastButton).toHaveClass("bg-white/10");
    });

    it("should reset highlighted index when search query changes", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Navigate down
      await user.keyboard("{ArrowDown}");
      await user.keyboard("{ArrowDown}");

      // Type in search (should reset to first item)
      await user.type(input, "Plan");

      // First matching item should be highlighted
      await waitFor(() => {
        const firstMatchingButton = screen.getByText("Plan 1").closest("button");
        expect(firstMatchingButton).toHaveClass("bg-white/10");
      });
    });
  });

  describe("Focus ring accessibility", () => {
    it("should show focus ring on highlighted item", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Navigate down
      await user.keyboard("{ArrowDown}");

      // Second item should have focus ring
      const secondButton = screen.getByText("Plan 2").closest("button");
      expect(secondButton).toHaveClass("ring-2");
      expect(secondButton).toHaveClass("ring-[#ff6b35]");
    });

    it("should move focus ring when navigating", async () => {
      const user = userEvent.setup();
      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // First item should be highlighted initially
      let highlightedButton = screen.getByText("Plan 1").closest("button");
      expect(highlightedButton).toHaveClass("ring-2");

      // Navigate down
      await user.keyboard("{ArrowDown}");

      // First item should no longer have ring
      expect(highlightedButton).not.toHaveClass("ring-2");

      // Second item should have ring
      highlightedButton = screen.getByText("Plan 2").closest("button");
      expect(highlightedButton).toHaveClass("ring-2");
    });
  });

  describe("Performance with long lists", () => {
    it("should handle 100+ items without performance degradation", async () => {
      const user = userEvent.setup();
      const longList = generatePlans(150);

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: any) => {
          const state = {
            activePlanByProject: {},
            planCandidates: longList,
            isLoading: false,
            error: null,
            loadCandidates: mockLoadCandidates,
            setActivePlan: mockSetActivePlan,
          };
          return selector(state);
        }
      );

      const startTime = performance.now();

      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const renderTime = performance.now() - startTime;

      // Rendering 150 items should be fast (< 1000ms)
      expect(renderTime).toBeLessThan(1000);

      const input = screen.getByPlaceholderText(/search plans/i);
      await user.click(input);

      // Navigation should be responsive
      const navStartTime = performance.now();
      await user.keyboard("{End}");
      await user.keyboard("{Home}");
      const navTime = performance.now() - navStartTime;

      // Navigation should be fast (< 100ms)
      expect(navTime).toBeLessThan(100);
    });

    it("should render all 150 items in the list", () => {
      const longList = generatePlans(150);

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
        (selector: any) => {
          const state = {
            activePlanByProject: {},
            planCandidates: longList,
            isLoading: false,
            error: null,
            loadCandidates: mockLoadCandidates,
            setActivePlan: mockSetActivePlan,
          };
          return selector(state);
        }
      );

      render(
        <PlanQuickSwitcherPalette
          projectId="test-project"
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      // Check first and last items exist
      expect(screen.getByText("Plan 1")).toBeInTheDocument();
      expect(screen.getByText("Plan 150")).toBeInTheDocument();
    });
  });
});
