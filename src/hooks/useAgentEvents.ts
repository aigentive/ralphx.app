/**
 * useAgentEvents hook - Event listener management for agent lifecycle events
 *
 * Handles real-time updates for agent runs across all contexts (ideation, task, review, project).
 * Listens to unified agent:* events and updates query cache and store state accordingly.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect, useLayoutEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useEventBus } from "@/providers/EventProvider";
import type { ChatMessageResponse } from "@/api/chat";
import type { ChatConversation, ContextType } from "@/types/chat-conversation";
import { useChatStore } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import { useTeamStore } from "@/stores/teamStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import { chatKeys } from "./useChat";
import type { Unsubscribe } from "@/lib/event-bus";

/**
 * Hook to manage agent event listeners
 *
 * Subscribes to Tauri events for real-time updates of agent runs.
 * Uses unified agent:* events (Phase 5-6 consolidation).
 *
 * @param activeConversationId - The currently active conversation ID to filter events
 * @param storeKey - Caller-provided store key for scoped setActiveConversation writes.
 *   When provided, agent:run_started uses this key instead of the event-derived key.
 *   Callers know which panel slot to write to; cross-context events are handled
 *   separately by IntegratedChatPanel's own bus.subscribe handler.
 */
export function useAgentEvents(activeConversationId: string | null, storeKey?: string) {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  // Ref for storeKey — prevents stale closure writes during teardown/resubscribe window.
  // useLayoutEffect keeps ref synchronised before any Tauri IPC events can arrive.
  const storeKeyRef = useRef(storeKey);
  useLayoutEffect(() => {
    storeKeyRef.current = storeKey;
  }, [storeKey]);
  const setAgentStatus = useChatStore((s) => s.setAgentStatus);
  const updateLastAgentEvent = useChatStore((s) => s.updateLastAgentEvent);
  const deleteQueuedMessage = useChatStore((s) => s.deleteQueuedMessage);
  const queueMessage = useChatStore((s) => s.queueMessage);
  const setActiveConversation = useChatStore((s) => s.setActiveConversation);
  const clearActiveQuestion = useUiStore((s) => s.clearActiveQuestion);
  const clearPendingPlan = useTeamStore((s) => s.clearPendingPlan);

  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // NOTE: Streaming cache updates disabled per user request.
    // Instead of trying to stream text/tool calls character-by-character,
    // we show a typing indicator while the agent is running and only
    // render the final message with proper content_blocks when the run completes.
    //
    // The agent:chunk and agent:tool_call events are still emitted by the backend
    // but we don't use them to update the UI during streaming. This avoids
    // issues with mismatched tool calls/results and partial content.

    // Listen for run started - set agent running state to true and update conversation cache
    // Skip teammate events — useTeamEvents handles those independently
    unsubscribes.push(
      bus.subscribe<{
        run_id: string;
        context_type: string;
        context_id: string;
        conversation_id: string;
        teammate_name?: string | null;
      }>("agent:run_started", (payload) => {
        if (payload.teammate_name) return;
        const { context_type, context_id: eventContextId, conversation_id } = payload;

        // Build context key from the event payload
        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        // Update watchdog timestamp only for initial spawns, not queue re-runs.
        // Queue re-runs emit run_started while already in "generating" state —
        // resetting the timestamp there would mask a real stuck-generating condition.
        const currentStatus = useChatStore.getState().agentStatus[eventContextKey];
        if (currentStatus !== "generating") {
          updateLastAgentEvent(eventContextKey);
        }

        // Set agent as generating for this context
        setAgentStatus(eventContextKey, "generating");

        // Invalidate conversations list to pick up newly created conversation
        // This fixes the race condition where the list query runs before the backend
        // creates the conversation (e.g., when task enters "reviewing" state)
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversationList(context_type as ContextType, eventContextId),
        });

        // If no active conversation is set, set it to this one
        // This handles the case where a new conversation was just created by the backend.
        // Use caller-provided storeKey when available — the caller knows which panel slot to
        // write to. Cross-context events are handled by IntegratedChatPanel's bus.subscribe.
        if (!activeConversationId && conversation_id) {
          setActiveConversation(storeKeyRef.current ?? eventContextKey, conversation_id);
        }
      })
    );

    // Listen for message created - optimistically add to cache for user messages only
    // Unified event: agent:message_created (replaces chat:message_created)
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        message_id: string;
        role: string;
        content: string;
        created_at?: string;
        metadata?: string | null;
      }>("agent:message_created", (payload) => {
        const { conversation_id, message_id, role, content, created_at } = payload;

        // Heartbeat: update watchdog timestamp on every message (active event flow).
        // Prevents watchdog from firing during normal streaming bursts.
        const msgContextKey = buildStoreKey(payload.context_type as ContextType, payload.context_id);
        updateLastAgentEvent(msgContextKey);

        // Always invalidate the conversation query for this message's conversation.
        // This handles both lead and teammate conversations — teammate messages
        // have their own conversation_id that won't match activeConversationId.
        if (role === "user" && conversation_id === activeConversationId) {
          // Optimistic append for user messages in the active conversation only
          queryClient.setQueryData<{ conversation: ChatConversation; messages: ChatMessageResponse[] }>(
            chatKeys.conversation(activeConversationId),
            (oldData) => {
              if (!oldData) return oldData;

              // Check if message already exists
              if (oldData.messages.some(m => m.id === message_id)) {
                return oldData;
              }

              const newMessage: ChatMessageResponse = {
                id: message_id,
                conversationId: conversation_id,
                sessionId: null,
                projectId: null,
                taskId: null,
                role: role as "user" | "assistant" | "system",
                content: content || "",
                metadata: payload.metadata ?? null,
                parentMessageId: null,
                createdAt: created_at ?? new Date().toISOString(),
                toolCalls: null,
                contentBlocks: null,
                sender: null,
              };
              return { ...oldData, messages: [...oldData.messages, newMessage] };
            }
          );
        } else if (conversation_id !== activeConversationId) {
          // Non-active conversation (e.g. teammate messages): invalidate to refetch from DB.
          // Active-conversation assistant messages are handled exclusively by useChatEvents
          // to avoid duplicate DB refetches that cause visual artifacts during streaming.
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id), // use payload ID, not stale closure
          });
        }
      })
    );

    // Listen for run completion
    // Unified event: agent:run_completed (replaces chat:run_completed)
    // Skip teammate events — useTeamEvents handles those independently
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        status: string;
        teammate_name?: string | null;
      }>("agent:run_completed", (payload) => {
        if (payload.teammate_name) return;
        const { conversation_id, context_type, context_id: eventContextId } = payload;

        // Build context key from the event payload
        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        // Final heartbeat — clears the "stuck" condition before transitioning to idle.
        updateLastAgentEvent(eventContextKey);

        // Clear agent status for the specific context (run is done)
        setAgentStatus(eventContextKey, "idle");

        // Clean up any pending questions for this session — the agent is gone,
        // so questions can no longer be answered.
        clearActiveQuestion(eventContextId);

        // Clear any pending team plan — the lead agent is gone so the plan
        // can no longer be approved or rejected.
        clearPendingPlan(eventContextKey);

        // Invalidate using conversation_id from the payload — avoids stale closure mismatch
        // where activeConversationId in the closure might differ from the just-completed run.
        queryClient.invalidateQueries({
          queryKey: chatKeys.agentRun(conversation_id),
        });
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversation_id),
        });

        // NOTE: Queue processing is now handled by the BACKEND
        // The backend automatically processes queued messages via --resume
        // when a run completes. We listen for agent:queue_sent to update UI.
      })
    );

    // Listen for turn completion (interactive mode - agent still alive)
    // Sets status to "waiting_for_input" so the UI shows the agent is idle between turns
    // (not "generating"), while the process remains alive.
    // Skip teammate events — useTeamEvents handles those independently
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        status: string;
        teammate_name?: string | null;
      }>("agent:turn_completed", (payload) => {
        if (payload.teammate_name) return;
        const { conversation_id, context_type, context_id: eventContextId } = payload;

        // Agent is still alive but waiting for user input — transition from "generating" to "waiting_for_input"
        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        // Heartbeat: agent is alive between turns, reset watchdog timer.
        updateLastAgentEvent(eventContextKey);

        setAgentStatus(eventContextKey, "waiting_for_input");

        // Invalidate using conversation_id from payload to avoid stale closure mismatch
        queryClient.invalidateQueries({
          queryKey: chatKeys.agentRun(conversation_id),
        });
        // Active-conversation assistant turns already invalidate/refetch via
        // agent:message_created in useChatEvents. Skipping the second active
        // conversation invalidation avoids overlapping layout/scroll churn
        // during finalization while preserving refetches for non-active tabs.
        if (conversation_id !== activeConversationId) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      })
    );

    // Listen for queue_sent - backend notifies us when it sends a queued message
    // This allows us to update the optimistic UI by removing the sent message
    // Since frontend and backend use the same ID, we can match exactly by ID
    unsubscribes.push(
      bus.subscribe<{
        message_id: string;
        conversation_id: string;
        context_type: string;
        context_id: string;
      }>("agent:queue_sent", (payload) => {
        const { message_id, context_type, context_id: eventContextId } = payload;

        // Build context key from the event payload - unified queue with context-aware keys
        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);
        // Remove from frontend optimistic queue by exact ID match
        deleteQueuedMessage(eventContextKey, message_id);
      })
    );

    // Listen for message_queued - backend notifies us when a message enters the queue (Gate 2)
    // Idempotent: queueMessage has a duplicate-ID guard, so calling it twice with the same ID is safe
    unsubscribes.push(
      bus.subscribe<{
        message_id: string;
        content: string;
        context_type: string;
        context_id: string;
        created_at: string;
      }>("agent:message_queued", (payload) => {
        const { message_id, content, context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);
        queueMessage(eventContextKey, content, message_id);
      })
    );

    // Listen for agent stopped - defensive cleanup if agent:run_completed emission regresses.
    // Backend emits agent:stopped immediately on SIGTERM, before agent:run_completed.
    // This ensures running state clears even if the subsequent run_completed is lost.
    // Skip teammate events — useTeamEvents handles those independently
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        agent_run_id: string;
        teammate_name?: string | null;
      }>("agent:stopped", (payload) => {
        if (payload.teammate_name) return;
        const { conversation_id, context_type, context_id: eventContextId } = payload;

        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        setAgentStatus(eventContextKey, "idle");

        // Clean up any pending questions — agent is being killed
        clearActiveQuestion(eventContextId);

        // Clear any pending team plan — the lead agent is gone so the plan
        // can no longer be approved or rejected.
        clearPendingPlan(eventContextKey);

        // Invalidate using conversation_id from payload to avoid stale closure mismatch
        queryClient.invalidateQueries({
          queryKey: chatKeys.agentRun(conversation_id),
        });
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversation_id),
        });
      })
    );

    // Listen for agent errors
    // Unified event: agent:error
    // Skip teammate events — useTeamEvents handles those independently
    unsubscribes.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        error: string;
        teammate_name?: string | null;
      }>("agent:error", (payload) => {
        if (payload.teammate_name) return;
        const { conversation_id, context_type, context_id: eventContextId } = payload;

        // Build context key from the event payload
        const eventContextKey = buildStoreKey(context_type as ContextType, eventContextId);

        // Clear agent status on error for the specific context
        setAgentStatus(eventContextKey, "idle");

        // Clean up any pending questions — agent errored out
        clearActiveQuestion(eventContextId);

        // Clear any pending team plan — the lead agent is gone so the plan
        // can no longer be approved or rejected.
        clearPendingPlan(eventContextKey);

        // Invalidate using conversation_id from payload to avoid stale closure mismatch
        queryClient.invalidateQueries({
          queryKey: chatKeys.agentRun(conversation_id),
        });
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversation_id),
        });

        // Show error toast for agent failures in execution contexts
        if (["task_execution", "review", "merge"].includes(context_type)) {
          const contextLabel = context_type === "task_execution" ? "Worker"
            : context_type === "review" ? "Reviewer"
            : "Merger";
          const errorMsg = payload.error ? String(payload.error).slice(0, 150) : "Agent process exited unexpectedly";
          toast.error(`${contextLabel} agent error: ${errorMsg}`, { duration: 8000 });
        }
      })
    );

    // Listen for session recovery events
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        message: string;
      }>("agent:session_recovered", (payload) => {
        console.info("[session-recovery]", payload.message);
      })
    );

    // Listen for synthetic heartbeat events emitted by backend during PID-alive bypass.
    // Refreshes lastAgentEventTimestamp so the frontend watchdog doesn't false-trigger
    // while the backend keeps the agent alive during buffered-stdout commands.
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        context_id: string;
        reason: string;
        pid?: number;
      }>("agent:heartbeat", (payload) => {
        // Find the store key by scanning all generating contexts for a matching conversation
        // The heartbeat payload has context_id, so we look up which context keys are generating
        // and match by context_id substring (context_id is the raw id, key is "type:id")
        const state = useChatStore.getState();
        for (const key of Object.keys(state.agentStatus)) {
          if (key.includes(payload.context_id)) {
            updateLastAgentEvent(key);
            break;
          }
        }
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, activeConversationId, storeKey, queryClient, setAgentStatus, updateLastAgentEvent, deleteQueuedMessage, queueMessage, setActiveConversation, clearActiveQuestion, clearPendingPlan]);

  // Global singleton watchdog — defense-in-depth for stuck generating state.
  // If the backend misses run_completed for any reason, this forces idle after
  // 5 minutes of no agent events for a context still in "generating" state.
  // Runs once per hook mount (empty deps) and checks all contexts every 30s.
  useEffect(() => {
    const WATCHDOG_TIMEOUT_MS = 300_000; // 5 minutes
    const CHECK_INTERVAL_MS = 30_000;    // Check every 30s

    const interval = setInterval(() => {
      const now = Date.now();
      const state = useChatStore.getState();

      for (const [key, status] of Object.entries(state.agentStatus)) {
        if (status !== "generating") continue;
        const lastEvent = state.lastAgentEventTimestamp[key] ?? 0;
        if (now - lastEvent > WATCHDOG_TIMEOUT_MS) {
          toast.warning('Agent appears to have stalled. Status reset to idle.');
          state.setAgentStatus(key, "idle");
        }
      }
    }, CHECK_INTERVAL_MS);

    return () => clearInterval(interval);
  }, []); // Empty deps — runs once globally, reads fresh state via getState()
}
