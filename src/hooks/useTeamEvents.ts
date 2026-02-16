/**
 * useTeamEvents — Team lifecycle event consumer
 *
 * Subscribes to team:* events and agent:* events with teammate_name.
 * Routes events to teamStore actions, filtered by contextKey.
 *
 * Split into two effects to fix the chicken-and-egg problem:
 *   Effect 1 (always active): team:created + team:disbanded — runs whenever
 *     contextKey is non-null so creation events are never missed.
 *   Effect 2 (gated by isTeamActive): all other team events — only subscribes
 *     once the team exists in the store.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useCallback, useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { useTeamStore } from "@/stores/teamStore";
import { useChatStore } from "@/stores/chatStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import type { ContextType } from "@/types/chat-conversation";
import type { Unsubscribe } from "@/lib/event-bus";
import type {
  TeamCreatedPayload,
  TeamDisbandedPayload,
  TeamTeammateSpawnedPayload,
  TeamTeammateIdlePayload,
  TeamTeammateShutdownPayload,
  TeamMessagePayload,
  TeamCostUpdatePayload,
  TeamPlanRequestedPayload,
} from "@/types/events";

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
  const setPendingPlan = useTeamStore((s) => s.setPendingPlan);

  // Derive isTeamActive from the teamStore so effect 2 re-runs when team is created
  const selectActive = useCallback(
    (s: { activeTeams: Record<string, unknown> }) =>
      contextKey ? contextKey in s.activeTeams : false,
    [contextKey],
  );
  const isTeamActive = useTeamStore(selectActive);

  // Helper: build key from event payload and check match
  const matchKey = useCallback(
    (payload: { context_type: string; context_id: string }): boolean => {
      if (!contextKey) return false;
      const key = buildStoreKey(payload.context_type as ContextType, payload.context_id);
      return key === contextKey;
    },
    [contextKey],
  );

  // ── Effect 1: Always subscribe to team:created + team:disbanded ──────────
  // These must fire even before a team exists so we catch the creation event.
  useEffect(() => {
    if (!contextKey) return;

    const unsubs: Unsubscribe[] = [];

    // team:created — lead_name may not be in payload; default to team_name
    unsubs.push(
      bus.subscribe<TeamCreatedPayload & { lead_name?: string }>("team:created", (payload) => {
        if (matchKey(payload)) {
          createTeam(contextKey, payload.team_name, payload.lead_name ?? payload.team_name);
          setTeamActive(contextKey, true);
        }
      }),
    );

    // team:disbanded
    unsubs.push(
      bus.subscribe<TeamDisbandedPayload>("team:disbanded", (payload) => {
        if (matchKey(payload)) {
          disbandTeam(contextKey);
          setTeamActive(contextKey, false);
        }
      }),
    );

    // team:plan_requested — show approval UI (global, not context-filtered)
    unsubs.push(
      bus.subscribe<TeamPlanRequestedPayload>("team:plan_requested", (payload) => {
        if (payload.validated) {
          setPendingPlan({
            planId: payload.plan_id,
            process: payload.process,
            teammates: payload.teammates,
          });
        }
      }),
    );

    return () => unsubs.forEach((u) => u());
  }, [bus, contextKey, matchKey, createTeam, disbandTeam, setTeamActive, setPendingPlan]);

  // ── Effect 2: Subscribe to remaining events when team is active ──────────
  useEffect(() => {
    if (!contextKey || !isTeamActive) return;

    const unsubs: Unsubscribe[] = [];

    // team:teammate_spawned — backend sends `role`, not `role_description`
    unsubs.push(
      bus.subscribe<TeamTeammateSpawnedPayload>("team:teammate_spawned", (payload) => {
        if (matchKey(payload)) {
          addTeammate(contextKey, {
            name: payload.teammate_name,
            color: payload.color,
            model: payload.model,
            roleDescription: payload.role,
            status: "spawning",
            currentActivity: null,
            tokensUsed: 0,
            estimatedCostUsd: 0,
            streamingText: "",
          });
        }
      }),
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
      }),
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
      }),
    );

    // team:teammate_idle
    unsubs.push(
      bus.subscribe<TeamTeammateIdlePayload & { last_activity?: string }>("team:teammate_idle", (payload) => {
        if (matchKey(payload)) {
          updateTeammateStatus(
            contextKey,
            payload.teammate_name,
            "idle",
            payload.last_activity,
          );
        }
      }),
    );

    // team:message — backend sends `sender`/`recipient`, not `from`/`to`
    unsubs.push(
      bus.subscribe<TeamMessagePayload>("team:message", (payload) => {
        if (matchKey(payload)) {
          addTeamMessage(contextKey, {
            id: payload.message_id ?? `msg-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
            from: payload.sender,
            to: payload.recipient ?? "*",
            content: payload.content,
            timestamp: payload.timestamp,
          });
        }
      }),
    );

    // team:cost_update — backend sends `input_tokens`+`output_tokens` and `estimated_usd`
    unsubs.push(
      bus.subscribe<TeamCostUpdatePayload>("team:cost_update", (payload) => {
        if (matchKey(payload)) {
          updateTeammateCost(
            contextKey,
            payload.teammate_name,
            payload.input_tokens + payload.output_tokens,
            payload.estimated_usd,
          );
        }
      }),
    );

    // team:teammate_shutdown
    unsubs.push(
      bus.subscribe<TeamTeammateShutdownPayload>("team:teammate_shutdown", (payload) => {
        if (matchKey(payload)) {
          updateTeammateStatus(contextKey, payload.teammate_name, "shutdown");
        }
      }),
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
      }),
    );

    return () => unsubs.forEach((u) => u());
  }, [
    bus, contextKey, isTeamActive, matchKey,
    addTeammate, updateTeammateStatus,
    appendTeammateChunk, clearTeammateStream, updateTeammateCost,
    addTeamMessage,
  ]);
}
