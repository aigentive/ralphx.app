/**
 * TaskCardContextMenu.test.tsx - Tests for TaskCardContextMenu component
 *
 * Tests the Kanban-specific wrapper and its delegation to TaskContextMenuItems.
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TaskCardContextMenu } from "./TaskCardContextMenu";
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

describe("TaskCardContextMenu", () => {
  const mockHandlers: Record<string, ReturnType<typeof vi.fn>> = {
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

  it("renders trigger children", () => {
    const task = createMockTask();
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="child">Child Content</div>
      </TaskCardContextMenu>
    );

    expect(screen.getByTestId("child")).toBeInTheDocument();
  });

  it("shows View Details for all tasks", () => {
    const task = createMockTask();
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("View Details")).toBeInTheDocument();
  });

  it("shows Edit for non-archived, non-system-controlled tasks", () => {
    const task = createMockTask({ internalStatus: "backlog" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Edit")).toBeInTheDocument();
  });

  it("hides Edit for system-controlled tasks", () => {
    const task = createMockTask({ internalStatus: "executing" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.queryByText("Edit")).not.toBeInTheDocument();
  });

  it("hides Edit for archived tasks", () => {
    const task = createMockTask({ archivedAt: new Date().toISOString() });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.queryByText("Edit")).not.toBeInTheDocument();
  });

  it("shows Archive for non-archived tasks", () => {
    const task = createMockTask({ internalStatus: "backlog" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Archive")).toBeInTheDocument();
  });

  it("shows Restore and Delete Permanently for archived tasks", () => {
    const task = createMockTask({ archivedAt: new Date().toISOString() });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Restore")).toBeInTheDocument();
    expect(screen.getByText("Delete Permanently")).toBeInTheDocument();
    expect(screen.queryByText("Archive")).not.toBeInTheDocument();
  });

  it("shows Cancel for backlog tasks", () => {
    const task = createMockTask({ internalStatus: "backlog" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Cancel")).toBeInTheDocument();
  });

  it("shows Block and Cancel for ready tasks", () => {
    const task = createMockTask({ internalStatus: "ready" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Block")).toBeInTheDocument();
    expect(screen.getByText("Cancel")).toBeInTheDocument();
  });

  it("shows Unblock and Cancel for blocked tasks", () => {
    const task = createMockTask({ internalStatus: "blocked" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Unblock")).toBeInTheDocument();
    expect(screen.getByText("Cancel")).toBeInTheDocument();
  });

  it("shows Re-open for approved tasks", () => {
    const task = createMockTask({ internalStatus: "approved" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Re-open")).toBeInTheDocument();
  });

  it("shows Retry for failed tasks", () => {
    const task = createMockTask({ internalStatus: "failed" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Retry")).toBeInTheDocument();
  });

  it("calls onViewDetails when View Details is clicked", () => {
    const task = createMockTask();
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));
    fireEvent.click(screen.getByText("View Details"));

    expect(mockHandlers.onViewDetails).toHaveBeenCalledTimes(1);
  });

  it("calls onEdit when Edit is clicked", () => {
    const task = createMockTask({ internalStatus: "backlog" });
    render(
      <TaskCardContextMenu task={task} {...mockHandlers}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));
    fireEvent.click(screen.getByText("Edit"));

    expect(mockHandlers.onEdit).toHaveBeenCalledTimes(1);
  });

  it("shows Start Ideation for backlog tasks when handler provided", () => {
    const task = createMockTask({ internalStatus: "backlog" });
    const onStartIdeation = vi.fn();
    render(
      <TaskCardContextMenu task={task} {...mockHandlers} onStartIdeation={onStartIdeation}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.getByText("Start Ideation")).toBeInTheDocument();
  });

  it("hides Start Ideation for non-backlog tasks", () => {
    const task = createMockTask({ internalStatus: "ready" });
    const onStartIdeation = vi.fn();
    render(
      <TaskCardContextMenu task={task} {...mockHandlers} onStartIdeation={onStartIdeation}>
        <div data-testid="trigger">Trigger</div>
      </TaskCardContextMenu>
    );

    fireEvent.contextMenu(screen.getByTestId("trigger"));

    expect(screen.queryByText("Start Ideation")).not.toBeInTheDocument();
  });
});
