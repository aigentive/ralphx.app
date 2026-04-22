import { useQuery } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import { chatKeys } from "@/hooks/useChat";

export function useProjectAgentConversations(
  projectId: string | null | undefined,
  includeArchived = false
) {
  return useQuery({
    queryKey: [...chatKeys.conversationList("project", projectId ?? ""), "archived", includeArchived],
    queryFn: () => chatApi.listConversations("project", projectId ?? "", includeArchived),
    enabled: Boolean(projectId),
    staleTime: 5_000,
  });
}
