/**
 * GroupContextMenuItems.test.tsx - Tests for group-level bulk action context menu items
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { GroupContextMenuItems } from "./GroupContextMenuItems";
import { useConfirmation } from "@/hooks/useConfirmation";

// ============================================================================
// Helpers
// ============================================================================

function TestWrapper({
  groupLabel,
  groupKind,
  taskCount,
  projectId = "project-1",
  groupId = "ready",
  onArchiveAll,
}: {
  groupLabel: string;
  groupKind: "column" | "plan" | "uncategorized";
  taskCount: number;
  projectId?: string;
  groupId?: string;
  onArchiveAll?: () => void;
}) {
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <div data-testid="trigger">Trigger</div>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <GroupContextMenuItems
            groupLabel={groupLabel}
            groupKind={groupKind}
            taskCount={taskCount}
            projectId={projectId}
            groupId={groupId}
            onArchiveAll={onArchiveAll}

            confirm={confirm}
          />
        </ContextMenuContent>
      </ContextMenu>
      <ConfirmationDialog {...confirmationDialogProps} />
    </>
  );
}

function openContextMenu() {
  fireEvent.contextMenu(screen.getByTestId("trigger"));
}

// ============================================================================
// Tests
// ============================================================================

describe("GroupContextMenuItems", () => {
  let onArchiveAll: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onArchiveAll = vi.fn();
  });

  describe("rendering", () => {
    it("renders 'Archive all Ready' for column kind", () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
          onArchiveAll={onArchiveAll}
        />,
      );
      openContextMenu();
      expect(screen.getByText("Archive all Ready")).toBeInTheDocument();
    });

    it("renders 'Archive all in [Plan]' for plan kind", () => {
      render(
        <TestWrapper
          groupLabel="Auth Feature"
          groupKind="plan"
          taskCount={5}
          groupId="session-abc"
          onArchiveAll={onArchiveAll}
        />,
      );
      openContextMenu();
      expect(screen.getByText("Archive all in Auth Feature")).toBeInTheDocument();
    });

    it("renders 'Archive all Uncategorized' for uncategorized kind", () => {
      render(
        <TestWrapper
          groupLabel=""
          groupKind="uncategorized"
          taskCount={2}
          onArchiveAll={onArchiveAll}
        />,
      );
      openContextMenu();
      expect(screen.getByText("Archive all Uncategorized")).toBeInTheDocument();
    });

    it("renders nothing when taskCount is 0", () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={0}
          onArchiveAll={onArchiveAll}
        />,
      );
      openContextMenu();
      expect(screen.queryByText(/Archive all/)).not.toBeInTheDocument();
    });

    it("has data-testid for archive-all action", () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
          onArchiveAll={onArchiveAll}
        />,
      );
      openContextMenu();
      expect(screen.getByTestId("archive-all-action")).toBeInTheDocument();
    });

    it("renders nothing when no handlers provided", () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
        />,
      );
      openContextMenu();
      expect(screen.queryByText(/Archive all/)).not.toBeInTheDocument();
    });
  });

  describe("confirmation flow", () => {
    it("shows confirmation dialog when archive-all clicked", async () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
          onArchiveAll={onArchiveAll}
        />,
      );
      openContextMenu();
      fireEvent.click(screen.getByText("Archive all Ready"));

      await waitFor(() => {
        expect(screen.getByText("Archive all Ready?")).toBeInTheDocument();
      });
      expect(screen.getByText(/3 tasks/)).toBeInTheDocument();
    });

    it("calls onArchiveAll when confirmed", async () => {
      render(
        <TestWrapper
          groupLabel="Blocked"
          groupKind="column"
          taskCount={2}
          groupId="blocked"
          onArchiveAll={onArchiveAll}
        />,
      );
      openContextMenu();
      fireEvent.click(screen.getByText("Archive all Blocked"));

      await waitFor(() => {
        expect(screen.getByText("Archive all Blocked?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Archive"));
      await waitFor(() => {
        expect(onArchiveAll).toHaveBeenCalledTimes(1);
      });
    });

    it("does not call onArchiveAll when cancelled", async () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
          onArchiveAll={onArchiveAll}
        />,
      );
      openContextMenu();
      fireEvent.click(screen.getByText("Archive all Ready"));

      await waitFor(() => {
        expect(screen.getByText("Archive all Ready?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Cancel"));
      await waitFor(() => {
        expect(screen.queryByText("Archive all Ready?")).not.toBeInTheDocument();
      });
      expect(onArchiveAll).not.toHaveBeenCalled();
    });
  });
});
