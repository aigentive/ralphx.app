/**
 * Tests for Column component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { DndContext } from "@dnd-kit/core";
import { createMockTask } from "@/test/mock-data";
import { Column } from "./Column";
import type { WorkflowColumnResponse } from "@/lib/api/workflows";
import type { InfiniteData } from "@tanstack/react-query";
import type { TaskListResponse } from "@/types/task";

// Mock IntersectionObserver
class MockIntersectionObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  constructor() {}
}
window.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

// Mock useInfiniteTasksQuery - keep flattenPages implementation, only mock the hook
vi.mock("@/hooks/useInfiniteTasksQuery", async (importOriginal) => {
  const actual = await importOriginal() as Record<string, unknown>;
  return {
    ...actual,
    useInfiniteTasksQuery: vi.fn(),
  };
});

import { useInfiniteTasksQuery } from "@/hooks/useInfiniteTasksQuery";

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
}

function DndWrapper({ children }: { children: React.ReactNode }) {
  const queryClient = createTestQueryClient();
  return (
    <QueryClientProvider client={queryClient}>
      <DndContext>{children}</DndContext>
    </QueryClientProvider>
  );
}

const createMockColumn = (overrides: Partial<WorkflowColumnResponse> = {}): WorkflowColumnResponse => ({
  id: "backlog",
  name: "Backlog",
  mapsTo: "backlog",
  ...overrides,
});

describe("Column", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default mock implementation
    vi.mocked(useInfiniteTasksQuery).mockReturnValue({
      data: { pages: [{ tasks: [], total: 0, hasMore: false, offset: 0 }], pageParams: [undefined] } as InfiniteData<TaskListResponse>,
      fetchNextPage: vi.fn(),
      hasNextPage: false,
      isFetchingNextPage: false,
      isLoading: false,
      isError: false,
      error: null,
    } as unknown as ReturnType<typeof useInfiniteTasksQuery>);
  });

  describe("rendering", () => {
    it("should render with data-testid", () => {
      const column = createMockColumn({ id: "my-column" });
      render(<Column column={column} projectId="p1" showArchived={false} />, { wrapper: DndWrapper });
      expect(screen.getByTestId("column-my-column")).toBeInTheDocument();
    });

    it("should render column name in header", () => {
      const column = createMockColumn({ name: "In Progress" });
      render(<Column column={column} projectId="p1" showArchived={false} />, { wrapper: DndWrapper });
      expect(screen.getByText("In Progress")).toBeInTheDocument();
    });

    it("should render task count in header", () => {
      const tasks = [createMockTask(), createMockTask(), createMockTask()];
      vi.mocked(useInfiniteTasksQuery).mockReturnValue({
        data: { pages: [{ tasks, total: 3, hasMore: false, offset: 0 }], pageParams: [undefined] } as InfiniteData<TaskListResponse>,
        fetchNextPage: vi.fn(),
        hasNextPage: false,
        isFetchingNextPage: false,
        isLoading: false,
        isError: false,
        error: null,
      } as unknown as ReturnType<typeof useInfiniteTasksQuery>);

      const column = createMockColumn();
      render(<Column column={column} projectId="p1" showArchived={false} />, { wrapper: DndWrapper });
      expect(screen.getByText("3")).toBeInTheDocument();
    });

    it("should render tasks", () => {
      const tasks = [
        createMockTask({ id: "t1", title: "Task One" }),
        createMockTask({ id: "t2", title: "Task Two" }),
      ];
      vi.mocked(useInfiniteTasksQuery).mockReturnValue({
        data: { pages: [{ tasks, total: 2, hasMore: false, offset: 0 }], pageParams: [undefined] } as InfiniteData<TaskListResponse>,
        fetchNextPage: vi.fn(),
        hasNextPage: false,
        isFetchingNextPage: false,
        isLoading: false,
        isError: false,
        error: null,
      } as unknown as ReturnType<typeof useInfiniteTasksQuery>);

      const column = createMockColumn();
      render(<Column column={column} projectId="p1" showArchived={false} />, { wrapper: DndWrapper });
      expect(screen.getByText("Task One")).toBeInTheDocument();
      expect(screen.getByText("Task Two")).toBeInTheDocument();
    });

    it("should render empty state when no tasks", () => {
      const column = createMockColumn();
      render(<Column column={column} projectId="p1" showArchived={false} />, { wrapper: DndWrapper });
      const columnEl = screen.getByTestId(`column-${column.id}`);
      expect(columnEl).toBeInTheDocument();
    });
  });

  describe("droppable behavior", () => {
    it("should be a droppable zone", () => {
      const column = createMockColumn();
      render(<Column column={column} projectId="p1" showArchived={false} />, { wrapper: DndWrapper });
      const dropZone = screen.getByTestId(`drop-zone-${column.id}`);
      expect(dropZone).toBeInTheDocument();
    });

    it("should apply isOver styling when isOver is true", () => {
      const column = createMockColumn();
      render(<Column column={column} projectId="p1" showArchived={false} isOver />, { wrapper: DndWrapper });
      const dropZone = screen.getByTestId(`drop-zone-${column.id}`);
      // The drop zone gets dashed orange border when hovering
      expect(dropZone.style.border).toContain("dashed");
    });

    it("should not apply isOver styling when isOver is false", () => {
      const column = createMockColumn();
      render(<Column column={column} projectId="p1" showArchived={false} isOver={false} />, { wrapper: DndWrapper });
      const dropZone = screen.getByTestId(`drop-zone-${column.id}`);
      // Drop zone should have transparent border when not over
      expect(dropZone.style.border).toContain("transparent");
    });
  });

  describe("locked columns", () => {
    it("should show invalid drop icon for In Progress column when isOver and isInvalid", () => {
      const column = createMockColumn({ id: "in_progress", name: "In Progress" });
      render(<Column column={column} projectId="p1" showArchived={false} isOver isInvalid />, { wrapper: DndWrapper });
      expect(screen.getByTestId("invalid-drop-icon")).toBeInTheDocument();
    });

    it("should show invalid drop icon for In Review column when isOver and isInvalid", () => {
      const column = createMockColumn({ id: "in_review", name: "In Review" });
      render(<Column column={column} projectId="p1" showArchived={false} isOver isInvalid />, { wrapper: DndWrapper });
      expect(screen.getByTestId("invalid-drop-icon")).toBeInTheDocument();
    });

    it("should show invalid drop icon for Done column when isOver and isInvalid", () => {
      const column = createMockColumn({ id: "done", name: "Done" });
      render(<Column column={column} projectId="p1" showArchived={false} isOver isInvalid />, { wrapper: DndWrapper });
      expect(screen.getByTestId("invalid-drop-icon")).toBeInTheDocument();
    });

    it("should apply error border when isOver and isInvalid", () => {
      const column = createMockColumn({ id: "in_progress" });
      render(<Column column={column} projectId="p1" showArchived={false} isOver isInvalid />, { wrapper: DndWrapper });
      const dropZone = screen.getByTestId(`drop-zone-${column.id}`);
      // Should have red dashed border for invalid drop
      expect(dropZone.style.border).toContain("dashed");
    });
  });

  describe("loading state", () => {
    it("should show skeleton cards when loading", () => {
      vi.mocked(useInfiniteTasksQuery).mockReturnValue({
        data: undefined,
        fetchNextPage: vi.fn(),
        hasNextPage: false,
        isFetchingNextPage: false,
        isLoading: true,
        isError: false,
        error: null,
      } as unknown as ReturnType<typeof useInfiniteTasksQuery>);

      const column = createMockColumn();
      const { container } = render(<Column column={column} projectId="p1" showArchived={false} />, { wrapper: DndWrapper });

      // Skeleton cards have the skeleton component class
      // The Skeleton component from shadcn renders with animate-shimmer class
      expect(screen.getByTestId(`column-${column.id}`)).toBeInTheDocument();
      // Verify skeletons are rendered (they use the Skeleton component class)
      const skeletons = container.querySelectorAll(".rounded-lg");
      expect(skeletons.length).toBeGreaterThan(0);
    });
  });
});
