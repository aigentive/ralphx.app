import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { TaskToolCallDelegatedTranscript } from "./TaskToolCallDelegatedTranscript";
import { createTestQueryClient } from "@/test/store-utils";
import { chatApi, type ChatMessageResponse } from "@/api/chat";

type EventHandler = (payload: unknown) => void;

const listeners = new Map<string, Set<EventHandler>>();

function mockSubscribe(event: string, handler: EventHandler) {
  if (!listeners.has(event)) {
    listeners.set(event, new Set());
  }
  listeners.get(event)!.add(handler);
  return () => {
    listeners.get(event)?.delete(handler);
  };
}

function emitEvent(event: string, payload: unknown) {
  listeners.get(event)?.forEach((handler) => handler(payload));
}

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: mockSubscribe,
    emit: vi.fn(),
  }),
}));

function renderWithQueryClient(ui: React.ReactElement) {
  const queryClient = createTestQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

afterEach(() => {
  listeners.clear();
  vi.restoreAllMocks();
});

describe("TaskToolCallDelegatedTranscript", () => {
  it("refetches when the delegated conversation receives a new message_created event", async () => {
    const getConversationSpy = vi
      .spyOn(chatApi, "getConversation")
      .mockResolvedValueOnce({
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
          messageCount: 1,
          lastMessageAt: "2026-04-12T10:00:00Z",
          createdAt: "2026-04-12T10:00:00Z",
          updatedAt: "2026-04-12T10:00:00Z",
        },
        messages: [
          {
            id: "child-msg-1",
            sessionId: null,
            projectId: null,
            taskId: null,
            role: "assistant",
            content: "First delegated update",
            metadata: null,
            parentMessageId: null,
            conversationId: "child-conv-1",
            toolCalls: null,
            contentBlocks: null,
            sender: null,
            createdAt: "2026-04-12T10:00:00Z",
          } satisfies ChatMessageResponse,
        ],
      })
      .mockResolvedValueOnce({
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
            role: "assistant",
            content: "First delegated update",
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
            content: "Second delegated update",
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

    renderWithQueryClient(
      <TaskToolCallDelegatedTranscript
        conversationId="child-conv-1"
        fallbackText="fallback"
      />,
    );

    expect(await screen.findByText("First delegated update")).toBeInTheDocument();
    expect(getConversationSpy).toHaveBeenCalledTimes(1);

    await act(async () => {
      emitEvent("agent:message_created", {
        conversation_id: "child-conv-1",
      });
    });

    await waitFor(() => expect(getConversationSpy).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("Second delegated update")).toBeInTheDocument();
  });
});
