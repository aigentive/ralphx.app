import { useQuery } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import { chatKeys } from "@/hooks/useChat";

export function useProjectAgentConversations(projectId: string | null | undefined) {
  return useQuery({
    queryKey: chatKeys.conversationList("project", projectId ?? ""),
    queryFn: () => chatApi.listConversations("project", projectId ?? ""),
    enabled: Boolean(projectId),
    staleTime: 5_000,
  });
}
