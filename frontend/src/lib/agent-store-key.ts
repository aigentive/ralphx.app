import { buildStoreKey } from "@/lib/chat-context-registry";
import type { ContextType } from "@/types/chat-conversation";

export function buildAgentEventStoreKey(
  contextType: ContextType | string,
  contextId: string,
  conversationId?: string | null
): string {
  const typedContext = contextType as ContextType;
  if (typedContext === "project" && conversationId) {
    return buildStoreKey("project", conversationId);
  }

  return buildStoreKey(typedContext, contextId);
}
