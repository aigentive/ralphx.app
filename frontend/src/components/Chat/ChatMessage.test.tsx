/**
 * ChatMessage component tests
 * Tests for individual chat message display with role-based styling
 */

import { afterEach, describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { ChatMessage } from "./ChatMessage";
import type { ChatMessage as ChatMessageType } from "@/types/ideation";
import { makeIdeationChatMessage } from "./__tests__/chatRenderFixtures";

afterEach(() => {
  vi.useRealTimers();
});

// ============================================================================
// Test Data
// ============================================================================

const userMessage: ChatMessageType = makeIdeationChatMessage({
  id: "msg-1",
  role: "user",
  content: "Hello, I need help with authentication",
  createdAt: "2026-01-24T12:00:00Z",
});

const orchestratorMessage: ChatMessageType = makeIdeationChatMessage({
  id: "msg-2",
  role: "orchestrator",
  content: "I can help you design an authentication system.",
  parentMessageId: "msg-1",
  createdAt: "2026-01-24T12:01:00Z",
});

const systemMessage: ChatMessageType = makeIdeationChatMessage({
  id: "msg-3",
  role: "system",
  content: "Session started",
  createdAt: "2026-01-24T11:59:00Z",
});

const markdownMessage: ChatMessageType = makeIdeationChatMessage({
  id: "msg-4",
  role: "orchestrator",
  content:
    "Here's a **bold** suggestion:\n\n1. First step\n2. Second step\n\n```typescript\nconst auth = new Auth();\n```",
  createdAt: "2026-01-24T12:05:00Z",
});

const messageWithToolCalls: ChatMessageType = makeIdeationChatMessage({
  id: "msg-5",
  role: "orchestrator",
  content: "I'll create a task proposal for you.",
  toolCalls: JSON.stringify([
    {
      id: "call-1",
      name: "create_task_proposal",
      input: {
        title: "Add authentication",
        category: "feature",
      },
      result: {
        proposal_id: "proposal-123",
      },
    },
    {
      id: "call-2",
      name: "update_task",
      input: {
        task_id: "task-456",
        status: "in_progress",
      },
      result: {
        success: true,
      },
    },
  ]),
  createdAt: "2026-01-24T12:10:00Z",
});

const messageWithFailedToolCall: ChatMessageType = makeIdeationChatMessage({
  id: "msg-6",
  role: "orchestrator",
  content: "I tried to read the file but encountered an error.",
  toolCalls: JSON.stringify([
    {
      id: "call-3",
      name: "read",
      input: {
        file_path: "/nonexistent/file.txt",
      },
      error: "File not found",
    },
  ]),
  createdAt: "2026-01-24T12:15:00Z",
});

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

      // User messages should have a bubble with orange gradient
      const container = screen.getByTestId(`chat-message-${userMessage.id}`);
      // The bubble uses inline styles with the warm orange gradient
      const bubble = container.querySelector('[style*="linear-gradient"]');
      expect(bubble).toBeInTheDocument();
    });

    it("shows orchestrator message bubble with neutral color", () => {
      render(<ChatMessage message={orchestratorMessage} />);

      // Orchestrator messages should have a bubble with dark gradient
      const container = screen.getByTestId(`chat-message-${orchestratorMessage.id}`);
      const bubble = container.querySelector('[style*="linear-gradient"]');
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
      const inlineCodeMessage: ChatMessageType = makeIdeationChatMessage({
        ...orchestratorMessage,
        id: "msg-inline",
        content: "Try using `useState` hook",
      });
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
    it("formats timestamp as human-diff text", () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date(2026, 3, 25, 16, 33, 0));
      const message = makeIdeationChatMessage({
        ...userMessage,
        createdAt: new Date(2026, 3, 25, 14, 33, 0).toISOString(),
      });

      render(<ChatMessage message={message} />);

      const timestamp = screen.getByTestId("chat-message-timestamp");
      expect(timestamp).toBeInTheDocument();
      expect(timestamp).toHaveTextContent("2 hours ago");
    });

    it("uses the native title for the full timestamp", () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date(2026, 3, 25, 16, 33, 0));
      const message = makeIdeationChatMessage({
        ...userMessage,
        createdAt: new Date(2026, 3, 25, 16, 32, 0).toISOString(),
      });

      render(<ChatMessage message={message} />);

      const timestamp = screen.getByTestId("chat-message-timestamp");
      expect(timestamp).toHaveTextContent("1 minute ago");
      expect(timestamp).toHaveAttribute("title", "Apr 25, 2026, 4:32 PM");
    });

    it("uses the time and date label for timestamps outside the 7-day window", () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date(2026, 3, 25, 16, 33, 0));
      const message = makeIdeationChatMessage({
        ...userMessage,
        createdAt: new Date(2026, 3, 17, 16, 33, 0).toISOString(),
      });

      render(<ChatMessage message={message} showFullTimestamp />);

      const timestamp = screen.getByTestId("chat-message-timestamp");
      expect(timestamp).toHaveTextContent("4:33 PM * Apr 17");
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

      // The bubble should have break-words class to handle long content
      const container = screen.getByTestId(`chat-message-msg-long`);
      const bubble = container.querySelector('[style*="linear-gradient"]');
      expect(bubble).toBeInTheDocument();
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

  describe("Tool Calls", () => {
    it("does not render tool calls section when message has no tool calls", () => {
      render(<ChatMessage message={userMessage} />);

      expect(
        screen.queryByTestId("chat-message-tool-calls")
      ).not.toBeInTheDocument();
    });

    it("does not render tool calls section when toolCalls is null", () => {
      render(<ChatMessage message={orchestratorMessage} />);

      expect(
        screen.queryByTestId("chat-message-tool-calls")
      ).not.toBeInTheDocument();
    });

    it("renders tool calls when message has tool calls", () => {
      render(<ChatMessage message={messageWithToolCalls} />);

      expect(screen.getByTestId("chat-message-tool-calls")).toBeInTheDocument();
    });

    it("renders multiple tool call indicators", () => {
      const { container } = render(<ChatMessage message={messageWithToolCalls} />);

      const toolCallIndicators = screen.getAllByTestId("tool-call-indicator");
      expect(toolCallIndicators).toHaveLength(1);
      expect(container.querySelector('[data-testid="proposal-widget-created"]')).toBeInTheDocument();
    });

    it("renders tool calls as part of message content", () => {
      render(<ChatMessage message={messageWithToolCalls} />);

      const article = screen.getByTestId(`chat-message-${messageWithToolCalls.id}`);
      const toolCallsSection = screen.getByTestId("chat-message-tool-calls");

      // Tool calls render within the message article
      expect(article).toContainElement(toolCallsSection);
    });

    it("handles failed tool calls", () => {
      render(<ChatMessage message={messageWithFailedToolCall} />);

      expect(screen.getByTestId("chat-message-tool-calls")).toBeInTheDocument();
      expect(screen.getByText("error")).toBeInTheDocument();
      expect(screen.getByText("File not found")).toBeInTheDocument();
    });

    it("handles invalid JSON in toolCalls gracefully", () => {
      const invalidMessage: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-invalid",
        toolCalls: "invalid json{{{",
      };
      render(<ChatMessage message={invalidMessage} />);

      // Should not render tool calls section for invalid JSON
      expect(
        screen.queryByTestId("chat-message-tool-calls")
      ).not.toBeInTheDocument();
    });

    it("handles non-array JSON in toolCalls gracefully", () => {
      const nonArrayMessage: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-non-array",
        toolCalls: JSON.stringify({ not: "an array" }),
      };
      render(<ChatMessage message={nonArrayMessage} />);

      // Should not render tool calls section for non-array JSON
      expect(
        screen.queryByTestId("chat-message-tool-calls")
      ).not.toBeInTheDocument();
    });
  });

  describe("Agent tool call rendering via ToolCallIndicator (ideation panel path)", () => {
    it("renders Agent tool call as TaskToolCallCard via contentBlocks", () => {
      // The ideation panel ChatMessage parses contentBlocks JSON and passes each
      // tool_use block to ToolCallIndicator, which routes Agent → TaskToolCallCard.
      const messageWithAgentCall: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-agent-contentblocks",
        content: "",
        toolCalls: null,
        contentBlocks: JSON.stringify([
          {
            type: "tool_use",
            id: "agent-call-ideation",
            name: "Agent",
            arguments: {
              description: "Explore the codebase",
              subagent_type: "Explore",
              model: "sonnet",
              prompt: "Find all TypeScript files",
            },
          },
        ]),
      };

      const { container } = render(<ChatMessage message={messageWithAgentCall} />);

      // Should render as TaskToolCallCard (not generic ToolCallIndicator)
      expect(container.querySelector('[data-testid="task-tool-call-card"]')).toBeInTheDocument();
    });

    it("renders Task tool call as TaskToolCallCard via contentBlocks", () => {
      const messageWithTaskCall: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-task-contentblocks",
        content: "",
        toolCalls: null,
        contentBlocks: JSON.stringify([
          {
            type: "tool_use",
            id: "task-call-ideation",
            name: "Task",
            arguments: {
              description: "Run tests",
              subagent_type: "general-purpose",
              model: "opus",
              prompt: "Execute the test suite",
            },
          },
        ]),
      };

      const { container } = render(<ChatMessage message={messageWithTaskCall} />);

      expect(container.querySelector('[data-testid="task-tool-call-card"]')).toBeInTheDocument();
    });

    it("renders Agent tool call description and subagent type in ideation panel", () => {
      const messageWithAgentCall: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-agent-badges",
        content: "",
        toolCalls: null,
        contentBlocks: JSON.stringify([
          {
            type: "tool_use",
            id: "agent-badges-call",
            name: "Agent",
            arguments: {
              description: "Plan the implementation",
              subagent_type: "Plan",
              model: "opus",
              prompt: "Create a detailed implementation plan",
            },
          },
        ]),
      };

      render(<ChatMessage message={messageWithAgentCall} />);

      // Verify description and subagent type badge render
      expect(screen.getByText("Plan the implementation")).toBeInTheDocument();
      expect(screen.getByText("Plan")).toBeInTheDocument();
      expect(screen.getByText("opus")).toBeInTheDocument();
    });

    it("renders interleaved text and Agent calls via contentBlocks", () => {
      const messageWithInterleaved: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-interleaved-agent",
        content: "",
        toolCalls: null,
        contentBlocks: JSON.stringify([
          { type: "text", text: "Spawning a research agent:" },
          {
            type: "tool_use",
            id: "agent-research-call",
            name: "Agent",
            arguments: {
              description: "Research the codebase",
              subagent_type: "general-purpose",
              model: "sonnet",
              prompt: "Find all authentication patterns",
            },
          },
          { type: "text", text: "Agent completed." },
        ]),
      };

      const { container } = render(<ChatMessage message={messageWithInterleaved} />);

      // Both text blocks render
      expect(screen.getByText("Spawning a research agent:")).toBeInTheDocument();
      expect(screen.getByText("Agent completed.")).toBeInTheDocument();

      // Agent card renders
      expect(container.querySelector('[data-testid="task-tool-call-card"]')).toBeInTheDocument();
    });

    it("falls back to generic ToolCallIndicator for non-subagent tool calls in ideation panel", () => {
      const messageWithGenericTool: ChatMessageType = {
        ...orchestratorMessage,
        id: "msg-generic-tool",
        content: "",
        toolCalls: null,
        contentBlocks: JSON.stringify([
          {
            type: "tool_use",
            id: "generic-tool-call",
            name: "update_task",
            arguments: { task_id: "task-123", status: "completed" },
            result: { ok: true },
          },
        ]),
      };

      const { container } = render(<ChatMessage message={messageWithGenericTool} />);

      // Should use generic ToolCallIndicator, not TaskToolCallCard
      expect(container.querySelector('[data-testid="tool-call-indicator"]')).toBeInTheDocument();
      expect(container.querySelector('[data-testid="task-tool-call-card"]')).not.toBeInTheDocument();
    });
  });
});
