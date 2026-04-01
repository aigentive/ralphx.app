/**
 * MergeConflictTaskDetail component tests
 * Tests rendering and error handling in action handlers (handleResolveConflicts, handleRetryMerge)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MergeConflictTaskDetail } from "./MergeConflictTaskDetail";
import type { Task } from "@/types/task";
import { invoke } from "@tauri-apps/api/core";

// Mock Tauri API — default to returning resolved promise (matches real invoke behavior)
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

const mockInvoke = vi.mocked(invoke);

function createTestTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task Title",
    description: "Test task description",
    priority: 2,
    internalStatus: "merge_conflict",
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

/** Helper: set up invoke mock that handles conflict detection and rejects a specific command */
function mockInvokeWithRejection(rejectCmd: string, rejection: unknown) {
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === "detect_merge_conflicts") return Promise.resolve([]);
    if (cmd === rejectCmd) return Promise.reject(rejection);
    return Promise.resolve(undefined);
  });
}

describe("MergeConflictTaskDetail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default invoke mock: return empty array for conflict detection, undefined for others
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_merge_conflicts") return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
  });

  it("renders container with status banner", () => {
    const task = createTestTask();
    render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("merge-conflict-task-detail")).toBeInTheDocument();
    expect(screen.getByText("Merge Conflict")).toBeInTheDocument();
  });

  describe("conflict type distinction in status banner", () => {
    it("shows 'Plan Update Conflict' for plan_update_conflict metadata", () => {
      const metadata = JSON.stringify({ plan_update_conflict: true });
      const task = createTestTask({ metadata });
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByText("Plan Update Conflict")).toBeInTheDocument();
      expect(screen.getByText("Manual resolution required to update plan from main")).toBeInTheDocument();
    });

    it("shows 'Task Update Conflict' for source_update_conflict metadata", () => {
      const metadata = JSON.stringify({ source_update_conflict: true });
      const task = createTestTask({ metadata });
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByText("Task Update Conflict")).toBeInTheDocument();
      expect(screen.getByText("Manual resolution required to update task from plan")).toBeInTheDocument();
    });

    it("shows 'Merge Conflict' when no conflict type flag is set", () => {
      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByText("Merge Conflict")).toBeInTheDocument();
      expect(screen.getByText("Manual resolution required")).toBeInTheDocument();
    });

    it("shows correct historical subtitle for plan_update_conflict", () => {
      const metadata = JSON.stringify({ plan_update_conflict: true });
      const task = createTestTask({ metadata });
      render(<MergeConflictTaskDetail task={task} isHistorical />, { wrapper: TestWrapper });

      expect(screen.getByText("Plan Update Conflict")).toBeInTheDocument();
      expect(screen.getByText("Manual resolution was required to update plan from main")).toBeInTheDocument();
    });

    it("shows correct historical subtitle for source_update_conflict", () => {
      const metadata = JSON.stringify({ source_update_conflict: true });
      const task = createTestTask({ metadata });
      render(<MergeConflictTaskDetail task={task} isHistorical />, { wrapper: TestWrapper });

      expect(screen.getByText("Task Update Conflict")).toBeInTheDocument();
      expect(screen.getByText("Manual resolution was required to update task from plan")).toBeInTheDocument();
    });

    it("shows 'Merge Conflict' and historical subtitle when no conflict type and isHistorical", () => {
      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} isHistorical />, { wrapper: TestWrapper });

      expect(screen.getByText("Merge Conflict")).toBeInTheDocument();
      expect(screen.getByText("Manual resolution was required")).toBeInTheDocument();
    });
  });

  it("shows conflict files from metadata", () => {
    const metadata = JSON.stringify({
      conflict_files: ["src/main.rs", "src/lib.rs"],
    });
    const task = createTestTask({ metadata });
    render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByText("src/main.rs")).toBeInTheDocument();
    expect(screen.getByText("src/lib.rs")).toBeInTheDocument();
  });

  it("shows action buttons in non-historical mode", () => {
    const task = createTestTask();
    render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("action-buttons")).toBeInTheDocument();
    expect(screen.getByTestId("retry-merge-button")).toBeInTheDocument();
    expect(screen.getByTestId("resolve-conflict-button")).toBeInTheDocument();
  });

  it("hides action buttons in historical mode", () => {
    const task = createTestTask();
    render(<MergeConflictTaskDetail task={task} isHistorical />, {
      wrapper: TestWrapper,
    });

    expect(screen.queryByTestId("action-buttons")).not.toBeInTheDocument();
  });

  describe("handleResolveConflicts error handling", () => {
    it("displays extracted message for string rejection (non-Error)", async () => {
      const user = userEvent.setup();
      mockInvokeWithRejection("resolve_merge_conflict", "Git merge failed: conflict in src/main.rs");

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("resolve-conflict-button"));

      await waitFor(() => {
        expect(screen.getByText("Git merge failed: conflict in src/main.rs")).toBeInTheDocument();
      });
    });

    it("displays extracted message for object rejection with .message", async () => {
      const user = userEvent.setup();
      mockInvokeWithRejection("resolve_merge_conflict", { message: "Branch not found" });

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("resolve-conflict-button"));

      await waitFor(() => {
        expect(screen.getByText("Branch not found")).toBeInTheDocument();
      });
    });

    it("displays fallback for unknown rejection type", async () => {
      const user = userEvent.setup();
      mockInvokeWithRejection("resolve_merge_conflict", 42);

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("resolve-conflict-button"));

      await waitFor(() => {
        expect(screen.getByText("Failed to mark conflicts as resolved")).toBeInTheDocument();
      });
    });

    it("displays Error instance message", async () => {
      const user = userEvent.setup();
      mockInvokeWithRejection("resolve_merge_conflict", new Error("Connection timeout"));

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("resolve-conflict-button"));

      await waitFor(() => {
        expect(screen.getByText("Connection timeout")).toBeInTheDocument();
      });
    });
  });

  describe("handleRetryMerge error handling", () => {
    it("displays extracted message for string rejection (non-Error)", async () => {
      const user = userEvent.setup();
      mockInvokeWithRejection("retry_merge", "Merge target branch is locked");

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("retry-merge-button"));

      await waitFor(() => {
        expect(screen.getByText("Merge target branch is locked")).toBeInTheDocument();
      });
    });

    it("displays extracted message for object rejection with .message", async () => {
      const user = userEvent.setup();
      mockInvokeWithRejection("retry_merge", { message: "Permission denied" });

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("retry-merge-button"));

      await waitFor(() => {
        expect(screen.getByText("Permission denied")).toBeInTheDocument();
      });
    });

    it("displays fallback for unknown rejection type", async () => {
      const user = userEvent.setup();
      mockInvokeWithRejection("retry_merge", undefined);

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("retry-merge-button"));

      await waitFor(() => {
        expect(screen.getByText("Failed to retry merge")).toBeInTheDocument();
      });
    });

    it("displays Error instance message", async () => {
      const user = userEvent.setup();
      mockInvokeWithRejection("retry_merge", new Error("Network error"));

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("retry-merge-button"));

      await waitFor(() => {
        expect(screen.getByText("Network error")).toBeInTheDocument();
      });
    });
  });

  describe("error clearing", () => {
    it("clears previous error when action succeeds", async () => {
      const user = userEvent.setup();
      // First click rejects resolve_merge_conflict, then we reset to succeed
      let shouldReject = true;
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "detect_merge_conflicts") return Promise.resolve([]);
        if (cmd === "resolve_merge_conflict" && shouldReject) {
          shouldReject = false;
          return Promise.reject(new Error("First failure"));
        }
        return Promise.resolve(undefined);
      });

      const task = createTestTask();
      render(<MergeConflictTaskDetail task={task} />, { wrapper: TestWrapper });

      // Trigger error
      await user.click(screen.getByTestId("resolve-conflict-button"));
      await waitFor(() => {
        expect(screen.getByText("First failure")).toBeInTheDocument();
      });

      // Retry succeeds - error should clear
      await user.click(screen.getByTestId("resolve-conflict-button"));
      await waitFor(() => {
        expect(screen.queryByText("First failure")).not.toBeInTheDocument();
      });
    });
  });
});
