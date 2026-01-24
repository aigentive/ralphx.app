/**
 * Tests for StateHistoryTimeline component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StateHistoryTimeline } from "./StateHistoryTimeline";

// Mock useTaskStateHistory hook
const mockUseTaskStateHistory = vi.fn();
vi.mock("@/hooks/useReviews", () => ({
  useTaskStateHistory: (...args: unknown[]) => mockUseTaskStateHistory(...args),
}));

function createQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
}

function renderWithProviders(ui: React.ReactElement) {
  return render(
    <QueryClientProvider client={createQueryClient()}>{ui}</QueryClientProvider>
  );
}

describe("StateHistoryTimeline", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("loading state", () => {
    it("should render loading spinner when data is loading", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [],
        isLoading: true,
        isEmpty: true,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByTestId("timeline-loading")).toBeInTheDocument();
    });
  });

  describe("empty state", () => {
    it("should render empty state when no history exists", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [],
        isLoading: false,
        isEmpty: true,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByTestId("timeline-empty")).toBeInTheDocument();
      expect(screen.getByText("No history")).toBeInTheDocument();
    });
  });

  describe("timeline entries", () => {
    const mockHistory = [
      {
        id: "note-1",
        task_id: "task-123",
        reviewer: "human" as const,
        outcome: "approved" as const,
        notes: "Looks good, nice work",
        created_at: new Date().toISOString(),
      },
      {
        id: "note-2",
        task_id: "task-123",
        reviewer: "ai" as const,
        outcome: "changes_requested" as const,
        notes: "Security-sensitive: adds auth bypass",
        created_at: new Date(Date.now() - 15 * 60 * 1000).toISOString(),
      },
    ];

    it("should render timeline entries", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: mockHistory,
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByTestId("timeline-container")).toBeInTheDocument();
      expect(screen.getAllByTestId(/^timeline-entry-/)).toHaveLength(2);
    });

    it("should display outcome label", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: mockHistory,
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByText("Approved")).toBeInTheDocument();
      expect(screen.getByText("Changes Requested")).toBeInTheDocument();
    });

    it("should display reviewer actor", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: mockHistory,
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByText("by: user")).toBeInTheDocument();
      expect(screen.getByText("by: ai_reviewer")).toBeInTheDocument();
    });

    it("should display notes when present", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: mockHistory,
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByText('"Looks good, nice work"')).toBeInTheDocument();
      expect(
        screen.getByText('"Security-sensitive: adds auth bypass"')
      ).toBeInTheDocument();
    });

    it("should not display notes container when notes are null", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-3",
            task_id: "task-123",
            reviewer: "ai" as const,
            outcome: "approved" as const,
            notes: null,
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.queryByText('"')).not.toBeInTheDocument();
    });

    it("should display relative timestamps", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: mockHistory,
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      // Should show relative time like "just now" or "15 min ago"
      const entries = screen.getAllByTestId(/^timeline-entry-/);
      expect(entries[0]).toHaveAttribute("data-timestamp");
    });
  });

  describe("outcome colors", () => {
    it("should apply green color for approved outcome", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "human" as const,
            outcome: "approved" as const,
            notes: null,
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      const dot = screen.getByTestId("timeline-dot-note-1");
      expect(dot.style.backgroundColor).toBe("var(--status-success)");
    });

    it("should apply orange color for changes_requested outcome", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "ai" as const,
            outcome: "changes_requested" as const,
            notes: null,
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      const dot = screen.getByTestId("timeline-dot-note-1");
      expect(dot.style.backgroundColor).toBe("var(--status-warning)");
    });

    it("should apply red color for rejected outcome", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "ai" as const,
            outcome: "rejected" as const,
            notes: null,
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      const dot = screen.getByTestId("timeline-dot-note-1");
      expect(dot.style.backgroundColor).toBe("var(--status-error)");
    });
  });

  describe("actor mapping", () => {
    it("should map 'human' reviewer to 'user'", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "human" as const,
            outcome: "approved" as const,
            notes: null,
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByText("by: user")).toBeInTheDocument();
    });

    it("should map 'ai' reviewer to 'ai_reviewer'", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "ai" as const,
            outcome: "approved" as const,
            notes: null,
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByText("by: ai_reviewer")).toBeInTheDocument();
    });
  });

  describe("hook integration", () => {
    it("should pass taskId to useTaskStateHistory", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [],
        isLoading: false,
        isEmpty: true,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-456" />);

      expect(mockUseTaskStateHistory).toHaveBeenCalledWith("task-456");
    });
  });

  describe("data attributes", () => {
    it("should have data-testid on container", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "ai" as const,
            outcome: "approved" as const,
            notes: null,
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      expect(screen.getByTestId("timeline-container")).toBeInTheDocument();
    });
  });

  describe("styling", () => {
    it("should use design system tokens", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "ai" as const,
            outcome: "approved" as const,
            notes: "Test notes",
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<StateHistoryTimeline taskId="task-123" />);

      const container = screen.getByTestId("timeline-container");
      expect(container.style.backgroundColor).toBe("var(--bg-surface)");
    });
  });
});
