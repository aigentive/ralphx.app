import { useQuery } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import {
  sortAgentConversations,
  toProjectAgentConversation,
  type AgentConversation,
} from "./agentConversations";

export const agentConversationKeys = {
  all: ["agents", "project-conversations"] as const,
  project: (projectId: string) => [...agentConversationKeys.all, projectId] as const,
  projectList: (projectId: string, includeArchived: boolean) =>
    [...agentConversationKeys.project(projectId), "archived", includeArchived] as const,
};

export function useProjectAgentConversations(
  projectId: string | null | undefined,
  includeArchived = false
) {
  return useQuery<AgentConversation[]>({
    queryKey: agentConversationKeys.projectList(projectId ?? "", includeArchived),
    queryFn: async () => {
      const targetProjectId = projectId ?? "";
      const projectConversations = await chatApi.listConversations(
        "project",
        targetProjectId,
        includeArchived
      );

      return sortAgentConversations([
        ...projectConversations.map(toProjectAgentConversation),
      ]);
    },
    enabled: Boolean(projectId),
    staleTime: 5_000,
  });
}
