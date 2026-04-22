import { useQuery } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import { ideationApi } from "@/api/ideation";
import {
  sortAgentConversations,
  toIdeationAgentConversation,
  toProjectAgentConversation,
  type AgentConversation,
  type AgentIdeationSession,
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
      const [projectConversations, activeSessions, archivedSessionList] =
        await Promise.all([
          chatApi.listConversations("project", targetProjectId, includeArchived),
          ideationApi.sessions.list(targetProjectId),
          includeArchived
            ? ideationApi.sessions.listByGroup(targetProjectId, "archived")
            : Promise.resolve(null),
        ]);

      const sessions = new Map<string, AgentIdeationSession>(
        activeSessions.map((session) => [session.id, session])
      );
      for (const session of archivedSessionList?.sessions ?? []) {
        sessions.set(session.id, session);
      }

      const ideationConversations = (
        await Promise.all(
          Array.from(sessions.values()).map(async (session) => {
            const conversations = await chatApi.listConversations(
              "ideation",
              session.id,
              includeArchived
            );
            const primaryConversation = sortAgentConversations(
              conversations.map((conversation) =>
                toIdeationAgentConversation(session, conversation)
              )
            )[0];
            return primaryConversation ?? null;
          })
        )
      ).filter((conversation): conversation is AgentConversation => Boolean(conversation));

      return sortAgentConversations([
        ...projectConversations.map(toProjectAgentConversation),
        ...ideationConversations,
      ]);
    },
    enabled: Boolean(projectId),
    staleTime: 5_000,
  });
}
