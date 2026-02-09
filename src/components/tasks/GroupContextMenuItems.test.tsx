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
  onRemoveAll,
}: {
  groupLabel: string;
  groupKind: "column" | "plan" | "uncategorized";
  taskCount: number;
  projectId?: string;
  groupId?: string;
  onRemoveAll: () => void;
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
            onRemoveAll={onRemoveAll}
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
  let onRemoveAll: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onRemoveAll = vi.fn();
  });

  describe("rendering", () => {
    it("renders 'Remove all Ready' for column kind", () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
          onRemoveAll={onRemoveAll}
        />,
      );
      openContextMenu();
      expect(screen.getByText("Remove all Ready")).toBeInTheDocument();
    });

    it("renders 'Remove all from [Plan]' for plan kind", () => {
      render(
        <TestWrapper
          groupLabel="Auth Feature"
          groupKind="plan"
          taskCount={5}
          groupId="session-abc"
          onRemoveAll={onRemoveAll}
        />,
      );
      openContextMenu();
      expect(screen.getByText("Remove all from Auth Feature")).toBeInTheDocument();
    });

    it("renders 'Remove all Uncategorized' for uncategorized kind", () => {
      render(
        <TestWrapper
          groupLabel=""
          groupKind="uncategorized"
          taskCount={2}
          onRemoveAll={onRemoveAll}
        />,
      );
      openContextMenu();
      expect(screen.getByText("Remove all Uncategorized")).toBeInTheDocument();
    });

    it("renders nothing when taskCount is 0", () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={0}
          onRemoveAll={onRemoveAll}
        />,
      );
      openContextMenu();
      expect(screen.queryByText(/Remove all/)).not.toBeInTheDocument();
    });

    it("has data-testid for remove-all action", () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
          onRemoveAll={onRemoveAll}
        />,
      );
      openContextMenu();
      expect(screen.getByTestId("remove-all-action")).toBeInTheDocument();
    });
  });

  describe("confirmation flow", () => {
    it("shows confirmation dialog when clicked", async () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
          onRemoveAll={onRemoveAll}
        />,
      );
      openContextMenu();
      fireEvent.click(screen.getByText("Remove all Ready"));

      await waitFor(() => {
        expect(screen.getByText("Remove all Ready?")).toBeInTheDocument();
      });
      expect(screen.getByText(/3 tasks/)).toBeInTheDocument();
    });

    it("calls onRemoveAll when confirmed", async () => {
      render(
        <TestWrapper
          groupLabel="Blocked"
          groupKind="column"
          taskCount={2}
          groupId="blocked"
          onRemoveAll={onRemoveAll}
        />,
      );
      openContextMenu();
      fireEvent.click(screen.getByText("Remove all Blocked"));

      await waitFor(() => {
        expect(screen.getByText("Remove all Blocked?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Remove"));
      await waitFor(() => {
        expect(onRemoveAll).toHaveBeenCalledTimes(1);
      });
    });

    it("does not call onRemoveAll when cancelled", async () => {
      render(
        <TestWrapper
          groupLabel="Ready"
          groupKind="column"
          taskCount={3}
          onRemoveAll={onRemoveAll}
        />,
      );
      openContextMenu();
      fireEvent.click(screen.getByText("Remove all Ready"));

      await waitFor(() => {
        expect(screen.getByText("Remove all Ready?")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Cancel"));
      await waitFor(() => {
        expect(screen.queryByText("Remove all Ready?")).not.toBeInTheDocument();
      });
      expect(onRemoveAll).not.toHaveBeenCalled();
    });
  });
});
