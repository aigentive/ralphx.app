/**
 * TaskContextMenuItems.test.tsx - Tests for the shared TaskContextMenuItems component
 *
 * Tests handler invocation through confirmation dialogs, status action rendering,
 * and the block reason dialog flow.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  TaskContextMenuItems,
  TaskContextMenuDialogs,
  useTaskContextMenuActions,
  type TaskContextMenuItemsHandlers,
} from "./TaskContextMenuItems";
import type { Task } from "@/types/task";

function createMockTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    projectId: "project-1",
    category: "feature",
    title: "Test Task",
    description: "Test description",
    priority: 3,
    internalStatus: "backlog",
    needsReviewPoint: false,
    sourceProposalId: null,
    planArtifactId: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    startedAt: null,
    completedAt: null,
    archivedAt: null,
    blockedReason: null,
    ...overrides,
  };
}

/**
 * Test wrapper that mirrors TaskCardContextMenu: hook at top,
 * items inside ContextMenuContent, dialogs outside.
 */
function TestContextMenu({ task, handlers }: { task: Task; handlers: TaskContextMenuItemsHandlers }) {
  const { menuHandlers, dialogProps } = useTaskContextMenuActions(handlers);

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <div data-testid="trigger">Trigger</div>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <TaskContextMenuItems task={task} handlers={handlers} menuHandlers={menuHandlers} />
        </ContextMenuContent>
      </ContextMenu>
      <TaskContextMenuDialogs dialogProps={dialogProps} onBlockWithReason={handlers.onBlockWithReason} />
    </>
  );
}

function renderInContextMenu(task: Task, handlers: TaskContextMenuItemsHandlers) {
  render(<TestContextMenu task={task} handlers={handlers} />);
  fireEvent.contextMenu(screen.getByTestId("trigger"));
}

describe("TaskContextMenuItems", () => {
  const mockHandlers = {
    onViewDetails: vi.fn(),
    onEdit: vi.fn(),
    onArchive: vi.fn(),
    onRestore: vi.fn(),
    onPermanentDelete: vi.fn(),
    onStatusChange: vi.fn(),
    onBlockWithReason: vi.fn(),
    onUnblock: vi.fn(),
  };

  beforeEach(() => {
    Object.values(mockHandlers).forEach((mock) => mock.mockClear());
  });

  describe("menu item visibility", () => {
    it("shows View Details for all tasks", () => {
      renderInContextMenu(createMockTask(), mockHandlers);
      expect(screen.getByText("View Details")).toBeInTheDocument();
    });

    it("shows Edit for editable tasks", () => {
      renderInContextMenu(createMockTask({ internalStatus: "backlog" }), mockHandlers);
      expect(screen.getByText("Edit")).toBeInTheDocument();
    });

    it("hides Edit for system-controlled statuses", () => {
      renderInContextMenu(createMockTask({ internalStatus: "executing" }), mockHandlers);
      expect(screen.queryByText("Edit")).not.toBeInTheDocument();
    });

    it("hides Edit for archived tasks", () => {
      renderInContextMenu(createMockTask({ archivedAt: new Date().toISOString() }), mockHandlers);
      expect(screen.queryByText("Edit")).not.toBeInTheDocument();
    });

    it("shows Archive for non-archived tasks", () => {
      renderInContextMenu(createMockTask(), mockHandlers);
      expect(screen.getByText("Archive")).toBeInTheDocument();
    });

    it("shows Restore and Delete Permanently for archived tasks", () => {
      renderInContextMenu(createMockTask({ archivedAt: new Date().toISOString() }), mockHandlers);
      expect(screen.getByText("Restore")).toBeInTheDocument();
      expect(screen.getByText("Delete Permanently")).toBeInTheDocument();
      expect(screen.queryByText("Archive")).not.toBeInTheDocument();
    });

    it("shows Cancel for backlog tasks", () => {
      renderInContextMenu(createMockTask({ internalStatus: "backlog" }), mockHandlers);
      expect(screen.getByText("Cancel")).toBeInTheDocument();
    });

    it("shows Block and Cancel for ready tasks", () => {
      renderInContextMenu(createMockTask({ internalStatus: "ready" }), mockHandlers);
      expect(screen.getByText("Block")).toBeInTheDocument();
      expect(screen.getByText("Cancel")).toBeInTheDocument();
    });

    it("shows Unblock and Cancel for blocked tasks", () => {
      renderInContextMenu(createMockTask({ internalStatus: "blocked" }), mockHandlers);
      expect(screen.getByText("Unblock")).toBeInTheDocument();
      expect(screen.getByText("Cancel")).toBeInTheDocument();
    });

    it("shows Re-open for approved tasks", () => {
      renderInContextMenu(createMockTask({ internalStatus: "approved" }), mockHandlers);
      expect(screen.getByText("Re-open")).toBeInTheDocument();
    });

    it("shows Retry for failed tasks", () => {
      renderInContextMenu(createMockTask({ internalStatus: "failed" }), mockHandlers);
      expect(screen.getByText("Retry")).toBeInTheDocument();
    });

    it("shows Re-open for cancelled tasks", () => {
      renderInContextMenu(createMockTask({ internalStatus: "cancelled" }), mockHandlers);
      expect(screen.getByText("Re-open")).toBeInTheDocument();
    });

    it("shows Start Ideation for backlog when handler provided", () => {
      renderInContextMenu(createMockTask({ internalStatus: "backlog" }), {
        ...mockHandlers,
        onStartIdeation: vi.fn(),
      });
      expect(screen.getByText("Start Ideation")).toBeInTheDocument();
    });

    it("hides Start Ideation when handler not provided", () => {
      renderInContextMenu(createMockTask({ internalStatus: "backlog" }), mockHandlers);
      expect(screen.queryByText("Start Ideation")).not.toBeInTheDocument();
    });

    it("hides Start Ideation for non-backlog tasks", () => {
      renderInContextMenu(createMockTask({ internalStatus: "ready" }), {
        ...mockHandlers,
        onStartIdeation: vi.fn(),
      });
      expect(screen.queryByText("Start Ideation")).not.toBeInTheDocument();
    });
  });

  describe("direct handler invocations (no confirmation)", () => {
    it("calls onViewDetails when View Details clicked", () => {
      renderInContextMenu(createMockTask(), mockHandlers);
      fireEvent.click(screen.getByText("View Details"));
      expect(mockHandlers.onViewDetails).toHaveBeenCalledTimes(1);
    });

    it("calls onEdit when Edit clicked", () => {
      renderInContextMenu(createMockTask({ internalStatus: "backlog" }), mockHandlers);
      fireEvent.click(screen.getByText("Edit"));
      expect(mockHandlers.onEdit).toHaveBeenCalledTimes(1);
    });

    it("calls onStartIdeation when Start Ideation clicked", () => {
      const onStartIdeation = vi.fn();
      renderInContextMenu(createMockTask({ internalStatus: "backlog" }), {
        ...mockHandlers,
        onStartIdeation,
      });
      fireEvent.click(screen.getByText("Start Ideation"));
      expect(onStartIdeation).toHaveBeenCalledTimes(1);
    });
  });

  describe("confirmation dialog flow", () => {
    it("shows confirmation when Archive clicked and calls onArchive on confirm", async () => {
      renderInContextMenu(createMockTask(), mockHandlers);
      fireEvent.click(screen.getByText("Archive"));

      await waitFor(() => {
        expect(screen.getByText("Archive this task?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByRole("button", { name: "Archive" }));

      await waitFor(() => {
        expect(mockHandlers.onArchive).toHaveBeenCalledTimes(1);
      });
    });

    it("shows confirmation when Restore clicked and calls onRestore on confirm", async () => {
      renderInContextMenu(createMockTask({ archivedAt: new Date().toISOString() }), mockHandlers);
      fireEvent.click(screen.getByText("Restore"));

      await waitFor(() => {
        expect(screen.getByText("Restore this task?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByRole("button", { name: "Restore" }));

      await waitFor(() => {
        expect(mockHandlers.onRestore).toHaveBeenCalledTimes(1);
      });
    });

    it("shows confirmation when Delete Permanently clicked and calls onPermanentDelete on confirm", async () => {
      renderInContextMenu(createMockTask({ archivedAt: new Date().toISOString() }), mockHandlers);
      fireEvent.click(screen.getByText("Delete Permanently"));

      await waitFor(() => {
        expect(screen.getByText("Delete permanently?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByRole("button", { name: "Delete" }));

      await waitFor(() => {
        expect(mockHandlers.onPermanentDelete).toHaveBeenCalledTimes(1);
      });
    });

    it("shows confirmation when Cancel status action clicked and calls onStatusChange", async () => {
      renderInContextMenu(createMockTask({ internalStatus: "backlog" }), mockHandlers);
      fireEvent.click(screen.getByText("Cancel"));

      await waitFor(() => {
        expect(screen.getByText("Cancel this task?")).toBeInTheDocument();
      });

      // The confirm button text is "Cancel" (the action label)
      // but there's also a dialog Cancel button — find the action button
      const buttons = screen.getAllByRole("button", { name: "Cancel" });
      const confirmButton = buttons.find((btn) =>
        btn.closest("[data-slot='alert-dialog-action']") || btn.hasAttribute("data-slot") && btn.getAttribute("data-slot") === "alert-dialog-action"
      ) ?? buttons[buttons.length - 1]; // Last Cancel button is likely the confirm action
      fireEvent.click(confirmButton);

      await waitFor(() => {
        expect(mockHandlers.onStatusChange).toHaveBeenCalledWith("cancelled");
      });
    });

    it("shows confirmation when Unblock clicked and calls onUnblock on confirm", async () => {
      renderInContextMenu(createMockTask({ internalStatus: "blocked" }), mockHandlers);
      fireEvent.click(screen.getByText("Unblock"));

      await waitFor(() => {
        expect(screen.getByText("Unblock this task?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByRole("button", { name: "Unblock" }));

      await waitFor(() => {
        expect(mockHandlers.onUnblock).toHaveBeenCalledTimes(1);
      });
    });

    it("shows confirmation when Re-open clicked and calls onStatusChange", async () => {
      renderInContextMenu(createMockTask({ internalStatus: "cancelled" }), mockHandlers);
      fireEvent.click(screen.getByText("Re-open"));

      await waitFor(() => {
        expect(screen.getByText("Re-open this task?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByRole("button", { name: "Re-open" }));

      await waitFor(() => {
        expect(mockHandlers.onStatusChange).toHaveBeenCalledWith("backlog");
      });
    });

    it("shows confirmation when Retry clicked and calls onStatusChange", async () => {
      renderInContextMenu(createMockTask({ internalStatus: "failed" }), mockHandlers);
      fireEvent.click(screen.getByText("Retry"));

      await waitFor(() => {
        expect(screen.getByText("Retry this task?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByRole("button", { name: "Retry" }));

      await waitFor(() => {
        expect(mockHandlers.onStatusChange).toHaveBeenCalledWith("backlog");
      });
    });

    it("does not call handler when confirmation is dismissed", async () => {
      renderInContextMenu(createMockTask(), mockHandlers);
      fireEvent.click(screen.getByText("Archive"));

      await waitFor(() => {
        expect(screen.getByText("Archive this task?")).toBeInTheDocument();
      });

      // Click the dialog's Cancel button (outline variant)
      const cancelButton = screen.getByRole("button", { name: /cancel/i });
      fireEvent.click(cancelButton);

      // Handler should NOT have been called
      expect(mockHandlers.onArchive).not.toHaveBeenCalled();
    });
  });

  describe("block dialog flow", () => {
    it("opens BlockReasonDialog when Block is clicked", async () => {
      renderInContextMenu(createMockTask({ internalStatus: "ready" }), mockHandlers);
      fireEvent.click(screen.getByText("Block"));

      await waitFor(() => {
        expect(screen.getByTestId("block-reason-dialog")).toBeInTheDocument();
      });
    });

    it("calls onBlockWithReason when block is confirmed", async () => {
      renderInContextMenu(createMockTask({ internalStatus: "ready" }), mockHandlers);
      fireEvent.click(screen.getByText("Block"));

      await waitFor(() => {
        expect(screen.getByTestId("block-reason-dialog")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByTestId("confirm-button"));

      await waitFor(() => {
        expect(mockHandlers.onBlockWithReason).toHaveBeenCalledTimes(1);
      });
    });

    it("calls onBlockWithReason with reason when provided", async () => {
      renderInContextMenu(createMockTask({ internalStatus: "ready" }), mockHandlers);
      fireEvent.click(screen.getByText("Block"));

      await waitFor(() => {
        expect(screen.getByTestId("block-reason-dialog")).toBeInTheDocument();
      });

      const textarea = screen.getByTestId("block-reason-input");
      fireEvent.change(textarea, { target: { value: "Waiting for API" } });
      fireEvent.click(screen.getByTestId("confirm-button"));

      await waitFor(() => {
        expect(mockHandlers.onBlockWithReason).toHaveBeenCalledWith("Waiting for API");
      });
    });
  });
});
