import type {
  AgentConversationWorkspace,
  AgentConversationWorkspaceFreshness,
} from "@/api/chat";

export function hasPublishedWorkspacePr(
  workspace: AgentConversationWorkspace | null
): boolean {
  return Boolean(workspace?.publicationPrNumber ?? workspace?.publicationPrUrl);
}

export function isAgentWorkspacePublishCurrent(
  workspace: AgentConversationWorkspace | null,
  freshness: AgentConversationWorkspaceFreshness | undefined
): boolean {
  return (
    hasPublishedWorkspacePr(workspace) &&
    workspace?.publicationPushStatus === "pushed" &&
    freshness !== undefined &&
    !freshness.isBaseAhead &&
    !freshness.hasUncommittedChanges &&
    freshness.unpublishedCommitCount === 0
  );
}
