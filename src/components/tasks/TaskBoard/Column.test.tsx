/**
 * Tests for Column component
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DndContext } from "@dnd-kit/core";
import { createMockTask } from "@/test/mock-data";
import { Column } from "./Column";
import type { BoardColumn } from "./hooks";

function DndWrapper({ children }: { children: React.ReactNode }) {
  return <DndContext>{children}</DndContext>;
}

const createMockColumn = (overrides: Partial<BoardColumn> = {}): BoardColumn => ({
  id: "backlog",
  name: "Backlog",
  mapsTo: "backlog",
  tasks: [],
  ...overrides,
});

describe("Column", () => {
  describe("rendering", () => {
    it("should render with data-testid", () => {
      const column = createMockColumn({ id: "my-column" });
      render(<Column column={column} />, { wrapper: DndWrapper });
      expect(screen.getByTestId("column-my-column")).toBeInTheDocument();
    });

    it("should render column name in header", () => {
      const column = createMockColumn({ name: "In Progress" });
      render(<Column column={column} />, { wrapper: DndWrapper });
      expect(screen.getByText("In Progress")).toBeInTheDocument();
    });

    it("should render task count in header", () => {
      const column = createMockColumn({
        tasks: [createMockTask(), createMockTask(), createMockTask()],
      });
      render(<Column column={column} />, { wrapper: DndWrapper });
      expect(screen.getByText("3")).toBeInTheDocument();
    });

    it("should render tasks", () => {
      const tasks = [
        createMockTask({ id: "t1", title: "Task One" }),
        createMockTask({ id: "t2", title: "Task Two" }),
      ];
      const column = createMockColumn({ tasks });
      render(<Column column={column} />, { wrapper: DndWrapper });
      expect(screen.getByText("Task One")).toBeInTheDocument();
      expect(screen.getByText("Task Two")).toBeInTheDocument();
    });

    it("should render empty state when no tasks", () => {
      const column = createMockColumn({ tasks: [] });
      render(<Column column={column} />, { wrapper: DndWrapper });
      const columnEl = screen.getByTestId(`column-${column.id}`);
      expect(columnEl).toBeInTheDocument();
    });
  });

  describe("droppable behavior", () => {
    it("should be a droppable zone", () => {
      const column = createMockColumn();
      render(<Column column={column} />, { wrapper: DndWrapper });
      const columnEl = screen.getByTestId(`column-${column.id}`);
      expect(columnEl).toBeInTheDocument();
    });

    it("should apply isOver styling when isOver is true", () => {
      const column = createMockColumn();
      render(<Column column={column} isOver />, { wrapper: DndWrapper });
      const columnEl = screen.getByTestId(`column-${column.id}`);
      expect(columnEl.style.borderColor).toBe("var(--accent-primary)");
    });

    it("should not apply isOver styling when isOver is false", () => {
      const column = createMockColumn();
      render(<Column column={column} isOver={false} />, { wrapper: DndWrapper });
      const columnEl = screen.getByTestId(`column-${column.id}`);
      expect(columnEl.style.borderColor).not.toBe("var(--accent-primary)");
    });
  });

  describe("locked columns", () => {
    it("should show invalid drop icon for In Progress column when isOver and isInvalid", () => {
      const column = createMockColumn({ id: "in_progress", name: "In Progress" });
      render(<Column column={column} isOver isInvalid />, { wrapper: DndWrapper });
      expect(screen.getByTestId("invalid-drop-icon")).toBeInTheDocument();
    });

    it("should show invalid drop icon for In Review column when isOver and isInvalid", () => {
      const column = createMockColumn({ id: "in_review", name: "In Review" });
      render(<Column column={column} isOver isInvalid />, { wrapper: DndWrapper });
      expect(screen.getByTestId("invalid-drop-icon")).toBeInTheDocument();
    });

    it("should show invalid drop icon for Done column when isOver and isInvalid", () => {
      const column = createMockColumn({ id: "done", name: "Done" });
      render(<Column column={column} isOver isInvalid />, { wrapper: DndWrapper });
      expect(screen.getByTestId("invalid-drop-icon")).toBeInTheDocument();
    });

    it("should apply error border when isOver and isInvalid", () => {
      const column = createMockColumn();
      render(<Column column={column} isOver isInvalid />, { wrapper: DndWrapper });
      const columnEl = screen.getByTestId(`column-${column.id}`);
      expect(columnEl.style.borderColor).toBe("var(--status-error)");
    });
  });
});
