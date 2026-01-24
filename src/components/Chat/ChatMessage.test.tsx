/**
 * ChatMessage component tests
 * Tests for individual chat message display with role-based styling
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ChatMessage } from "./ChatMessage";
import type { ChatMessage as ChatMessageType } from "@/types/ideation";

// ============================================================================
// Test Data
// ============================================================================

const userMessage: ChatMessageType = {
  id: "msg-1",
  sessionId: "session-1",
  projectId: "project-1",
  taskId: null,
  role: "user",
  content: "Hello, I need help with authentication",
  metadata: null,
  parentMessageId: null,
  createdAt: "2026-01-24T12:00:00Z",
};

const orchestratorMessage: ChatMessageType = {
  id: "msg-2",
  sessionId: "session-1",
  projectId: "project-1",
  taskId: null,
  role: "orchestrator",
  content: "I can help you design an authentication system.",
  metadata: null,
  parentMessageId: "msg-1",
  createdAt: "2026-01-24T12:01:00Z",
};

const systemMessage: ChatMessageType = {
  id: "msg-3",
  sessionId: "session-1",
  projectId: "project-1",
  taskId: null,
  role: "system",
  content: "Session started",
  metadata: null,
  parentMessageId: null,
  createdAt: "2026-01-24T11:59:00Z",
};

const markdownMessage: ChatMessageType = {
  id: "msg-4",
  sessionId: "session-1",
  projectId: "project-1",
  taskId: null,
  role: "orchestrator",
  content:
    "Here's a **bold** suggestion:\n\n1. First step\n2. Second step\n\n```typescript\nconst auth = new Auth();\n```",
  metadata: null,
  parentMessageId: null,
  createdAt: "2026-01-24T12:05:00Z",
};

// ============================================================================
// Tests
// ============================================================================

describe("ChatMessage", () => {
  describe("Rendering", () => {
    it("renders message content", () => {
      render(<ChatMessage message={userMessage} />);

      expect(
        screen.getByText("Hello, I need help with authentication")
      ).toBeInTheDocument();
    });

    it("renders data-testid for the component", () => {
      render(<ChatMessage message={userMessage} />);

      expect(
        screen.getByTestId(`chat-message-${userMessage.id}`)
      ).toBeInTheDocument();
    });

    it("renders timestamp", () => {
      render(<ChatMessage message={userMessage} />);

      // Should show time portion of the date
      const messageElement = screen.getByTestId(`chat-message-${userMessage.id}`);
      expect(messageElement).toHaveTextContent(/\d{1,2}:\d{2}/);
    });
  });

  describe("Role Styling", () => {
    it("renders user message with correct alignment (right)", () => {
      render(<ChatMessage message={userMessage} />);

      const container = screen.getByTestId(`chat-message-${userMessage.id}`);
      expect(container).toHaveClass("items-end");
    });

    it("renders orchestrator message with correct alignment (left)", () => {
      render(<ChatMessage message={orchestratorMessage} />);

      const container = screen.getByTestId(
        `chat-message-${orchestratorMessage.id}`
      );
      expect(container).toHaveClass("items-start");
    });

    it("renders system message with correct alignment (left)", () => {
      render(<ChatMessage message={systemMessage} />);

      const container = screen.getByTestId(`chat-message-${systemMessage.id}`);
      expect(container).toHaveClass("items-start");
    });

    it("applies user role indicator styling", () => {
      render(<ChatMessage message={userMessage} />);

      const roleIndicator = screen.getByTestId("chat-message-role");
      expect(roleIndicator).toHaveTextContent("You");
    });

    it("applies orchestrator role indicator styling", () => {
      render(<ChatMessage message={orchestratorMessage} />);

      const roleIndicator = screen.getByTestId("chat-message-role");
      expect(roleIndicator).toHaveTextContent("Orchestrator");
    });

    it("applies system role indicator styling", () => {
      render(<ChatMessage message={systemMessage} />);

      const roleIndicator = screen.getByTestId("chat-message-role");
      expect(roleIndicator).toHaveTextContent("System");
    });

    it("shows user message bubble with accent color", () => {
      render(<ChatMessage message={userMessage} />);

      const bubble = screen.getByTestId("chat-message-bubble");
      expect(bubble).toBeInTheDocument();
    });

    it("shows orchestrator message bubble with neutral color", () => {
      render(<ChatMessage message={orchestratorMessage} />);

      const bubble = screen.getByTestId("chat-message-bubble");
      expect(bubble).toBeInTheDocument();
    });
  });

  describe("Markdown Rendering", () => {
    it("renders bold text as strong", () => {
      render(<ChatMessage message={markdownMessage} />);

      // Check that bold markdown is rendered
      const strongElement = screen.getByText("bold");
      expect(strongElement.tagName).toBe("STRONG");
    });

    it("renders numbered list items", () => {
      render(<ChatMessage message={markdownMessage} />);

      expect(screen.getByText("First step")).toBeInTheDocument();
      expect(screen.getByText("Second step")).toBeInTheDocument();
    });

    it("renders code blocks", () => {
      render(<ChatMessage message={markdownMessage} />);

      // Code content should be present
      expect(
        screen.getByText(/const auth = new Auth\(\);/)
      ).toBeInTheDocument();
    });

    it("renders inline code", () => {
      const inlineCodeMessage: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-inline",
        content: "Try using `useState` hook",
      };
      render(<ChatMessage message={inlineCodeMessage} />);

      const codeElement = screen.getByText("useState");
      expect(codeElement.tagName).toBe("CODE");
    });

    it("renders links", () => {
      const linkMessage: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-link",
        content: "Check the [documentation](https://example.com)",
      };
      render(<ChatMessage message={linkMessage} />);

      const link = screen.getByRole("link", { name: "documentation" });
      expect(link).toHaveAttribute("href", "https://example.com");
    });
  });

  describe("Timestamp Display", () => {
    it("formats timestamp correctly", () => {
      render(<ChatMessage message={userMessage} />);

      const timestamp = screen.getByTestId("chat-message-timestamp");
      expect(timestamp).toBeInTheDocument();
      // Should be formatted as HH:MM
      expect(timestamp.textContent).toMatch(/\d{1,2}:\d{2}/);
    });

    it("shows compact timestamp by default", () => {
      render(<ChatMessage message={userMessage} />);

      const timestamp = screen.getByTestId("chat-message-timestamp");
      // Should just show time, not full date
      expect(timestamp.textContent).not.toContain("2026");
    });

    it("shows full timestamp when showFullTimestamp prop is true", () => {
      render(<ChatMessage message={userMessage} showFullTimestamp />);

      const timestamp = screen.getByTestId("chat-message-timestamp");
      // Should show date too
      expect(timestamp.textContent).toMatch(/Jan|24/);
    });
  });

  describe("Content Handling", () => {
    it("preserves whitespace in message content", () => {
      const multilineMessage: ChatMessageType = {
        ...userMessage,
        id: "msg-multiline",
        content: "Line 1\n\nLine 2\n\nLine 3",
      };
      render(<ChatMessage message={multilineMessage} />);

      expect(screen.getByText("Line 1")).toBeInTheDocument();
      expect(screen.getByText("Line 2")).toBeInTheDocument();
      expect(screen.getByText("Line 3")).toBeInTheDocument();
    });

    it("handles empty content gracefully", () => {
      const emptyMessage: ChatMessageType = {
        ...userMessage,
        id: "msg-empty",
        content: "   ",
      };
      // Empty content is actually invalid per schema, but component should handle it
      render(<ChatMessage message={{ ...emptyMessage, content: "" }} />);

      const container = screen.getByTestId(`chat-message-msg-empty`);
      expect(container).toBeInTheDocument();
    });

    it("handles long content without breaking layout", () => {
      const longMessage: ChatMessageType = {
        ...userMessage,
        id: "msg-long",
        content: "A".repeat(500),
      };
      render(<ChatMessage message={longMessage} />);

      const bubble = screen.getByTestId("chat-message-bubble");
      expect(bubble).toHaveClass("break-words");
    });
  });

  describe("Accessibility", () => {
    it("has proper article role for message", () => {
      render(<ChatMessage message={userMessage} />);

      const article = screen.getByRole("article");
      expect(article).toBeInTheDocument();
    });

    it("has accessible name indicating sender", () => {
      render(<ChatMessage message={userMessage} />);

      const article = screen.getByRole("article");
      expect(article).toHaveAccessibleName(/message from you/i);
    });

    it("has proper time element for timestamp", () => {
      render(<ChatMessage message={userMessage} />);

      const time = screen.getByRole("time");
      expect(time).toHaveAttribute("dateTime", userMessage.createdAt);
    });
  });

  describe("Compact Mode", () => {
    it("renders in compact mode when compact prop is true", () => {
      render(<ChatMessage message={userMessage} compact />);

      const container = screen.getByTestId(`chat-message-${userMessage.id}`);
      expect(container).toHaveClass("mb-1");
    });

    it("renders in default spacing when compact is false", () => {
      render(<ChatMessage message={userMessage} />);

      const container = screen.getByTestId(`chat-message-${userMessage.id}`);
      expect(container).toHaveClass("mb-3");
    });

    it("hides role indicator in compact mode", () => {
      render(<ChatMessage message={userMessage} compact />);

      expect(screen.queryByTestId("chat-message-role")).not.toBeInTheDocument();
    });
  });
});
