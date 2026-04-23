import { useInfiniteQuery } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import { toProjectAgentConversation } from "./agentConversations";

export const AGENT_CONVERSATIONS_PAGE_SIZE = 6;

export const agentConversationKeys = {
  all: ["agents", "project-conversations"] as const,
  project: (projectId: string) => [...agentConversationKeys.all, projectId] as const,
  projectList: (projectId: string, includeArchived: boolean, search = "") =>
    [
      ...agentConversationKeys.project(projectId),
      "archived",
      includeArchived,
      "search",
      search.trim().toLowerCase(),
    ] as const,
};

export function useProjectAgentConversations(
  projectId: string | null | undefined,
  includeArchived = false,
  options?: { search?: string; enabled?: boolean }
) {
  const normalizedSearch = options?.search?.trim() ?? "";

  const query = useInfiniteQuery({
    queryKey: agentConversationKeys.projectList(
      projectId ?? "",
      includeArchived,
      normalizedSearch
    ),
    queryFn: async ({ pageParam = 0 }) => {
      const targetProjectId = projectId ?? "";
      const page = await chatApi.listConversationsPage(
        "project",
        targetProjectId,
        AGENT_CONVERSATIONS_PAGE_SIZE,
        pageParam,
        includeArchived,
        normalizedSearch || undefined
      );

      return {
        ...page,
        conversations: page.conversations.map(toProjectAgentConversation),
      };
    },
    getNextPageParam: (lastPage) =>
      lastPage.hasMore
        ? lastPage.offset + lastPage.conversations.length
        : undefined,
    initialPageParam: 0,
    enabled: Boolean(projectId) && (options?.enabled ?? true),
    staleTime: 5_000,
  });

  const conversations = query.data?.pages.flatMap((page) => page.conversations) ?? [];
  const total = query.data?.pages[0]?.total ?? 0;

  return {
    ...query,
    data: conversations,
    conversations,
    total,
  };
}
