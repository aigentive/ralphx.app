/**
 * MergingTaskDetail component tests
 *
 * Covers: progress rendering with mock merge_progress events,
 * reload-style remount recovery, fallback when events are missing/delayed,
 * and validation progress display.
 */

import React from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, act, cleanup } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MergingTaskDetail } from "./MergingTaskDetail";
import type { Task } from "@/types/task";
import type { MergeProgressEvent } from "@/types/events";

// Stable mock listeners for EventBus
const mockListeners = new Map<string, Set<(payload: unknown) => void>>();

const stableBus = {
  subscribe: (eventName: string, callback: (payload: unknown) => void) => {
    if (!mockListeners.has(eventName)) {
      mockListeners.set(eventName, new Set());
    }
    mockListeners.get(eventName)!.add(callback);
    return () => {
      mockListeners.get(eventName)?.delete(callback);
    };
  },
};

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => stableBus,
}));

function emitEvent(eventName: string, payload: unknown) {
  const listeners = mockListeners.get(eventName);
  if (listeners) {
    for (const listener of listeners) {
      listener(payload);
    }
  }
}

function createTestTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task",
    description: "Test description",
    priority: 2,
    internalStatus: "pending_merge",
    needsReviewPoint: false,
    createdAt: "2026-01-28T12:00:00+00:00",
    updatedAt: "2026-01-28T12:00:00+00:00",
    startedAt: null,
    completedAt: null,
    archivedAt: null,
    blockedReason: null,
    taskBranch: "ralphx/ralphx/task-123",
    worktreePath: null,
    mergeCommitSha: null,
    metadata: null,
    ...overrides,
  };
}

function makeProgressEvent(
  overrides: Partial<MergeProgressEvent> = {}
): MergeProgressEvent {
  return {
    task_id: "task-123",
    phase: "worktree_setup",
    status: "started",
    message: "",
    timestamp: "2026-02-11T10:00:00Z",
    ...overrides,
  };
}

function TestWrapper({ children }: { children: React.ReactNode }) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

function renderWithProviders(ui: React.ReactElement) {
  return render(ui, { wrapper: TestWrapper });
}

describe("MergingTaskDetail", () => {
  beforeEach(() => {
    mockListeners.clear();
  });

  describe("progress rendering", () => {
    it("shows 'Waiting for merge progress...' when no progress events received", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      expect(screen.getByTestId("merge-resuming-section")).toBeInTheDocument();
      expect(screen.getByText("Waiting for merge progress...")).toBeInTheDocument();
    });

    it("renders phase timeline when progress events arrive", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      // Initially shows waiting state
      expect(screen.getByText("Waiting for merge progress...")).toBeInTheDocument();

      // Emit first progress event
      act(() => {
        emitEvent(
          "task:merge_progress",
          makeProgressEvent({ phase: "worktree_setup", status: "started", message: "Setting up..." })
        );
      });

      // Phase timeline should appear, waiting state should disappear
      expect(screen.getByTestId("merge-phase-timeline")).toBeInTheDocument();
      expect(screen.queryByText("Waiting for merge progress...")).not.toBeInTheDocument();
      expect(screen.getByText("Worktree Setup")).toBeInTheDocument();
    });

    it("renders sequence of phases in correct order with proper status indicators", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      // Emit full sequence
      const phases: Array<{ phase: MergeProgressEvent["phase"]; status: MergeProgressEvent["status"]; message: string }> = [
        { phase: "worktree_setup", status: "started", message: "Setting up worktree" },
        { phase: "worktree_setup", status: "passed", message: "" },
        { phase: "programmatic_merge", status: "started", message: "Merging branches" },
        { phase: "programmatic_merge", status: "passed", message: "" },
        { phase: "typecheck", status: "started", message: "Running type checker..." },
      ];

      for (const { phase, status, message } of phases) {
        act(() => {
          emitEvent("task:merge_progress", makeProgressEvent({ phase, status, message }));
        });
      }

      // All phases up to typecheck should be visible
      expect(screen.getByText("Worktree Setup")).toBeInTheDocument();
      expect(screen.getByText("Merge")).toBeInTheDocument();
      expect(screen.getByText("Type Check")).toBeInTheDocument();
      expect(screen.getByText("Running type checker...")).toBeInTheDocument();
    });

    it("updates phase status from started to passed", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      // Start worktree_setup
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "worktree_setup",
          status: "started",
          message: "Setting up...",
        }));
      });

      expect(screen.getByText("Setting up...")).toBeInTheDocument();

      // Complete worktree_setup
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "worktree_setup",
          status: "passed",
          message: "Done",
        }));
      });

      // Message for passed phases should not be shown (per MergePhaseTimeline behavior)
      expect(screen.queryByText("Done")).not.toBeInTheDocument();
      expect(screen.queryByText("Setting up...")).not.toBeInTheDocument();
    });

    it("shows failed phase message when a phase fails", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "worktree_setup",
          status: "passed",
        }));
      });

      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "programmatic_merge",
          status: "passed",
        }));
      });

      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "typecheck",
          status: "failed",
          message: "Type errors found in 3 files",
        }));
      });

      expect(screen.getByText("Type errors found in 3 files")).toBeInTheDocument();
    });

    it("renders full seven-phase sequence through finalize", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      const fullSequence: Array<{ phase: MergeProgressEvent["phase"]; status: MergeProgressEvent["status"] }> = [
        { phase: "worktree_setup", status: "passed" },
        { phase: "programmatic_merge", status: "passed" },
        { phase: "typecheck", status: "passed" },
        { phase: "lint", status: "passed" },
        { phase: "clippy", status: "passed" },
        { phase: "test", status: "passed" },
        { phase: "finalize", status: "started" },
      ];

      for (const { phase, status } of fullSequence) {
        act(() => {
          emitEvent("task:merge_progress", makeProgressEvent({ phase, status }));
        });
      }

      expect(screen.getByText("Worktree Setup")).toBeInTheDocument();
      expect(screen.getByText("Merge")).toBeInTheDocument();
      expect(screen.getByText("Type Check")).toBeInTheDocument();
      expect(screen.getByText("Lint")).toBeInTheDocument();
      expect(screen.getByText("Clippy")).toBeInTheDocument();
      expect(screen.getByText("Test")).toBeInTheDocument();
      expect(screen.getByText("Finalize")).toBeInTheDocument();
    });
  });

  describe("reload-style remount recovery", () => {
    it("accumulates events, loses them on unmount, and remounts to fresh state", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });

      // First render
      const { unmount } = renderWithProviders(<MergingTaskDetail task={task} />);

      // Emit some progress events
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "worktree_setup",
          status: "passed",
        }));
      });
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "programmatic_merge",
          status: "started",
        }));
      });

      // Should show phase timeline with accumulated events
      expect(screen.getByTestId("merge-phase-timeline")).toBeInTheDocument();
      expect(screen.getByText("Worktree Setup")).toBeInTheDocument();
      expect(screen.getByText("Merge")).toBeInTheDocument();

      // Simulate reload: unmount component
      unmount();

      // Events emitted while component is unmounted won't be captured
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "programmatic_merge",
          status: "passed",
        }));
      });
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "typecheck",
          status: "started",
        }));
      });

      // Remount: component starts fresh (hooks re-subscribe)
      renderWithProviders(<MergingTaskDetail task={task} />);

      // After remount with no new events, should show waiting state
      expect(screen.getByText("Waiting for merge progress...")).toBeInTheDocument();
      expect(screen.queryByTestId("merge-phase-timeline")).not.toBeInTheDocument();
    });

    it("recovers and displays new events after remount", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });

      // First render
      const { unmount } = renderWithProviders(<MergingTaskDetail task={task} />);

      // Emit initial event
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "worktree_setup",
          status: "started",
        }));
      });

      expect(screen.getByTestId("merge-phase-timeline")).toBeInTheDocument();

      // Unmount (simulate reload)
      unmount();
      cleanup();

      // Remount
      renderWithProviders(<MergingTaskDetail task={task} />);

      // Starts with waiting state
      expect(screen.getByText("Waiting for merge progress...")).toBeInTheDocument();

      // New events arrive after remount
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "typecheck",
          status: "started",
          message: "Checking types...",
        }));
      });

      // Component recovers and shows new events
      expect(screen.getByTestId("merge-phase-timeline")).toBeInTheDocument();
      expect(screen.getByText("Type Check")).toBeInTheDocument();
      expect(screen.getByText("Checking types...")).toBeInTheDocument();
    });
  });

  describe("fallback behavior when events are missing or delayed", () => {
    it("shows waiting state for pending_merge task with no events", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      // Should show the resuming/waiting section
      expect(screen.getByTestId("merge-resuming-section")).toBeInTheDocument();
      expect(screen.getByText("Waiting for merge progress...")).toBeInTheDocument();
    });

    it("does not show phase timeline or waiting state for historical pending_merge", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} isHistorical viewStatus="pending_merge" />);

      // Historical mode should not show live progress or waiting state
      expect(screen.queryByTestId("merge-phase-timeline")).not.toBeInTheDocument();
      expect(screen.queryByTestId("merge-resuming-section")).not.toBeInTheDocument();
    });

    it("does not show phase timeline or waiting state for merging (agent phase)", () => {
      const task = createTestTask({ internalStatus: "merging" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      // Agent phase doesn't show merge progress timeline
      expect(screen.queryByTestId("merge-phase-timeline")).not.toBeInTheDocument();
      expect(screen.queryByTestId("merge-resuming-section")).not.toBeInTheDocument();
    });

    it("does not show stale metadata validation log in live mode", () => {
      const metadata = JSON.stringify({
        validation_log: [
          {
            task_id: "task-123",
            phase: "validate",
            command: "npm run typecheck",
            path: ".",
            label: "Type Check",
            status: "success",
            duration_ms: 3200,
          },
        ],
      });

      const task = createTestTask({
        internalStatus: "pending_merge",
        metadata,
      });
      renderWithProviders(<MergingTaskDetail task={task} />);

      // In live mode (not historical), stale metadata should NOT be shown
      expect(screen.queryByTestId(`validation-progress-${task.id}`)).not.toBeInTheDocument();
    });

    it("shows metadata validation log in historical mode", () => {
      const metadata = JSON.stringify({
        validation_log: [
          {
            task_id: "task-123",
            phase: "validate",
            command: "npm run typecheck",
            path: ".",
            label: "Type Check",
            status: "success",
            duration_ms: 3200,
          },
          {
            task_id: "task-123",
            phase: "validate",
            command: "npm run lint",
            path: ".",
            label: "Lint",
            status: "failed",
            exit_code: 1,
            stderr: "ESLint error",
          },
        ],
      });

      const task = createTestTask({
        internalStatus: "pending_merge",
        metadata,
      });
      renderWithProviders(<MergingTaskDetail task={task} isHistorical viewStatus="pending_merge" />);

      // In historical mode, metadata should be shown
      expect(screen.getByTestId(`validation-progress-${task.id}`)).toBeInTheDocument();
      expect(screen.getByText("Merge Validation")).toBeInTheDocument();
    });

    it("ignores events for different task IDs", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          task_id: "other-task-999",
          phase: "worktree_setup",
          status: "started",
        }));
      });

      // Should still show waiting state since event was for different task
      expect(screen.getByText("Waiting for merge progress...")).toBeInTheDocument();
      expect(screen.queryByTestId("merge-phase-timeline")).not.toBeInTheDocument();
    });

    it("ignores malformed progress event payloads", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      act(() => {
        emitEvent("task:merge_progress", { invalid: "data" });
      });

      act(() => {
        emitEvent("task:merge_progress", null);
      });

      act(() => {
        emitEvent("task:merge_progress", "not an object");
      });

      // Should still show waiting state
      expect(screen.getByText("Waiting for merge progress...")).toBeInTheDocument();
    });

    it("transitions from waiting to phase timeline when first event arrives after delay", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      // Initially waiting
      expect(screen.getByText("Waiting for merge progress...")).toBeInTheDocument();

      // First event arrives (simulating delayed backend response)
      act(() => {
        emitEvent("task:merge_progress", makeProgressEvent({
          phase: "worktree_setup",
          status: "started",
          message: "Initializing...",
        }));
      });

      // Should switch to timeline
      expect(screen.queryByText("Waiting for merge progress...")).not.toBeInTheDocument();
      expect(screen.getByTestId("merge-phase-timeline")).toBeInTheDocument();
      expect(screen.getByText("Initializing...")).toBeInTheDocument();
    });
  });

  describe("pending_merge basic rendering", () => {
    it("renders merging-task-detail test id", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      expect(screen.getByTestId("merging-task-detail")).toBeInTheDocument();
    });

    it("shows 'Merging Changes...' title for active pending_merge", () => {
      const task = createTestTask({ internalStatus: "pending_merge" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      expect(screen.getByText("Merging Changes...")).toBeInTheDocument();
    });

    it("shows branch info", () => {
      const task = createTestTask({
        internalStatus: "pending_merge",
        taskBranch: "ralphx/ralphx/task-123",
      });
      renderWithProviders(<MergingTaskDetail task={task} />);

      expect(screen.getByText("ralphx/ralphx/task-123")).toBeInTheDocument();
    });
  });

  describe("merging (agent phase) rendering", () => {
    it("shows 'Resolving Merge Conflicts' for agent phase", () => {
      const task = createTestTask({ internalStatus: "merging" });
      renderWithProviders(<MergingTaskDetail task={task} />);

      expect(screen.getByText("Resolving Merge Conflicts")).toBeInTheDocument();
    });

    it("shows conflict files when present in metadata", () => {
      const metadata = JSON.stringify({
        conflict_files: ["src/main.ts", "src/lib/utils.ts"],
      });
      const task = createTestTask({ internalStatus: "merging", metadata });
      renderWithProviders(<MergingTaskDetail task={task} />);

      expect(screen.getByTestId("conflict-files-section")).toBeInTheDocument();
      expect(screen.getByText("src/main.ts")).toBeInTheDocument();
      expect(screen.getByText("src/lib/utils.ts")).toBeInTheDocument();
    });
  });

  describe("validation recovery mode", () => {
    it("shows recovery UI when validation_recovery is true in metadata", () => {
      const metadata = JSON.stringify({
        validation_recovery: true,
        validation_failures: [
          { command: "npm run typecheck", exit_code: 1, stderr: "Type errors" },
        ],
      });
      const task = createTestTask({ internalStatus: "merging", metadata });
      renderWithProviders(<MergingTaskDetail task={task} />);

      // Recovery mode shows "Fixing Validation Errors..." title
      expect(screen.getByText("Fixing Validation Errors...")).toBeInTheDocument();
      // Validation failures are shown via ValidationProgress (no separate section)
      expect(screen.queryByTestId("validation-failures-section")).not.toBeInTheDocument();
    });
  });
});
