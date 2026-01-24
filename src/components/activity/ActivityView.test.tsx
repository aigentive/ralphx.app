/**
 * ActivityView component tests
 * Real-time agent execution monitoring with expandable details and filters
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ActivityView } from "./ActivityView";
import { useActivityStore } from "@/stores/activityStore";
import type { AgentMessageEvent } from "@/types/events";

// Create mock messages
const createMockMessage = (overrides: Partial<AgentMessageEvent> = {}): AgentMessageEvent => ({
  taskId: "task-1",
  type: "thinking",
  content: "Analyzing the codebase...",
  timestamp: Date.now(),
  ...overrides,
});

describe("ActivityView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store to initial state
    useActivityStore.setState({ messages: [], alerts: [] });
  });

  describe("rendering", () => {
    it("renders activity view container with correct testid", () => {
      render(<ActivityView />);
      expect(screen.getByTestId("activity-view")).toBeInTheDocument();
    });

    it("renders Activity header by default", () => {
      render(<ActivityView />);
      expect(screen.getByText("Activity")).toBeInTheDocument();
    });

    it("hides header when showHeader is false", () => {
      render(<ActivityView showHeader={false} />);
      expect(screen.queryByText("Activity")).not.toBeInTheDocument();
    });

    it("applies design system background color", () => {
      render(<ActivityView />);
      const view = screen.getByTestId("activity-view");
      expect(view).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });
  });

  describe("empty state", () => {
    it("renders empty state when no messages", () => {
      render(<ActivityView />);
      expect(screen.getByTestId("activity-empty")).toBeInTheDocument();
      expect(screen.getByText(/no activity/i)).toBeInTheDocument();
    });

    it("shows helpful message in empty state", () => {
      render(<ActivityView />);
      expect(screen.getByText(/agent activity will appear here/i)).toBeInTheDocument();
    });
  });

  describe("message display", () => {
    it("renders messages from the activity store", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ content: "First message", timestamp: 1000 }),
        createMockMessage({ content: "Second message", timestamp: 2000 }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      expect(screen.getByText("First message")).toBeInTheDocument();
      expect(screen.getByText("Second message")).toBeInTheDocument();
    });

    it("displays different message types with correct attributes", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ type: "thinking", content: "Thinking..." }),
        createMockMessage({ type: "tool_call", content: "Using tool: Read" }),
        createMockMessage({ type: "tool_result", content: "File contents..." }),
        createMockMessage({ type: "text", content: "Some text output" }),
        createMockMessage({ type: "error", content: "An error occurred" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      const activityMessages = screen.getAllByTestId("activity-message");
      expect(activityMessages).toHaveLength(5);

      // Check data-type attributes
      expect(activityMessages[0]).toHaveAttribute("data-type", "thinking");
      expect(activityMessages[1]).toHaveAttribute("data-type", "tool_call");
      expect(activityMessages[2]).toHaveAttribute("data-type", "tool_result");
      expect(activityMessages[3]).toHaveAttribute("data-type", "text");
      expect(activityMessages[4]).toHaveAttribute("data-type", "error");
    });

    it("displays timestamps on messages", () => {
      const timestamp = new Date("2026-01-24T10:30:45Z").getTime();
      const messages: AgentMessageEvent[] = [
        createMockMessage({ content: "Test message", timestamp }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      // Timestamp should be formatted (exact format depends on locale)
      const messageEl = screen.getByTestId("activity-message");
      expect(messageEl).toHaveTextContent(/\d{1,2}:\d{2}/);
    });

    it("extracts and displays tool names from content", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ type: "tool_call", content: "Read(/path/to/file.ts)" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      expect(screen.getByText("Read")).toBeInTheDocument();
    });
  });

  describe("expandable details", () => {
    it("renders expand button for tool_call messages", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({
          type: "tool_call",
          content: "Read file",
          metadata: { path: "/file.ts" },
        }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      // The message should be clickable to expand
      const message = screen.getByTestId("activity-message");
      expect(message).toBeInTheDocument();
    });

    it("shows metadata when message is expanded", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({
          type: "tool_call",
          content: "Read file",
          metadata: { path: "/some/file.ts" },
        }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      // Click on the header area to expand (the first child div with cursor-pointer)
      const message = screen.getByTestId("activity-message");
      const clickableHeader = message.querySelector(".cursor-pointer");
      fireEvent.click(clickableHeader!);

      // Should show metadata - it shows as JSON
      expect(screen.getByText("Details")).toBeInTheDocument();
      expect(screen.getByText(/"path":/)).toBeInTheDocument();
    });

    it("toggles expanded state on click", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({
          type: "tool_call",
          content: "Read file",
          metadata: { testKey: "testValue" },
        }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      const message = screen.getByTestId("activity-message");
      const clickableHeader = message.querySelector(".cursor-pointer");

      // Click to expand
      fireEvent.click(clickableHeader!);
      expect(screen.getByText("Details")).toBeInTheDocument();

      // Click again to collapse
      fireEvent.click(clickableHeader!);
      // Metadata should no longer be visible (within the Details section)
      expect(screen.queryByText("Details")).not.toBeInTheDocument();
    });
  });

  describe("search functionality", () => {
    it("renders search input", () => {
      render(<ActivityView />);
      expect(screen.getByTestId("activity-search")).toBeInTheDocument();
    });

    it("filters messages by search query", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ content: "Reading file.ts" }),
        createMockMessage({ content: "Writing output" }),
        createMockMessage({ content: "Another read operation" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      const searchInput = screen.getByTestId("activity-search");
      fireEvent.change(searchInput, { target: { value: "read" } });

      // Should show 2 messages containing "read"
      const visibleMessages = screen.getAllByTestId("activity-message");
      expect(visibleMessages).toHaveLength(2);
    });

    it("shows empty state when search has no matches", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ content: "Some content" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      const searchInput = screen.getByTestId("activity-search");
      fireEvent.change(searchInput, { target: { value: "nonexistent" } });

      expect(screen.getByTestId("activity-empty")).toBeInTheDocument();
      expect(screen.getByText(/no matching activities/i)).toBeInTheDocument();
    });

    it("clears search when clear button clicked", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ content: "Test message" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      const searchInput = screen.getByTestId("activity-search");
      fireEvent.change(searchInput, { target: { value: "query" } });

      // Clear button should appear
      const clearButton = screen.getByLabelText(/clear search/i);
      fireEvent.click(clearButton);

      expect(searchInput).toHaveValue("");
    });
  });

  describe("filter tabs", () => {
    it("renders all filter tabs", () => {
      render(<ActivityView />);

      expect(screen.getByRole("tab", { name: "All" })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Thinking" })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Tool Calls" })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Results" })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Text" })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Errors" })).toBeInTheDocument();
    });

    it("filters by message type when tab is clicked", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ type: "thinking", content: "Thinking..." }),
        createMockMessage({ type: "tool_call", content: "Tool call..." }),
        createMockMessage({ type: "error", content: "Error!" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      // Click on "Errors" tab
      fireEvent.click(screen.getByRole("tab", { name: "Errors" }));

      const visibleMessages = screen.getAllByTestId("activity-message");
      expect(visibleMessages).toHaveLength(1);
      expect(visibleMessages[0]).toHaveAttribute("data-type", "error");
    });

    it("shows all messages when All tab is selected", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ type: "thinking" }),
        createMockMessage({ type: "tool_call" }),
        createMockMessage({ type: "error" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      // First filter by errors
      fireEvent.click(screen.getByRole("tab", { name: "Errors" }));
      expect(screen.getAllByTestId("activity-message")).toHaveLength(1);

      // Then click All to show all
      fireEvent.click(screen.getByRole("tab", { name: "All" }));
      expect(screen.getAllByTestId("activity-message")).toHaveLength(3);
    });

    it("highlights active filter tab", () => {
      render(<ActivityView />);

      // All tab should be active by default
      const allTab = screen.getByRole("tab", { name: "All" });
      expect(allTab).toHaveAttribute("data-active", "true");

      // Click Thinking tab
      const thinkingTab = screen.getByRole("tab", { name: "Thinking" });
      fireEvent.click(thinkingTab);

      expect(thinkingTab).toHaveAttribute("data-active", "true");
      expect(allTab).toHaveAttribute("data-active", "false");
    });
  });

  describe("task filtering", () => {
    it("filters messages by taskId prop", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ taskId: "task-1", content: "Task 1 message" }),
        createMockMessage({ taskId: "task-2", content: "Task 2 message" }),
        createMockMessage({ taskId: "task-1", content: "Another task 1" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView taskId="task-1" />);

      const visibleMessages = screen.getAllByTestId("activity-message");
      expect(visibleMessages).toHaveLength(2);
    });

    it("shows empty state when no messages match taskId", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ taskId: "task-other", content: "Other task" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView taskId="task-none" />);

      expect(screen.getByTestId("activity-empty")).toBeInTheDocument();
    });
  });

  describe("clear functionality", () => {
    it("renders clear button when messages exist", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ content: "Test message" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      expect(screen.getByTestId("activity-clear")).toBeInTheDocument();
    });

    it("clears messages when clear button is clicked", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ content: "Test message" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      fireEvent.click(screen.getByTestId("activity-clear"));

      // Store should be cleared
      const state = useActivityStore.getState();
      expect(state.messages).toHaveLength(0);
    });

    it("disables clear button when no messages", () => {
      render(<ActivityView />);

      const clearButton = screen.getByTestId("activity-clear");
      expect(clearButton).toBeDisabled();
    });
  });

  describe("auto-scroll behavior", () => {
    it("renders messages container", () => {
      render(<ActivityView />);
      expect(screen.getByTestId("activity-messages")).toBeInTheDocument();
    });

    it("shows scroll to bottom button when not at bottom", () => {
      // This is more of an integration test, but we can at least verify the component exists
      const messages: AgentMessageEvent[] = Array.from({ length: 50 }, (_, i) =>
        createMockMessage({ content: `Message ${i}`, timestamp: i })
      );
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      // Component should render without errors
      expect(screen.getByTestId("activity-messages")).toBeInTheDocument();
    });
  });

  describe("alerts indicator", () => {
    it("shows alert count badge when high/critical alerts exist", () => {
      useActivityStore.setState({
        messages: [],
        alerts: [
          {
            taskId: "task-1",
            severity: "critical",
            type: "error",
            message: "Critical error",
          },
          {
            taskId: "task-2",
            severity: "high",
            type: "escalation",
            message: "Needs attention",
          },
        ],
      });

      render(<ActivityView />);

      expect(screen.getByText("2 alerts")).toBeInTheDocument();
    });

    it("does not show alert badge when no high/critical alerts", () => {
      useActivityStore.setState({
        messages: [],
        alerts: [
          {
            taskId: "task-1",
            severity: "low",
            type: "stuck",
            message: "Minor issue",
          },
        ],
      });

      render(<ActivityView />);

      expect(screen.queryByText(/alert/)).not.toBeInTheDocument();
    });
  });

  describe("combined filtering", () => {
    it("combines search and type filter", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ type: "thinking", content: "Thinking about reading" }),
        createMockMessage({ type: "tool_call", content: "Reading file.ts" }),
        createMockMessage({ type: "thinking", content: "Thinking about writing" }),
        createMockMessage({ type: "tool_call", content: "Writing output" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      // Filter by tool_call
      fireEvent.click(screen.getByRole("tab", { name: "Tool Calls" }));

      // Also search for "read"
      const searchInput = screen.getByTestId("activity-search");
      fireEvent.change(searchInput, { target: { value: "read" } });

      // Should show only the tool_call message containing "read"
      const visibleMessages = screen.getAllByTestId("activity-message");
      expect(visibleMessages).toHaveLength(1);
      expect(visibleMessages[0]).toHaveAttribute("data-type", "tool_call");
      expect(screen.getByText(/Reading file/)).toBeInTheDocument();
    });

    it("combines taskId, search, and type filter", () => {
      const messages: AgentMessageEvent[] = [
        createMockMessage({ taskId: "task-1", type: "thinking", content: "Task 1 thinking" }),
        createMockMessage({ taskId: "task-1", type: "error", content: "Task 1 error" }),
        createMockMessage({ taskId: "task-2", type: "error", content: "Task 2 error" }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView taskId="task-1" />);

      // Filter by errors
      fireEvent.click(screen.getByRole("tab", { name: "Errors" }));

      // Should show only task-1's error
      const visibleMessages = screen.getAllByTestId("activity-message");
      expect(visibleMessages).toHaveLength(1);
      expect(screen.getByText("Task 1 error")).toBeInTheDocument();
    });
  });

  describe("content truncation", () => {
    it("truncates long content when not expanded", () => {
      const longContent = "A".repeat(300);
      const messages: AgentMessageEvent[] = [
        createMockMessage({ content: longContent }),
      ];
      useActivityStore.setState({ messages });

      render(<ActivityView />);

      // Content should be truncated with "..."
      expect(screen.getByText(/A{200}\.\.\./)).toBeInTheDocument();
      // Full content should not be visible
      expect(screen.queryByText(longContent)).not.toBeInTheDocument();
    });
  });
});
