/**
 * Shared utilities for agent event handling.
 *
 * NOTE: This file MUST NOT import from `chat-context-registry.ts`.
 * `chatStore.ts` already imports from `chat-context-registry.ts`, so importing
 * it here would create a circular dependency.
 */

import { useChatStore } from "@/stores/chatStore";

/**
 * Finds the store key for a given context ID by scanning all known agent statuses.
 * Used by heartbeat/task lifecycle events that only have the raw context_id.
 */
export function findStoreKeyForContextId(contextId: string): string | undefined {
  const state = useChatStore.getState();
  for (const key of Object.keys(state.agentStatus)) {
    if (key.includes(contextId)) {
      return key;
    }
  }
  return undefined;
}
