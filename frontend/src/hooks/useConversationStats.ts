import { useQuery } from "@tanstack/react-query";
import { chatApi, type ChatMessageResponse, type ConversationStatsResponse } from "@/api/chat";
import type { ChatConversation } from "@/types/chat-conversation";
import { buildFallbackConversationStats } from "@/lib/chat/conversation-stats";

export const conversationStatsKey = (conversationId: string) =>
  ["chat", "conversation-stats", conversationId] as const;

export { buildFallbackConversationStats } from "@/lib/chat/conversation-stats";

export function selectConversationStats(
  stats: ConversationStatsResponse | null | undefined,
  fallbackStats: ConversationStatsResponse | null,
): ConversationStatsResponse | null {
  if (!stats) {
    return fallbackStats;
  }

  if (
    fallbackStats
    && stats.usageCoverage.effectiveTotalsSource === "none"
    && fallbackStats.usageCoverage.effectiveTotalsSource !== "none"
  ) {
    return fallbackStats;
  }

  return stats;
}

export function useConversationStats(
  conversationId: string | null,
  options?: {
    fallbackConversation?: ChatConversation | null | undefined;
    fallbackMessages?: ChatMessageResponse[] | null | undefined;
  },
) {
  const statsQuery = useQuery<ConversationStatsResponse | null, Error>({
    queryKey: conversationId ? conversationStatsKey(conversationId) : ["chat", "conversation-stats", "none"],
    queryFn: () => {
      if (!conversationId) {
        return Promise.resolve(null);
      }
      return chatApi.getConversationStats(conversationId);
    },
    enabled: !!conversationId,
    staleTime: 0,
    refetchOnMount: "always",
    refetchOnWindowFocus: "always",
  });

  const fallbackStats = buildFallbackConversationStats(
    options?.fallbackConversation,
    options?.fallbackMessages,
  );
  const selectedStats = selectConversationStats(statsQuery.data, fallbackStats);

  return {
    ...statsQuery,
    data: selectedStats,
    isLoading: statsQuery.isLoading && !selectedStats,
  };
}
