/**
 * StatusDropdown component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StatusDropdown } from "./StatusDropdown";
import { api } from "@/lib/tauri";
import type { StatusTransition } from "@/types/task";

// Mock the tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      getValidTransitions: vi.fn(),
    },
  },
}));

// Helper to wrap component with QueryClient
function renderWithQueryClient(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
  );
}

describe("StatusDropdown", () => {
  const mockOnTransition = vi.fn();
  const mockTransitions: StatusTransition[] = [
    { status: "ready", label: "Ready for Work" },
    { status: "cancelled", label: "Cancel" },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders loading state while fetching transitions", () => {
    vi.mocked(api.tasks.getValidTransitions).mockImplementation(
      () => new Promise(() => {}) // Never resolves
    );

    renderWithQueryClient(
      <StatusDropdown
        taskId="task-1"
        currentStatus="backlog"
        onTransition={mockOnTransition}
      />
    );

    expect(screen.getByRole("button")).toBeDisabled();
    expect(screen.getByRole("button").querySelector("svg")).toHaveClass(
      "animate-spin"
    );
  });

  it("renders error state when fetch fails", async () => {
    vi.mocked(api.tasks.getValidTransitions).mockRejectedValue(
      new Error("Failed to fetch")
    );

    renderWithQueryClient(
      <StatusDropdown
        taskId="task-1"
        currentStatus="backlog"
        onTransition={mockOnTransition}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Error")).toBeInTheDocument();
    });
  });

  it("renders read-only badge when no transitions available", async () => {
    vi.mocked(api.tasks.getValidTransitions).mockResolvedValue([]);

    renderWithQueryClient(
      <StatusDropdown
        taskId="task-1"
        currentStatus="executing"
        onTransition={mockOnTransition}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Executing")).toBeInTheDocument();
    });

    // Should not be a button (read-only)
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders dropdown with valid transitions", async () => {
    vi.mocked(api.tasks.getValidTransitions).mockResolvedValue(
      mockTransitions
    );

    renderWithQueryClient(
      <StatusDropdown
        taskId="task-1"
        currentStatus="backlog"
        onTransition={mockOnTransition}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Backlog")).toBeInTheDocument();
    });

    expect(screen.getByRole("button")).not.toBeDisabled();
  });

  it("shows transition options when dropdown is opened", async () => {
    vi.mocked(api.tasks.getValidTransitions).mockResolvedValue(
      mockTransitions
    );

    const user = userEvent.setup();

    renderWithQueryClient(
      <StatusDropdown
        taskId="task-1"
        currentStatus="backlog"
        onTransition={mockOnTransition}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Backlog")).toBeInTheDocument();
    });

    // Click to open dropdown
    await user.click(screen.getByRole("button"));

    // Wait for menu items to appear
    await waitFor(() => {
      expect(screen.getByText("Ready for Work")).toBeInTheDocument();
      expect(screen.getByText("Cancel")).toBeInTheDocument();
    });
  });

  it("calls onTransition when a transition is selected", async () => {
    vi.mocked(api.tasks.getValidTransitions).mockResolvedValue(
      mockTransitions
    );

    const user = userEvent.setup();

    renderWithQueryClient(
      <StatusDropdown
        taskId="task-1"
        currentStatus="backlog"
        onTransition={mockOnTransition}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Backlog")).toBeInTheDocument();
    });

    // Open dropdown
    await user.click(screen.getByRole("button"));

    // Click on "Ready for Work" option
    await user.click(screen.getByText("Ready for Work"));

    // Should call onTransition with correct status
    expect(mockOnTransition).toHaveBeenCalledWith("ready");
  });

  it("disables dropdown when disabled prop is true", async () => {
    vi.mocked(api.tasks.getValidTransitions).mockResolvedValue(
      mockTransitions
    );

    renderWithQueryClient(
      <StatusDropdown
        taskId="task-1"
        currentStatus="backlog"
        onTransition={mockOnTransition}
        disabled={true}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Backlog")).toBeInTheDocument();
    });

    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("displays correct status label and color for different statuses", async () => {
    vi.mocked(api.tasks.getValidTransitions).mockResolvedValue([]);

    const { rerender } = renderWithQueryClient(
      <StatusDropdown
        taskId="task-1"
        currentStatus="approved"
        onTransition={mockOnTransition}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Approved")).toBeInTheDocument();
    });

    // Rerender with different status
    rerender(
      <QueryClientProvider
        client={
          new QueryClient({
            defaultOptions: {
              queries: {
                retry: false,
              },
            },
          })
        }
      >
        <StatusDropdown
          taskId="task-2"
          currentStatus="blocked"
          onTransition={mockOnTransition}
        />
      </QueryClientProvider>
    );

    await waitFor(() => {
      expect(screen.getByText("Blocked")).toBeInTheDocument();
    });
  });

  it("uses correct query key for caching", async () => {
    vi.mocked(api.tasks.getValidTransitions).mockResolvedValue(
      mockTransitions
    );

    renderWithQueryClient(
      <StatusDropdown
        taskId="task-123"
        currentStatus="backlog"
        onTransition={mockOnTransition}
      />
    );

    await waitFor(() => {
      expect(api.tasks.getValidTransitions).toHaveBeenCalledWith("task-123");
    });
  });
});
