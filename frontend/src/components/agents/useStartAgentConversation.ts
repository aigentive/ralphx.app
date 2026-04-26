import { useCallback } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { QueryClient } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import type {
  AgentConversationBaseSelection,
  AgentConversationWorkspace,
  AgentConversationWorkspaceMode,
} from "@/api/chat";
import { chatKeys, invalidateConversationDataQueries } from "@/hooks/useChat";
import type { AgentRuntimeSelection } from "@/stores/agentSessionStore";

import {
  getAgentConversationStoreKey,
  toProjectAgentConversation,
  type AgentConversation,
} from "./agentConversations";
import { normalizeRuntimeSelection } from "./agentOptions";
import { uploadDraftAttachment } from "./chatAttachmentUpload";

interface HandleAutoManagedTitleArgs {
  content: string;
  conversationId: string;
  targetProjectId: string;
  shouldSpawnSessionNamer: boolean;
}

interface UseStartAgentConversationArgs {
  handleAutoManagedTitle: (args: HandleAutoManagedTitleArgs) => void;
  invalidateProjectConversations: (targetProjectId: string) => Promise<unknown>;
  queryClient: QueryClient;
  selectConversation: (projectId: string, conversationId: string) => void;
  setActiveConversation: (contextKey: string, conversationId: string) => void;
  setFocusedProject: (projectId: string | null) => void;
  setOptimisticConversationsById: Dispatch<SetStateAction<Record<string, AgentConversation>>>;
  setOptimisticSelectedConversationId: Dispatch<SetStateAction<string | null>>;
  setOptimisticWorkspacesByConversationId: Dispatch<
    SetStateAction<Record<string, AgentConversationWorkspace>>
  >;
  setRuntimeForConversation: (
    conversationId: string,
    projectId: string,
    runtime: AgentRuntimeSelection
  ) => void;
}

export function useStartAgentConversation({
  handleAutoManagedTitle,
  invalidateProjectConversations,
  queryClient,
  selectConversation,
  setActiveConversation,
  setFocusedProject,
  setOptimisticConversationsById,
  setOptimisticSelectedConversationId,
  setOptimisticWorkspacesByConversationId,
  setRuntimeForConversation,
}: UseStartAgentConversationArgs) {
  const handleStartAgentConversation = useCallback(
    async ({
      projectId: targetProjectId,
      content,
      runtime,
      mode,
      base,
      files,
    }: {
      projectId: string;
      content: string;
      runtime: AgentRuntimeSelection;
      mode: AgentConversationWorkspaceMode;
      base: AgentConversationBaseSelection | null;
      files: File[];
    }) => {
      const normalizedRuntime = normalizeRuntimeSelection(runtime);
      let conversationIdOverride: string | null = null;

      if (files.length > 0) {
        const createdConversation = await chatApi.createConversation("project", targetProjectId);
        conversationIdOverride = createdConversation.id;
        await Promise.all(
          files.map((file) => uploadDraftAttachment(createdConversation.id, file))
        );
      }

      const result = await chatApi.startAgentConversation({
        projectId: targetProjectId,
        content,
        ...(conversationIdOverride ? { conversationId: conversationIdOverride } : {}),
        providerHarness: normalizedRuntime.provider,
        modelId: normalizedRuntime.modelId,
        mode,
        ...(base ? { base } : {}),
      });
      const resolvedConversationId = result.conversation.id;
      const optimisticConversation = toProjectAgentConversation(result.conversation);

      setOptimisticConversationsById((current) => ({
        ...current,
        [resolvedConversationId]: optimisticConversation,
      }));
      const optimisticWorkspace = result.workspace;
      if (optimisticWorkspace) {
        setOptimisticWorkspacesByConversationId((current) => ({
          ...current,
          [resolvedConversationId]: optimisticWorkspace,
        }));
      }
      queryClient.setQueryData(chatKeys.conversation(resolvedConversationId), {
        conversation: result.conversation,
        messages: [],
      });
      queryClient.setQueryData(
        ["agents", "conversation-workspace", resolvedConversationId],
        optimisticWorkspace ?? null,
      );
      setOptimisticSelectedConversationId(resolvedConversationId);
      setFocusedProject(targetProjectId);
      setRuntimeForConversation(resolvedConversationId, targetProjectId, normalizedRuntime);
      selectConversation(targetProjectId, resolvedConversationId);
      setActiveConversation(
        getAgentConversationStoreKey({
          id: resolvedConversationId,
          contextType: "project",
          contextId: targetProjectId,
        }),
        resolvedConversationId
      );
      invalidateConversationDataQueries(queryClient, resolvedConversationId);
      await invalidateProjectConversations(targetProjectId);
      handleAutoManagedTitle({
        content,
        conversationId: resolvedConversationId,
        targetProjectId,
        shouldSpawnSessionNamer: true,
      });
    },
    [
      handleAutoManagedTitle,
      invalidateProjectConversations,
      queryClient,
      selectConversation,
      setActiveConversation,
      setFocusedProject,
      setOptimisticConversationsById,
      setOptimisticSelectedConversationId,
      setOptimisticWorkspacesByConversationId,
      setRuntimeForConversation,
    ]
  );

  return handleStartAgentConversation;
}
