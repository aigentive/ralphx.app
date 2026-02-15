/**
 * useTeamEvents — Team lifecycle event consumer
 *
 * Subscribes to team:* events and agent:* events with teammate_name.
 * Routes events to teamStore actions, filtered by contextKey.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { useTeamStore } from "@/stores/teamStore";
import { useChatStore } from "@/stores/chatStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import type { ContextType } from "@/types/chat-conversation";
import type { Unsubscribe } from "@/lib/event-bus";

export function useTeamEvents(contextKey: string | null) {
  const bus = useEventBus();
  const createTeam = useTeamStore((s) => s.createTeam);
  const addTeammate = useTeamStore((s) => s.addTeammate);
  const updateTeammateStatus = useTeamStore((s) => s.updateTeammateStatus);
  const appendTeammateChunk = useTeamStore((s) => s.appendTeammateChunk);
  const clearTeammateStream = useTeamStore((s) => s.clearTeammateStream);
  const updateTeammateCost = useTeamStore((s) => s.updateTeammateCost);
  const addTeamMessage = useTeamStore((s) => s.addTeamMessage);
  const disbandTeam = useTeamStore((s) => s.disbandTeam);
  const setTeamActive = useChatStore((s) => s.setTeamActive);

  useEffect(() => {
    if (!contextKey) return;

    const unsubs: Unsubscribe[] = [];

    // Helper: build key from event payload and check match
    const matchKey = (payload: { context_type: string; context_id: string }): boolean => {
      const key = buildStoreKey(payload.context_type as ContextType, payload.context_id);
      return key === contextKey;
    };

    // team:created
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        team_name: string;
        lead_name: string;
      }>("team:created", (payload) => {
        if (matchKey(payload)) {
          createTeam(contextKey, payload.team_name, payload.lead_name);
          setTeamActive(contextKey, true);
        }
      })
    );

    // team:teammate_spawned
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        team_name: string;
        teammate_name: string;
        role_description: string;
        color: string;
        model: string;
      }>("team:teammate_spawned", (payload) => {
        if (matchKey(payload)) {
          addTeammate(contextKey, {
            name: payload.teammate_name,
            color: payload.color,
            model: payload.model,
            roleDescription: payload.role_description,
            status: "spawning",
            currentActivity: null,
            tokensUsed: 0,
            estimatedCostUsd: 0,
            streamingText: "",
          });
        }
      })
    );

    // agent:run_started — route to teammate status when teammate_name present
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        teammate_name?: string | null;
      }>("agent:run_started", (payload) => {
        if (payload.teammate_name && matchKey(payload)) {
          updateTeammateStatus(contextKey, payload.teammate_name, "running");
        }
      })
    );

    // agent:run_completed — teammate idle
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        teammate_name?: string | null;
      }>("agent:run_completed", (payload) => {
        if (payload.teammate_name && matchKey(payload)) {
          updateTeammateStatus(contextKey, payload.teammate_name, "idle");
          clearTeammateStream(contextKey, payload.teammate_name);
        }
      })
    );

    // team:teammate_idle
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        teammate_name: string;
        last_activity?: string;
      }>("team:teammate_idle", (payload) => {
        if (matchKey(payload)) {
          updateTeammateStatus(
            contextKey,
            payload.teammate_name,
            "idle",
            payload.last_activity,
          );
        }
      })
    );

    // team:message
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        from: string;
        to: string;
        content: string;
        timestamp: string;
      }>("team:message", (payload) => {
        if (matchKey(payload)) {
          addTeamMessage(contextKey, {
            id: `msg-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
            from: payload.from,
            to: payload.to,
            content: payload.content,
            timestamp: payload.timestamp,
          });
        }
      })
    );

    // team:cost_update
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        teammate_name: string;
        tokens_used: number;
        estimated_cost_usd: number;
      }>("team:cost_update", (payload) => {
        if (matchKey(payload)) {
          updateTeammateCost(
            contextKey,
            payload.teammate_name,
            payload.tokens_used,
            payload.estimated_cost_usd,
          );
        }
      })
    );

    // team:teammate_shutdown
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        teammate_name: string;
      }>("team:teammate_shutdown", (payload) => {
        if (matchKey(payload)) {
          updateTeammateStatus(contextKey, payload.teammate_name, "shutdown");
        }
      })
    );

    // team:disbanded
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
      }>("team:disbanded", (payload) => {
        if (matchKey(payload)) {
          disbandTeam(contextKey);
          setTeamActive(contextKey, false);
        }
      })
    );

    // agent:chunk — route teammate streaming text
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        teammate_name?: string | null;
        text: string;
      }>("agent:chunk", (payload) => {
        if (payload.teammate_name && matchKey(payload)) {
          appendTeammateChunk(contextKey, payload.teammate_name, payload.text);
        }
      })
    );

    return () => unsubs.forEach((u) => u());
  }, [
    bus, contextKey,
    createTeam, addTeammate, updateTeammateStatus,
    appendTeammateChunk, clearTeammateStream, updateTeammateCost,
    addTeamMessage, disbandTeam, setTeamActive,
  ]);
}
