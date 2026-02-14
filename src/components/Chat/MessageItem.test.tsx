/**
 * MessageItem.test.tsx - Tests for MessageItem component
 *
 * Tests attachment rendering integration with MessageAttachments component
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MessageItem } from "./MessageItem";
import type { MessageAttachment } from "./MessageAttachments";

describe("MessageItem - Attachment Integration", () => {
  const baseProps = {
    role: "user",
    content: "Hello world",
    createdAt: new Date().toISOString(),
  };

  const mockAttachments: MessageAttachment[] = [
    {
      id: "att-1",
      fileName: "test.txt",
      fileSize: 1024,
      mimeType: "text/plain",
    },
    {
      id: "att-2",
      fileName: "image.png",
      fileSize: 2048,
      mimeType: "image/png",
    },
  ];

  it("renders MessageAttachments for user messages with attachments", () => {
    render(
      <MessageItem {...baseProps} role="user" attachments={mockAttachments} />
    );

    // MessageAttachments should render chips with data-testid="attachment-chip"
    const chips = screen.getAllByTestId("attachment-chip");
    expect(chips).toHaveLength(2);

    // Verify file names are displayed
    expect(screen.getByText("test.txt")).toBeInTheDocument();
    expect(screen.getByText("image.png")).toBeInTheDocument();
  });

  it("does NOT render MessageAttachments for user messages without attachments", () => {
    render(<MessageItem {...baseProps} role="user" />);

    // No attachment chips should be present
    const chips = screen.queryAllByTestId("attachment-chip");
    expect(chips).toHaveLength(0);
  });

  it("does NOT render MessageAttachments for user messages with empty attachments array", () => {
    render(<MessageItem {...baseProps} role="user" attachments={[]} />);

    // No attachment chips should be present
    const chips = screen.queryAllByTestId("attachment-chip");
    expect(chips).toHaveLength(0);
  });

  it("does NOT render MessageAttachments for assistant messages even if attachments prop is passed", () => {
    render(
      <MessageItem
        {...baseProps}
        role="assistant"
        attachments={mockAttachments}
      />
    );

    // No attachment chips should be present for assistant messages
    const chips = screen.queryAllByTestId("attachment-chip");
    expect(chips).toHaveLength(0);
  });

  it("MessageAttachments appear above the text bubble for user messages", () => {
    const { container } = render(
      <MessageItem {...baseProps} role="user" attachments={mockAttachments} />
    );

    // Find the parent flex column container
    const flexColumn = container.querySelector(".flex.flex-col");
    expect(flexColumn).toBeInTheDocument();

    if (!flexColumn) {
      throw new Error("Flex column container not found");
    }

    // Get all children of the flex column
    const children = Array.from(flexColumn.children);

    // MessageAttachments should be first (index 0)
    const firstChild = children[0];
    expect(firstChild?.querySelector('[data-testid="attachment-chip"]')).toBeInTheDocument();

    // Text bubble should come after attachments
    const textBubble = children.find((child) =>
      child.textContent?.includes("Hello world")
    );
    expect(textBubble).toBeInTheDocument();

    // Verify attachments come before text bubble in DOM order
    const attachmentsIndex = children.indexOf(firstChild);
    const textBubbleIndex = textBubble ? children.indexOf(textBubble) : -1;
    expect(attachmentsIndex).toBeLessThan(textBubbleIndex);
  });

  it("works with content blocks rendering", () => {
    const contentBlocks = [
      { type: "text" as const, text: "First block" },
      { type: "text" as const, text: "Second block" },
    ];

    render(
      <MessageItem
        {...baseProps}
        role="user"
        contentBlocks={contentBlocks}
        attachments={mockAttachments}
      />
    );

    // Attachments should render
    const chips = screen.getAllByTestId("attachment-chip");
    expect(chips).toHaveLength(2);

    // Content blocks should also render
    expect(screen.getByText("First block")).toBeInTheDocument();
    expect(screen.getByText("Second block")).toBeInTheDocument();
  });

  it("works with legacy rendering (toolCalls + text)", () => {
    const toolCalls = [
      {
        id: "call-1",
        name: "read_file",
        arguments: { path: "test.txt" },
        result: "file content",
      },
    ];

    render(
      <MessageItem
        {...baseProps}
        role="assistant"
        toolCalls={toolCalls}
        attachments={mockAttachments}
      />
    );

    // For assistant messages, attachments should NOT render
    const chips = screen.queryAllByTestId("attachment-chip");
    expect(chips).toHaveLength(0);

    // Tool calls should render (we can check for tool call indicator presence)
    expect(screen.getByText("read_file")).toBeInTheDocument();
  });
});
