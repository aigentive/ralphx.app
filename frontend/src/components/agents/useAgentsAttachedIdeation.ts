import { useEffect, useMemo, useRef } from "react";
import { useQuery } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import type {
  AgentConversationWorkspace,
  AgentConversationWorkspaceMode,
  ChatMessageResponse,
} from "@/api/chat";
import { ideationApi } from "@/api/ideation";
import { ideationKeys } from "@/hooks/useIdeation";

import type { AgentConversation } from "./agentConversations";
import { resolveAttachedIdeationSessionId } from "./attachedIdeationSession";

interface UseAgentsAttachedIdeationArgs {
  activeConversation: AgentConversation | null;
  activeConversationMode: AgentConversationWorkspaceMode | null;
  activeWorkspace: AgentConversationWorkspace | null;
  invalidateProjectConversations: (targetProjectId: string) => Promise<unknown>;
  selectedConversationMessages: ChatMessageResponse[];
}

export function useAgentsAttachedIdeation({
  activeConversation,
  activeConversationMode,
  activeWorkspace,
  invalidateProjectConversations,
  selectedConversationMessages,
}: UseAgentsAttachedIdeationArgs) {
  const childArchiveSyncRef = useRef<Set<string>>(new Set());
  const shouldHydrateAttachedIdeation =
    activeConversation?.contextType === "ideation" ||
    (activeConversation?.contextType === "project" &&
      (activeConversationMode === "ideation" ||
        Boolean(activeWorkspace?.linkedIdeationSessionId || activeWorkspace?.linkedPlanBranchId)));
  const attachedIdeationSessionId = useMemo(
    () =>
      shouldHydrateAttachedIdeation
        ? resolveAttachedIdeationSessionId(activeConversation, selectedConversationMessages)
        : null,
    [activeConversation, selectedConversationMessages, shouldHydrateAttachedIdeation],
  );
  const attachedIdeationSessionQuery = useQuery({
    queryKey: ideationKeys.sessionWithData(attachedIdeationSessionId ?? ""),
    queryFn: () => ideationApi.sessions.getWithData(attachedIdeationSessionId!),
    enabled: shouldHydrateAttachedIdeation && !!attachedIdeationSessionId,
    staleTime: 5_000,
  });
  const attachedIdeationSessionData =
    attachedIdeationSessionId &&
    attachedIdeationSessionQuery.data?.session.id === attachedIdeationSessionId
      ? attachedIdeationSessionQuery.data
      : null;
  const hasAutoOpenArtifacts = useMemo(() => {
    if (!attachedIdeationSessionData) {
      return false;
    }

    const session = attachedIdeationSessionData.session;
    return Boolean(
      session.planArtifactId ||
        session.inheritedPlanArtifactId ||
        session.acceptanceStatus === "pending" ||
        session.verificationInProgress ||
        session.verificationStatus !== "unverified" ||
        attachedIdeationSessionData.proposals.length > 0
    );
  }, [attachedIdeationSessionData]);
  useEffect(() => {
    if (
      activeConversation?.contextType !== "project" ||
      !attachedIdeationSessionData ||
      activeConversation.archivedAt ||
      childArchiveSyncRef.current.has(activeConversation.id)
    ) {
      return;
    }
    const session = attachedIdeationSessionData.session;
    const sessionArchived = session.status === "archived" || Boolean(session.archivedAt);
    if (!sessionArchived) {
      return;
    }
    childArchiveSyncRef.current.add(activeConversation.id);
    void chatApi.archiveConversation(activeConversation.id)
      .then(() => invalidateProjectConversations(activeConversation.projectId))
      .catch(() => {
        childArchiveSyncRef.current.delete(activeConversation.id);
        // Status sync is best-effort; manual archive remains available.
      });
  }, [
    activeConversation,
    attachedIdeationSessionData,
    invalidateProjectConversations,
  ]);
  return {
    attachedIdeationSessionData,
    attachedIdeationSessionId,
    hasAutoOpenArtifacts,
  };
}
