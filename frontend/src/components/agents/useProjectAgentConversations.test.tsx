import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ChatConversation } from "@/types/chat-conversation";
import { useProjectAgentConversations } from "./useProjectAgentConversations";

const { listConversationsPage } = vi.hoisted(() => ({
  listConversationsPage: vi.fn(),
}));

vi.mock("@/api/chat", () => ({
  chatApi: {
    listConversationsPage,
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
    listConversationsPage.mockReset();
  });

  it("fetches the first project agent page from the paginated API", async () => {
    listConversationsPage.mockResolvedValueOnce({
      conversations: [conversation({ id: "project-conversation" })],
      limit: 6,
      offset: 0,
      total: 1,
      hasMore: false,
    });

    const { result } = renderHook(
      () => useProjectAgentConversations("project-1", false),
      { wrapper }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(listConversationsPage).toHaveBeenCalledTimes(1);
    expect(listConversationsPage).toHaveBeenCalledWith(
      "project",
      "project-1",
      6,
      0,
      false,
      undefined
    );
    expect(result.current.data?.map((item) => item.id)).toEqual([
      "project-conversation",
    ]);
    expect(result.current.total).toBe(1);
  });

  it("appends pages and threads search into the server request", async () => {
    listConversationsPage
      .mockResolvedValueOnce({
        conversations: [conversation({ id: "conversation-1", title: "Fix bug" })],
        limit: 6,
        offset: 0,
        total: 2,
        hasMore: true,
      })
      .mockResolvedValueOnce({
        conversations: [conversation({ id: "conversation-2", title: "Fix bug again" })],
        limit: 6,
        offset: 1,
        total: 2,
        hasMore: false,
      });

    const { result } = renderHook(
      () =>
        useProjectAgentConversations("project-1", false, {
          search: "fix bug",
        }),
      { wrapper }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.hasNextPage).toBe(true);

    await act(async () => {
      await result.current.fetchNextPage();
    });

    await waitFor(() =>
      expect(result.current.data?.map((item) => item.id)).toEqual([
        "conversation-1",
        "conversation-2",
      ])
    );

    expect(listConversationsPage).toHaveBeenNthCalledWith(
      1,
      "project",
      "project-1",
      6,
      0,
      false,
      "fix bug"
    );
    expect(listConversationsPage).toHaveBeenNthCalledWith(
      2,
      "project",
      "project-1",
      6,
      1,
      false,
      "fix bug"
    );
  });
});
