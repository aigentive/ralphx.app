/**
 * useGlobalAgentLifecycle — Always-on global hook for agent lifecycle status tracking.
 *
 * Handles agent lifecycle events (run_started, run_completed, turn_completed, stopped, error)
 * and updates chatStore.agentStatus globally, ensuring sidebar PlanItems show status for ALL
 * sessions regardless of which chat panel is currently mounted.
 *
 * Mounted in GlobalEventListeners (EventProvider) — not per-panel.
 *
 * Does NOT manage:
 * - General query cache (per-panel hook's responsibility — requires activeConversationId)
 * - setActiveConversation (requires per-panel storeKey context)
 * - Queue processing (backend-managed, per-panel hook handles UI)
 *
 * EXCEPTION: verification query cache invalidation IS included in handleChildTerminationReverseLink
 * because it uses session ID from event payload, not activeConversationId.
 */

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useEventBus } from "@/providers/EventProvider";
import { useChatStore } from "@/stores/chatStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { useUiStore } from "@/stores/uiStore";
import { useTeamStore } from "@/stores/teamStore";
import { buildStoreKey, parseStoreKey } from "@/lib/chat-context-registry";
import { findStoreKeyForContextId } from "@/lib/agent-event-utils";
import type { ContextType } from "@/types/chat-conversation";
import type { Unsubscribe } from "@/lib/event-bus";
import { logger } from "@/lib/logger";

export function useGlobalAgentLifecycle() {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Reverse lookup: when a verification child session terminates, find any parent that has
    // it as activeVerificationChildId and clean up parent's synthetic generating state.
    // Scoped exception: includes verification query cache invalidation because it uses
    // session ID from event payload, not activeConversationId.
    function handleChildTerminationReverseLink(eventContextId: string) {
      const ideationState = useIdeationStore.getState();
      const chatState = useChatStore.getState();
      for (const [parentSessionId, childId] of Object.entries(ideationState.activeVerificationChildId)) {
        if (childId !== null && childId === eventContextId) {
          ideationState.setActiveVerificationChildId(parentSessionId, null);
          chatState.setAgentStatus(buildStoreKey("ideation", parentSessionId), "idle");
          const verificationData = queryClient.getQueryData<{ inProgress?: boolean }>([
            "verification",
            parentSessionId,
          ]);
          if (verificationData?.inProgress) {
            logger.warn(
              `[GlobalAgentLifecycle] Child session ${eventContextId} terminated while verification still in_progress for parent ${parentSessionId} — invalidating verification cache`
            );
            queryClient.invalidateQueries({ queryKey: ["verification", parentSessionId] });
          }
        }
      }
    }

    // Guard: if the parent session has an active verification child, re-assert `generating`
    // instead of clearing to `idle`. Parent's generating state is synthetic — reflects
    // the child session running. Normal termination events must not clear it prematurely.
    function guardedTermination(
      storeKey: string,
      eventContextId: string,
      contextType: string
    ) {
      const parsed = parseStoreKey(storeKey);
      if (parsed?.contextType === "ideation") {
        const activeChildId =
          useIdeationStore.getState().activeVerificationChildId[parsed.contextId];
        if (activeChildId) {
          useChatStore.getState().setAgentStatus(storeKey, "generating");
          return;
        }
      }

      useChatStore.getState().setAgentStatus(storeKey, "idle");

      // Scope guards for cleanup calls:
      // clearActiveQuestion: ideation contexts only (semantically incorrect otherwise)
      if (contextType === "ideation") {
        useUiStore.getState().clearActiveQuestion(eventContextId);
      }

      // clearPendingPlan: team mode active only (no ghost approval banners)
      const chatState = useChatStore.getState();
      if (chatState.isTeamActive?.[storeKey]) {
        useTeamStore.getState().clearPendingPlan(storeKey);
      }
    }

    // agent:run_started → setAgentStatus generating
    // Skip teammate events (handled by useTeamEvents)
    unsubscribes.push(
      bus.subscribe<{
        run_id: string;
        context_type: string;
        context_id: string;
        conversation_id: string;
        teammate_name?: string | null;
      }>("agent:run_started", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        // Guard: only update watchdog on initial spawn, not queue re-runs.
        // Queue re-runs emit run_started while already in "generating" state —
        // resetting here would mask a real stuck-generating condition.
        const currentStatus = useChatStore.getState().agentStatus[eventContextKey];
        if (currentStatus !== "generating") {
          useChatStore.getState().updateLastAgentEvent(eventContextKey);
        }

        useChatStore.getState().setAgentStatus(eventContextKey, "generating");
      })
    );

    // agent:run_completed → guarded termination
    // Skip teammate events
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        status: string;
        teammate_name?: string | null;
      }>("agent:run_completed", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        // Final heartbeat before transitioning to idle
        useChatStore.getState().updateLastAgentEvent(eventContextKey);

        guardedTermination(eventContextKey, eventContextId, context_type);
        handleChildTerminationReverseLink(eventContextId);
      })
    );

    // agent:turn_completed → waiting_for_input (with verification child guard)
    // Skip teammate events
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        status: string;
        teammate_name?: string | null;
      }>("agent:turn_completed", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        // Heartbeat: agent alive between turns
        useChatStore.getState().updateLastAgentEvent(eventContextKey);

        // Guard: if parent ideation session has active verification child, maintain
        // generating instead of transitioning to waiting_for_input
        const parsedKey = parseStoreKey(eventContextKey);
        if (parsedKey?.contextType === "ideation") {
          const activeChildId =
            useIdeationStore.getState().activeVerificationChildId[parsedKey.contextId];
          if (activeChildId) {
            useChatStore.getState().setAgentStatus(eventContextKey, "generating");
          } else {
            useChatStore.getState().setAgentStatus(eventContextKey, "waiting_for_input");
          }
        } else {
          useChatStore.getState().setAgentStatus(eventContextKey, "waiting_for_input");
        }
      })
    );

    // agent:stopped → guarded termination
    // Skip teammate events
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        agent_run_id: string;
        teammate_name?: string | null;
      }>("agent:stopped", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        guardedTermination(eventContextKey, eventContextId, context_type);
        handleChildTerminationReverseLink(eventContextId);
      })
    );

    // agent:error → guarded termination + error toast for execution contexts
    // Skip teammate events
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        error: string;
        teammate_name?: string | null;
      }>("agent:error", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        guardedTermination(eventContextKey, eventContextId, context_type);
        handleChildTerminationReverseLink(eventContextId);

        // Error toast for execution contexts with deterministic id for deduplication.
        // Sonner does NOT auto-deduplicate — explicit id prevents duplicate toasts
        // when both global and per-panel hooks are mounted simultaneously.
        if (["task_execution", "review", "merge"].includes(context_type)) {
          const contextLabel =
            context_type === "task_execution"
              ? "Worker"
              : context_type === "review"
                ? "Reviewer"
                : "Merger";
          const errorMsg = payload.error
            ? String(payload.error).slice(0, 150)
            : "Agent process exited unexpectedly";
          toast.error(`${contextLabel} agent error: ${errorMsg}`, {
            id: `error:${eventContextKey}`,
            duration: 8000,
          });
        }
      })
    );

    // agent:heartbeat — no context_type in payload, use findStoreKeyForContextId scan
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        context_id: string;
        reason: string;
        pid?: number;
      }>("agent:heartbeat", (payload) => {
        const key = findStoreKeyForContextId(payload.context_id);
        if (key) useChatStore.getState().updateLastAgentEvent(key);
      })
    );

    // agent:task_started — context_type available, use buildStoreKey directly
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        context_id: string;
        context_type?: string;
      }>("agent:task_started", (payload) => {
        // Prefer buildStoreKey when context_type available; fall back to scan
        if (payload.context_type) {
          const key = buildStoreKey(payload.context_type as ContextType, payload.context_id);
          useChatStore.getState().updateLastAgentEvent(key);
        } else {
          const key = findStoreKeyForContextId(payload.context_id);
          if (key) useChatStore.getState().updateLastAgentEvent(key);
        }
      })
    );

    // agent:task_completed — context_type available, use buildStoreKey directly
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        context_id: string;
        context_type?: string;
      }>("agent:task_completed", (payload) => {
        if (payload.context_type) {
          const key = buildStoreKey(payload.context_type as ContextType, payload.context_id);
          useChatStore.getState().updateLastAgentEvent(key);
        } else {
          const key = findStoreKeyForContextId(payload.context_id);
          if (key) useChatStore.getState().updateLastAgentEvent(key);
        }
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, queryClient]);
}
