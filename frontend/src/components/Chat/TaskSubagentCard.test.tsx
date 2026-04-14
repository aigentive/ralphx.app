import { describe, expect, it, vi, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { TaskSubagentCard } from "./TaskSubagentCard";
import type { StreamingTask } from "@/types/streaming-task";
import { createTestQueryClient } from "@/test/store-utils";
import { chatApi, type ChatMessageResponse } from "@/api/chat";

function makeStreamingTask(overrides?: Partial<StreamingTask>): StreamingTask {
  return {
    toolUseId: "toolu-task-1",
    toolName: "Task",
    description: "Inspect repository layout",
    subagentType: "Explore",
    model: "sonnet",
    status: "completed",
    startedAt: Date.now() - 6_200,
    completedAt: Date.now(),
    totalDurationMs: 6_200,
    totalTokens: 1_532,
    totalToolUseCount: 3,
    estimatedUsd: 0.43,
    childToolCalls: [],
    ...overrides,
  };
}

function renderWithQueryClient(ui: React.ReactElement) {
  const queryClient = createTestQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("TaskSubagentCard", () => {
  it("renders delegated streaming cards with shared provider chrome", () => {
    render(
      <TaskSubagentCard
        task={makeStreamingTask({
          toolName: "delegate_start",
          description: "Review delegated patch",
          subagentType: "delegated",
          model: "gpt-5.4",
          providerHarness: "codex",
          providerSessionId: "thread-1234567890",
          upstreamProvider: "openai",
          providerProfile: "openai",
          logicalModel: "gpt-5.4",
          effectiveModelId: "gpt-5.4",
        })}
      />,
    );

    expect(screen.getByText("Delegate")).toBeInTheDocument();
    expect(screen.getByText("Codex")).toHaveAttribute(
      "title",
      expect.stringContaining("Upstream: openai"),
    );
    expect(screen.getByText("gpt-5.4")).toBeInTheDocument();
    expect(screen.queryByText("delegated")).not.toBeInTheDocument();
  });

  it("shows collapsed completed summary metrics", () => {
    render(<TaskSubagentCard task={makeStreamingTask()} />);

    expect(
      screen.getByText("6s · 1,532 tokens · 3 tools · $0.43"),
    ).toBeInTheDocument();
  });

  it("shows failure status while preserving subagent type chrome", () => {
    render(
      <TaskSubagentCard
        task={makeStreamingTask({
          status: "failed",
          totalDurationMs: undefined,
          totalTokens: undefined,
          totalToolUseCount: undefined,
          estimatedUsd: undefined,
        })}
      />,
    );

    expect(screen.getByText("Explore")).toBeInTheDocument();
    expect(screen.getByText("failed")).toBeInTheDocument();
  });

  it("does not fetch the delegated conversation until the streaming card is expanded", () => {
    const getConversationSpy = vi.spyOn(chatApi, "getConversation").mockResolvedValue({
      conversation: {
        id: "child-conv-1",
        contextType: "project",
        contextId: "project-1",
        claudeSessionId: null,
        providerSessionId: "thread-123",
        providerHarness: "codex",
        upstreamProvider: "openai",
        providerProfile: "openai",
        title: "Delegated reviewer",
        messageCount: 0,
        lastMessageAt: null,
        createdAt: "2026-04-12T10:00:00Z",
        updatedAt: "2026-04-12T10:00:00Z",
      },
      messages: [],
    });

    renderWithQueryClient(
      <TaskSubagentCard
        task={makeStreamingTask({
          toolName: "delegate_start",
          description: "Review delegated patch",
          subagentType: "delegated",
          model: "gpt-5.4",
          delegatedConversationId: "child-conv-1",
          textOutput: "Delegated review finished",
        })}
      />,
    );

    expect(getConversationSpy).not.toHaveBeenCalled();
  });

  it("renders the delegated conversation transcript inside the expanded streaming card", async () => {
    const getConversationSpy = vi.spyOn(chatApi, "getConversation").mockResolvedValue({
      conversation: {
        id: "child-conv-1",
        contextType: "project",
        contextId: "project-1",
        claudeSessionId: null,
        providerSessionId: "thread-123",
        providerHarness: "codex",
        upstreamProvider: "openai",
        providerProfile: "openai",
        title: "Delegated reviewer",
        messageCount: 2,
        lastMessageAt: "2026-04-12T10:00:06Z",
        createdAt: "2026-04-12T10:00:00Z",
        updatedAt: "2026-04-12T10:00:06Z",
      },
      messages: [
        {
          id: "child-msg-1",
          sessionId: null,
          projectId: null,
          taskId: null,
          role: "user",
          content: "Please inspect the patch",
          metadata: null,
          parentMessageId: null,
          conversationId: "child-conv-1",
          toolCalls: null,
          contentBlocks: null,
          sender: null,
          createdAt: "2026-04-12T10:00:00Z",
        } satisfies ChatMessageResponse,
        {
          id: "child-msg-2",
          sessionId: null,
          projectId: null,
          taskId: null,
          role: "assistant",
          content: "Review complete with no blockers",
          metadata: null,
          parentMessageId: null,
          conversationId: "child-conv-1",
          toolCalls: null,
          contentBlocks: null,
          sender: null,
          createdAt: "2026-04-12T10:00:06Z",
        } satisfies ChatMessageResponse,
      ],
    });
    const user = userEvent.setup();

    renderWithQueryClient(
      <TaskSubagentCard
        task={makeStreamingTask({
          toolName: "delegate_start",
          description: "Review delegated patch",
          subagentType: "delegated",
          model: "gpt-5.4",
          delegatedConversationId: "child-conv-1",
          textOutput: "Delegated review finished",
        })}
      />,
    );

    await user.click(
      screen.getByRole("button", { name: /delegated subagent: review delegated patch/i }),
    );

    await waitFor(() => expect(getConversationSpy).toHaveBeenCalledWith("child-conv-1"));
    expect(await screen.findByText("Delegated conversation")).toBeInTheDocument();
    expect(screen.getByText("Please inspect the patch")).toBeInTheDocument();
    expect(screen.getByText("Review complete with no blockers")).toBeInTheDocument();
  });
});
