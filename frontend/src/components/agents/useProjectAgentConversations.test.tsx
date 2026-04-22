import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ChatConversation } from "@/types/chat-conversation";
import { useProjectAgentConversations } from "./useProjectAgentConversations";

const { listConversations, listIdeationSessions, listArchivedIdeationSessions } =
  vi.hoisted(() => ({
    listConversations: vi.fn(),
    listIdeationSessions: vi.fn(),
    listArchivedIdeationSessions: vi.fn(),
  }));

vi.mock("@/api/chat", () => ({
  chatApi: {
    listConversations,
  },
}));

vi.mock("@/api/ideation", () => ({
  ideationApi: {
    sessions: {
      list: listIdeationSessions,
      listByGroup: listArchivedIdeationSessions,
    },
  },
}));

const conversation = (
  overrides: Partial<ChatConversation> = {}
): ChatConversation => ({
  id: "conversation-1",
  contextType: "project",
  contextId: "project-1",
  claudeSessionId: null,
  providerSessionId: "thread-1",
  providerHarness: "codex",
  upstreamProvider: null,
  providerProfile: null,
  title: "Project agent",
  messageCount: 1,
  lastMessageAt: "2026-04-22T12:00:00Z",
  createdAt: "2026-04-22T10:00:00Z",
  updatedAt: "2026-04-22T12:00:00Z",
  archivedAt: null,
  ...overrides,
});

function wrapper({ children }: { children: ReactNode }) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}

describe("useProjectAgentConversations", () => {
  beforeEach(() => {
    listConversations.mockReset();
    listIdeationSessions.mockReset();
    listArchivedIdeationSessions.mockReset();
  });

  it("lists only project agent conversations, not child ideation sessions", async () => {
    listConversations.mockResolvedValueOnce([
      conversation({ id: "project-conversation" }),
    ]);

    const { result } = renderHook(
      () => useProjectAgentConversations("project-1", false),
      { wrapper }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(listConversations).toHaveBeenCalledTimes(1);
    expect(listConversations).toHaveBeenCalledWith("project", "project-1", false);
    expect(listIdeationSessions).not.toHaveBeenCalled();
    expect(listArchivedIdeationSessions).not.toHaveBeenCalled();
    expect(result.current.data?.map((item) => item.id)).toEqual([
      "project-conversation",
    ]);
  });
});
