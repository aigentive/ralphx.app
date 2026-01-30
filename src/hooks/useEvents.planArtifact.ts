/**
 * Plan artifact event hooks - Tauri plan artifact event listeners with type-safe validation
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { PlanArtifactEventSchema } from "@/types/events";
import { useIdeationStore } from "@/stores/ideationStore";
import { ideationKeys } from "./useIdeation";
import type { Artifact } from "@/types/artifact";

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
  const setPlanArtifact = useIdeationStore((s) => s.setPlanArtifact);
  const activeSessionId = useIdeationStore((s) => s.activeSessionId);
  const sessions = useIdeationStore((s) => s.sessions);
  const queryClient = useQueryClient();

  useEffect(() => {
    // Listen for created events
    const unlistenCreated: Promise<UnlistenFn> = listen<unknown>(
      "plan_artifact:created",
      (event) => {
        const parsed = PlanArtifactEventSchema.safeParse({
          type: "created",
          ...(event.payload as Record<string, unknown>),
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
            setPlanArtifact(planArtifact);
          }

          // Invalidate session query to refetch with new plan artifact link
          queryClient.invalidateQueries({
            queryKey: ideationKeys.sessionWithData(sessionId),
          });
        }
      }
    );

    // Listen for updated events
    const unlistenUpdated: Promise<UnlistenFn> = listen<unknown>(
      "plan_artifact:updated",
      (event) => {
        const parsed = PlanArtifactEventSchema.safeParse({
          type: "updated",
          ...(event.payload as Record<string, unknown>),
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

          // Find the session that has this artifact linked
          // Check active session first (most common case)
          const activeSession = activeSessionId
            ? sessions[activeSessionId]
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
            setPlanArtifact(planArtifact);
          }

          // Find any session with this artifact and invalidate its query
          for (const session of Object.values(sessions)) {
            if (session.planArtifactId === artifactId) {
              queryClient.invalidateQueries({
                queryKey: ideationKeys.sessionWithData(session.id),
              });
            }
          }
        }
      }
    );

    return () => {
      unlistenCreated.then((fn) => fn());
      unlistenUpdated.then((fn) => fn());
    };
  }, [setPlanArtifact, activeSessionId, sessions, queryClient]);
}
