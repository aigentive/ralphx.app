import { useQuery } from "@tanstack/react-query";
import { chatApi, type ConversationStatsResponse } from "@/api/chat";

export const conversationStatsKey = (conversationId: string) =>
  ["chat", "conversation-stats", conversationId] as const;

export function useConversationStats(conversationId: string | null) {
  return useQuery<ConversationStatsResponse | null, Error>({
    queryKey: conversationId ? conversationStatsKey(conversationId) : ["chat", "conversation-stats", "none"],
    queryFn: () => {
      if (!conversationId) {
        return Promise.resolve(null);
      }
      return chatApi.getConversationStats(conversationId);
    },
    enabled: !!conversationId,
    staleTime: 15_000,
  });
}
