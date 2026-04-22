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
import { buildAgentEventStoreKey } from "@/lib/agent-store-key";
import { findStoreKeyForContextId } from "@/lib/agent-event-utils";
import type { ModelDisplay } from "@/types/chat-conversation";
import type { Unsubscribe } from "@/lib/event-bus";
import type {
  AgentRunCompletedPayload,
  AgentRunStartedPayload,
} from "@/types/events";
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
      contextType: string,
      conversationId: string
    ) {
      // Stale conversation check (matches useAgentEvents.ts:101-107).
      // Fail-open when activeConvId is null/undefined — prevents stuck generating
      // for sessions never visited by a per-panel hook.
      const activeConvId = useChatStore.getState().activeConversationIds[storeKey];
      if (activeConvId != null && conversationId !== activeConvId) {
        logger.warn(
          `[GlobalAgentLifecycle] Ignoring stale termination: conv=${conversationId} != active=${activeConvId} for key=${storeKey}`
        );
        return;
      }

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
      bus.subscribe<AgentRunStartedPayload>("agent:run_started", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildAgentEventStoreKey(
          context_type,
          eventContextId,
          payload.conversation_id
        );

        // Guard: only update watchdog on initial spawn, not queue re-runs.
        // Queue re-runs emit run_started while already in "generating" state —
        // resetting here would mask a real stuck-generating condition.
        const currentStatus = useChatStore.getState().agentStatus[eventContextKey];
        if (currentStatus !== "generating") {
          useChatStore.getState().updateLastAgentEvent(eventContextKey);
        }

        useChatStore.getState().setAgentStatus(eventContextKey, "generating");
        // Track the active conversation for this context so the stale guard can function
        // for ALL sessions, not just those with mounted per-panel hooks.
        useChatStore.getState().setActiveConversation(eventContextKey, payload.conversation_id);

        // Populate effective model if both fields are present
        const effectiveModelId = payload.effective_model_id ?? payload.effectiveModelId;
        const effectiveModelLabel =
          payload.effective_model_label ?? payload.effectiveModelLabel;
        if (effectiveModelId && effectiveModelLabel) {
          const model: ModelDisplay = {
            id: effectiveModelId,
            label: effectiveModelLabel,
          };
          useChatStore.getState().setEffectiveModel(eventContextKey, model);
        }
      })
    );

    // agent:run_completed → guarded termination
    // Skip teammate events
    unsubscribes.push(
      bus.subscribe<AgentRunCompletedPayload>("agent:run_completed", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildAgentEventStoreKey(
          context_type,
          eventContextId,
          payload.conversation_id
        );

        // Final heartbeat before transitioning to idle
        useChatStore.getState().updateLastAgentEvent(eventContextKey);

        guardedTermination(eventContextKey, eventContextId, context_type, payload.conversation_id);
        handleChildTerminationReverseLink(eventContextId);
      })
    );

    // agent:turn_completed → waiting_for_input (with verification child guard)
    // Skip teammate events
    unsubscribes.push(
      bus.subscribe<AgentRunCompletedPayload>("agent:turn_completed", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildAgentEventStoreKey(
          context_type,
          eventContextId,
          payload.conversation_id
        );

        // Heartbeat: agent alive between turns
        useChatStore.getState().updateLastAgentEvent(eventContextKey);

        // Stale conversation check MUST run before verification child guard.
        // A stale turn_completed from an old conversation must not trigger the
        // re-assert generating path for a session that should be idle.
        const activeConvIdForTurn = useChatStore.getState().activeConversationIds[eventContextKey];
        if (activeConvIdForTurn != null && payload.conversation_id !== activeConvIdForTurn) {
          logger.warn(
            `[GlobalAgentLifecycle] Ignoring stale turn_completed: conv=${payload.conversation_id} != active=${activeConvIdForTurn} for key=${eventContextKey}`
          );
          return;
        }

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

        const eventContextKey = buildAgentEventStoreKey(
          context_type,
          eventContextId,
          payload.conversation_id
        );

        guardedTermination(eventContextKey, eventContextId, context_type, payload.conversation_id);
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

        const eventContextKey = buildAgentEventStoreKey(
          context_type,
          eventContextId,
          payload.conversation_id
        );

        guardedTermination(eventContextKey, eventContextId, context_type, payload.conversation_id);
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

    // agent:conversation_created → track new conversations for the stale guard
    // Only sets activeConversationIds when no entry exists — avoids poisoning the guard
    // if conversation_created fires but run_started never follows (e.g., spawn failure).
    // NOTE: AgentConversationCreatedPayload does not include teammate_name (it's only emitted
    // for primary agent conversations), so no teammate filter is needed here.
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        context_type: string;
        context_id: string;
      }>("agent:conversation_created", (payload) => {
        const key = buildAgentEventStoreKey(
          payload.context_type,
          payload.context_id,
          payload.conversation_id
        );
        const existing = useChatStore.getState().activeConversationIds[key];
        if (existing == null) {
          useChatStore.getState().setActiveConversation(key, payload.conversation_id);
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
        // Prefer typed event keys when context_type is available; fall back to scan
        if (payload.context_type) {
          const key = buildAgentEventStoreKey(
            payload.context_type,
            payload.context_id,
            payload.conversation_id
          );
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
          const key = buildAgentEventStoreKey(
            payload.context_type,
            payload.context_id,
            payload.conversation_id
          );
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
