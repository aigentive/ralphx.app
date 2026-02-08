/**
 * Plan artifact event hooks - Tauri plan artifact event listeners with type-safe validation
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { PlanArtifactEventSchema } from "@/types/events";
import { useIdeationStore } from "@/stores/ideationStore";
import { ideationKeys } from "./useIdeation";
import type { Artifact } from "@/types/artifact";
import type { IdeationSession } from "@/types/ideation";
import type { Unsubscribe } from "@/lib/event-bus";

/**
 * Hook to listen for plan artifact events from the backend
 *
 * Listens to 'plan_artifact:created' and 'plan_artifact:updated' events
 * and updates the ideation store/query cache accordingly. This enables
 * real-time plan artifact updates when the orchestrator creates or updates plans.
 *
 * @example
 * ```tsx
 * function App() {
 *   usePlanArtifactEvents(); // Sets up listener automatically
 *   return <IdeationView />;
 * }
 * ```
 */
export function usePlanArtifactEvents() {
  const bus = useEventBus();
  const setPlanArtifact = useIdeationStore((s) => s.setPlanArtifact);
  const activeSessionId = useIdeationStore((s) => s.activeSessionId);
  const sessions = useIdeationStore((s) => s.sessions);
  const queryClient = useQueryClient();

  // Keep sessions and setPlanArtifact in refs so the effect doesn't
  // re-subscribe every time the sessions Record or store actions change.
  const sessionsRef = useRef<Record<string, IdeationSession>>(sessions);
  const setPlanArtifactRef = useRef(setPlanArtifact);
  const queryClientRef = useRef(queryClient);
  useEffect(() => { sessionsRef.current = sessions; }, [sessions]);
  useEffect(() => { setPlanArtifactRef.current = setPlanArtifact; }, [setPlanArtifact]);
  useEffect(() => { queryClientRef.current = queryClient; }, [queryClient]);

  // Dedup guard: skip duplicate events during subscribe/unsubscribe cycles
  const lastProcessedRef = useRef<string | null>(null);

  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Listen for created events
    unsubscribes.push(
      bus.subscribe<unknown>("plan_artifact:created", (payload) => {
        const parsed = PlanArtifactEventSchema.safeParse({
          type: "created",
          ...(payload as Record<string, unknown>),
        });

        if (!parsed.success) {
          console.error(
            "Invalid plan_artifact:created event:",
            parsed.error.message
          );
          return;
        }

        if (parsed.data.type === "created") {
          const { sessionId, artifact } = parsed.data;

          // Dedup: skip if we already processed this exact event
          const eventKey = `created:${artifact.id}:${artifact.version}`;
          if (lastProcessedRef.current === eventKey) return;
          lastProcessedRef.current = eventKey;

          // Only update store if this is for the active session
          if (sessionId === activeSessionId) {
            // Transform to Artifact type
            const planArtifact: Artifact = {
              id: artifact.id,
              type: "specification",
              name: artifact.name,
              content: { type: "inline", text: artifact.content },
              metadata: {
                createdAt: new Date().toISOString(),
                createdBy: "orchestrator",
                version: artifact.version,
              },
              derivedFrom: [],
            };
            setPlanArtifactRef.current(planArtifact);
          }

          // Invalidate session query to refetch with new plan artifact link
          queryClientRef.current.invalidateQueries({
            queryKey: ideationKeys.sessionWithData(sessionId),
          });
        }
      })
    );

    // Listen for updated events
    unsubscribes.push(
      bus.subscribe<unknown>("plan_artifact:updated", (payload) => {
        const parsed = PlanArtifactEventSchema.safeParse({
          type: "updated",
          ...(payload as Record<string, unknown>),
        });

        if (!parsed.success) {
          console.error(
            "Invalid plan_artifact:updated event:",
            parsed.error.message
          );
          return;
        }

        if (parsed.data.type === "updated") {
          const { artifactId, artifact } = parsed.data;

          // Dedup: skip if we already processed this exact event
          const eventKey = `updated:${artifact.id}:${artifact.version}`;
          if (lastProcessedRef.current === eventKey) return;
          lastProcessedRef.current = eventKey;

          // Find the session that has this artifact linked
          // Check active session first (most common case)
          const currentSessions = sessionsRef.current;
          const activeSession = activeSessionId
            ? currentSessions[activeSessionId]
            : null;
          const isActiveSessionArtifact =
            activeSession?.planArtifactId === artifactId;

          if (isActiveSessionArtifact) {
            // Update store directly for active session
            const planArtifact: Artifact = {
              id: artifact.id,
              type: "specification",
              name: artifact.name,
              content: { type: "inline", text: artifact.content },
              metadata: {
                createdAt: new Date().toISOString(),
                createdBy: "orchestrator",
                version: artifact.version,
              },
              derivedFrom: [],
            };
            setPlanArtifactRef.current(planArtifact);
          }

          // Find any session with this artifact and invalidate its query
          for (const session of Object.values(currentSessions)) {
            if (session.planArtifactId === artifactId) {
              queryClientRef.current.invalidateQueries({
                queryKey: ideationKeys.sessionWithData(session.id),
              });
            }
          }
        }
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, activeSessionId]);
}
