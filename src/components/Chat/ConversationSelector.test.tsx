/**
 * ConversationSelector tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ConversationSelector } from "./ConversationSelector";
import type { ChatConversation } from "@/types/chat-conversation";

// ============================================================================
// Test Data
// ============================================================================

const mockConversations: ChatConversation[] = [
  {
    id: "conv-1",
    contextType: "ideation",
    contextId: "session-1",
    claudeSessionId: "claude-session-1",
    title: "Dark mode implementation",
    messageCount: 12,
    lastMessageAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(), // 2 hours ago
    createdAt: new Date(Date.now() - 3 * 60 * 60 * 1000).toISOString(),
    updatedAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
  },
  {
    id: "conv-2",
    contextType: "ideation",
    contextId: "session-1",
    claudeSessionId: "claude-session-2",
    title: "API refactoring discussion",
    messageCount: 8,
    lastMessageAt: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(), // 1 day ago
    createdAt: new Date(Date.now() - 25 * 60 * 60 * 1000).toISOString(),
    updatedAt: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
  },
  {
    id: "conv-3",
    contextType: "ideation",
    contextId: "session-1",
    claudeSessionId: null,
    title: null,
    messageCount: 0,
    lastMessageAt: null,
    createdAt: new Date(Date.now() - 30 * 60 * 1000).toISOString(), // 30 min ago
    updatedAt: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
  },
];

// ============================================================================
// Tests
// ============================================================================

describe("ConversationSelector", () => {
  const defaultProps = {
    contextType: "ideation" as const,
    contextId: "session-1",
    conversations: mockConversations,
    activeConversationId: "conv-1",
    onSelectConversation: vi.fn(),
    onNewConversation: vi.fn(),
  };

  it("renders the history icon button", () => {
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    expect(trigger).toBeInTheDocument();
    expect(trigger).toHaveAttribute("aria-label", "Conversation history");
  });

  it("opens dropdown menu when clicked", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    const menu = screen.getByTestId("conversation-selector-menu");
    expect(menu).toBeInTheDocument();
  });

  it("shows 'New Conversation' option", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    const newButton = screen.getByTestId("conversation-selector-new");
    expect(newButton).toBeInTheDocument();
    expect(newButton).toHaveTextContent("New Conversation");
  });

  it("calls onNewConversation when 'New Conversation' is clicked", async () => {
    const user = userEvent.setup();
    const onNewConversation = vi.fn();
    render(
      <ConversationSelector {...defaultProps} onNewConversation={onNewConversation} />
    );

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    const newButton = screen.getByTestId("conversation-selector-new");
    await user.click(newButton);

    expect(onNewConversation).toHaveBeenCalledTimes(1);
  });

  it("displays all conversations", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    mockConversations.forEach((conv) => {
      const item = screen.getByTestId(`conversation-item-${conv.id}`);
      expect(item).toBeInTheDocument();
    });
  });

  it("shows conversation titles", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    expect(screen.getByText("Dark mode implementation")).toBeInTheDocument();
    expect(screen.getByText("API refactoring discussion")).toBeInTheDocument();
  });

  it("generates fallback title for conversations without title", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    // conv-3 has no title and no messages
    expect(screen.getByText("New conversation")).toBeInTheDocument();
  });

  it("shows message count for each conversation", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    expect(screen.getByText("12 messages")).toBeInTheDocument();
    expect(screen.getByText("8 messages")).toBeInTheDocument();
    expect(screen.getByText("0 messages")).toBeInTheDocument();
  });

  it("shows singular 'message' for count of 1", async () => {
    const user = userEvent.setup();
    const singleMessageConv: ChatConversation = {
      ...mockConversations[0],
      id: "conv-single",
      messageCount: 1,
    };

    render(
      <ConversationSelector
        {...defaultProps}
        conversations={[singleMessageConv]}
      />
    );

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    expect(screen.getByText("1 message")).toBeInTheDocument();
  });

  it("displays relative time for last message", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    // Check for "ago" suffix (exact time depends on date-fns)
    const items = screen.getAllByText(/ago$/);
    expect(items.length).toBeGreaterThan(0);
  });

  it("shows 'No messages' for conversations without messages", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    expect(screen.getByText("No messages")).toBeInTheDocument();
  });

  it("indicates active conversation with filled dot", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    const activeItem = screen.getByTestId("conversation-item-conv-1");
    expect(activeItem).toHaveAttribute("data-active", "true");

    const inactiveItem = screen.getByTestId("conversation-item-conv-2");
    expect(inactiveItem).toHaveAttribute("data-active", "false");
  });

  it("calls onSelectConversation when a conversation is clicked", async () => {
    const user = userEvent.setup();
    const onSelectConversation = vi.fn();
    render(
      <ConversationSelector
        {...defaultProps}
        onSelectConversation={onSelectConversation}
      />
    );

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    const item = screen.getByTestId("conversation-item-conv-2");
    await user.click(item);

    expect(onSelectConversation).toHaveBeenCalledWith("conv-2");
  });

  it("sorts conversations by last message date (most recent first)", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    const items = screen.getAllByTestId(/^conversation-item-/);

    // conv-1 (2 hours ago) should be first
    expect(items[0]).toHaveAttribute("data-testid", "conversation-item-conv-1");
    // conv-2 (1 day ago) should be second
    expect(items[1]).toHaveAttribute("data-testid", "conversation-item-conv-2");
    // conv-3 (no messages) should be last
    expect(items[2]).toHaveAttribute("data-testid", "conversation-item-conv-3");
  });

  it("shows loading state", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} isLoading={true} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    expect(screen.getByText("Loading conversations...")).toBeInTheDocument();
    expect(screen.queryByTestId(/^conversation-item-/)).not.toBeInTheDocument();
  });

  it("shows empty state when no conversations", async () => {
    const user = userEvent.setup();
    render(<ConversationSelector {...defaultProps} conversations={[]} />);

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    expect(screen.getByText("No conversations yet")).toBeInTheDocument();
  });

  it("does not show empty state when loading", async () => {
    const user = userEvent.setup();
    render(
      <ConversationSelector {...defaultProps} conversations={[]} isLoading={true} />
    );

    const trigger = screen.getByTestId("conversation-selector-trigger");
    await user.click(trigger);

    expect(screen.getByText("Loading conversations...")).toBeInTheDocument();
    expect(screen.queryByText("No conversations yet")).not.toBeInTheDocument();
  });
});
