/**
 * Tests for StepProgressBar component
 * Verifies progress dots rendering, status colors, and compact/full modes
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StepProgressBar } from "./StepProgressBar";
import { api } from "@/lib/tauri";
import type { StepProgressSummary } from "@/types/task-step";

// Mock the API
vi.mock("@/lib/tauri", () => ({
  api: {
    steps: {
      getProgress: vi.fn(),
    },
  },
}));

const mockApi = api as {
  steps: {
    getProgress: ReturnType<typeof vi.fn>;
  };
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

const mockProgress: StepProgressSummary = {
  taskId: "task-1",
  total: 7,
  completed: 2,
  inProgress: 1,
  pending: 3,
  skipped: 1,
  failed: 0,
  currentStep: {
    id: "step-3",
    taskId: "task-1",
    title: "Add OAuth providers",
    description: null,
    status: "in_progress",
    sortOrder: 2,
    dependsOn: null,
    createdBy: "proposal",
    completionNote: null,
    createdAt: "2026-01-26T00:00:00Z",
    updatedAt: "2026-01-26T00:00:00Z",
    startedAt: "2026-01-26T00:10:00Z",
    completedAt: null,
  },
  nextStep: {
    id: "step-4",
    taskId: "task-1",
    title: "Add session management",
    description: null,
    status: "pending",
    sortOrder: 3,
    dependsOn: null,
    createdBy: "proposal",
    completionNote: null,
    createdAt: "2026-01-26T00:00:00Z",
    updatedAt: "2026-01-26T00:00:00Z",
    startedAt: null,
    completedAt: null,
  },
  percentComplete: 42.86,
};

describe("StepProgressBar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Loading State", () => {
    it("returns null while loading", () => {
      mockApi.steps.getProgress.mockReturnValue(
        new Promise(() => {}) // Never resolves
      );

      const { container } = render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      expect(container.firstChild).toBeNull();
    });
  });

  describe("Empty State", () => {
    it("returns null when no data", async () => {
      mockApi.steps.getProgress.mockResolvedValue(null);

      const { container } = render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        expect(container.firstChild).toBeNull();
      });
    });

    it("returns null when total is 0", async () => {
      mockApi.steps.getProgress.mockResolvedValue({
        ...mockProgress,
        total: 0,
      });

      const { container } = render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        expect(container.firstChild).toBeNull();
      });
    });
  });

  describe("Progress Dots", () => {
    it("renders correct number of dots", async () => {
      mockApi.steps.getProgress.mockResolvedValue(mockProgress);

      const { container } = render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        const dots = container.querySelectorAll(".h-1\\.5.w-1\\.5.rounded-full");
        expect(dots).toHaveLength(7);
      });
    });

    it("applies correct color classes for step statuses", async () => {
      mockApi.steps.getProgress.mockResolvedValue(mockProgress);

      const { container } = render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        const dots = container.querySelectorAll(".h-1\\.5.w-1\\.5.rounded-full");

        // First 2 completed (green)
        expect(dots[0].className).toContain("bg-status-success");
        expect(dots[1].className).toContain("bg-status-success");

        // Next 1 skipped (muted)
        expect(dots[2].className).toContain("bg-text-muted");

        // Next 1 in_progress (accent with pulse)
        expect(dots[3].className).toContain("bg-accent-primary");
        expect(dots[3].className).toContain("animate-pulse");

        // Remaining 3 pending (border color)
        expect(dots[4].className).toContain("bg-border-default");
        expect(dots[5].className).toContain("bg-border-default");
        expect(dots[6].className).toContain("bg-border-default");
      });
    });

    it("handles failed steps correctly", async () => {
      const progressWithFailed: StepProgressSummary = {
        ...mockProgress,
        completed: 2,
        skipped: 0,
        failed: 1,
        inProgress: 1,
        pending: 3,
      };

      mockApi.steps.getProgress.mockResolvedValue(progressWithFailed);

      const { container } = render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        const dots = container.querySelectorAll(".h-1\\.5.w-1\\.5.rounded-full");

        // First 2 completed
        expect(dots[0].className).toContain("bg-status-success");
        expect(dots[1].className).toContain("bg-status-success");

        // Next 1 failed (error red)
        expect(dots[2].className).toContain("bg-status-error");

        // Next 1 in_progress
        expect(dots[3].className).toContain("bg-accent-primary");
      });
    });
  });

  describe("Compact Mode", () => {
    it("does not show text in compact mode", async () => {
      mockApi.steps.getProgress.mockResolvedValue(mockProgress);

      const { container } = render(<StepProgressBar taskId="task-1" compact />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        const dots = container.querySelectorAll(".h-1\\.5.w-1\\.5.rounded-full");
        expect(dots).toHaveLength(7);
      });

      // Should not contain text summary
      expect(screen.queryByText("3/7")).not.toBeInTheDocument();
    });
  });

  describe("Full Mode", () => {
    it("shows text summary in full mode", async () => {
      mockApi.steps.getProgress.mockResolvedValue(mockProgress);

      render(<StepProgressBar taskId="task-1" compact={false} />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        // completed (2) + skipped (1) = 3
        expect(screen.getByText("3/7")).toBeInTheDocument();
      });
    });

    it("calculates completed + skipped correctly", async () => {
      const progress: StepProgressSummary = {
        ...mockProgress,
        total: 10,
        completed: 4,
        skipped: 2,
        failed: 0,
        inProgress: 1,
        pending: 3,
      };

      mockApi.steps.getProgress.mockResolvedValue(progress);

      render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        // 4 completed + 2 skipped = 6
        expect(screen.getByText("6/10")).toBeInTheDocument();
      });
    });
  });

  describe("Integration", () => {
    it("works with all completed steps", async () => {
      const allCompleted: StepProgressSummary = {
        ...mockProgress,
        total: 5,
        completed: 5,
        inProgress: 0,
        pending: 0,
        skipped: 0,
        failed: 0,
        percentComplete: 100,
      };

      mockApi.steps.getProgress.mockResolvedValue(allCompleted);

      const { container } = render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        const dots = container.querySelectorAll(".h-1\\.5.w-1\\.5.rounded-full");
        expect(dots).toHaveLength(5);

        // All should be completed (green)
        dots.forEach((dot) => {
          expect(dot.className).toContain("bg-status-success");
        });
      });

      expect(screen.getByText("5/5")).toBeInTheDocument();
    });

    it("works with all pending steps", async () => {
      const allPending: StepProgressSummary = {
        ...mockProgress,
        total: 5,
        completed: 0,
        inProgress: 0,
        pending: 5,
        skipped: 0,
        failed: 0,
        percentComplete: 0,
      };

      mockApi.steps.getProgress.mockResolvedValue(allPending);

      const { container } = render(<StepProgressBar taskId="task-1" />, {
        wrapper: createWrapper(),
      });

      await vi.waitFor(() => {
        const dots = container.querySelectorAll(".h-1\\.5.w-1\\.5.rounded-full");
        expect(dots).toHaveLength(5);

        // All should be pending (border color)
        dots.forEach((dot) => {
          expect(dot.className).toContain("bg-border-default");
        });
      });

      expect(screen.getByText("0/5")).toBeInTheDocument();
    });
  });
});
