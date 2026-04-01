import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ActivityView } from "./ActivityView";
import { useActivityStore } from "@/stores/activityStore";
import type { AgentMessageEvent } from "@/types/events";

vi.mock("react-intersection-observer", () => ({
  useInView: () => ({ ref: vi.fn(), inView: false }),
}));

vi.mock("@/hooks/useActivityEvents", () => ({
  useTaskActivityEvents: vi.fn(() => ({
    data: undefined,
    hasNextPage: false,
    isFetchingNextPage: false,
    fetchNextPage: vi.fn(),
  })),
  useSessionActivityEvents: vi.fn(() => ({
    data: undefined,
    hasNextPage: false,
    isFetchingNextPage: false,
    fetchNextPage: vi.fn(),
  })),
  useAllActivityEvents: vi.fn(() => ({
    data: undefined,
    hasNextPage: false,
    isFetchingNextPage: false,
    fetchNextPage: vi.fn(),
  })),
  flattenActivityPages: vi.fn(() => []),
}));

const createMessage = (overrides: Partial<AgentMessageEvent> = {}): AgentMessageEvent => ({
  taskId: "task-1",
  type: "thinking",
  content: "Analyzing the codebase...",
  timestamp: Date.now(),
  ...overrides,
});

describe("ActivityView", () => {
  const renderWithQueryClient = (ui: React.ReactElement) => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
    return render(
      <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
    );
  };

  beforeEach(() => {
    vi.clearAllMocks();
    useActivityStore.setState({ messages: [], alerts: [], lastEventTime: null });
  });

  it("renders the activity view container", () => {
    renderWithQueryClient(<ActivityView />);
    expect(screen.getByTestId("activity-view")).toBeInTheDocument();
  });

  it("shows and hides header based on showHeader", () => {
    const { rerender } = renderWithQueryClient(<ActivityView />);
    expect(screen.getByText("Activity")).toBeInTheDocument();

    rerender(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <ActivityView showHeader={false} />
      </QueryClientProvider>
    );
    expect(screen.queryByText("Activity")).not.toBeInTheDocument();
  });

  it("shows empty state when there are no messages", () => {
    renderWithQueryClient(<ActivityView />);
    expect(screen.getByTestId("activity-empty")).toBeInTheDocument();
  });

  it("renders messages from the activity store", async () => {
    useActivityStore.setState({
      messages: [
        createMessage({ content: "First message", timestamp: 1000 }),
        createMessage({ content: "Second message", timestamp: 2000 }),
      ],
    });

    renderWithQueryClient(<ActivityView />);

    await waitFor(() => {
      expect(screen.getByText("First message")).toBeInTheDocument();
      expect(screen.getByText("Second message")).toBeInTheDocument();
    });
  });

  it("filters messages by search query", async () => {
    useActivityStore.setState({
      messages: [
        createMessage({ content: "Reading file.ts" }),
        createMessage({ content: "Writing output" }),
        createMessage({ content: "Another read operation" }),
      ],
    });

    renderWithQueryClient(<ActivityView />);

    const searchInput = screen.getByTestId("activity-search");
    fireEvent.change(searchInput, { target: { value: "read" } });

    await waitFor(() => {
      const visibleMessages = screen.getAllByTestId("activity-message");
      expect(visibleMessages).toHaveLength(2);
    });
  });
});
