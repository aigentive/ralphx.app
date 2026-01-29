/**
 * Ideation event hooks - Tauri ideation event listeners with type-safe validation
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { z } from "zod";
import { useQueryClient } from "@tanstack/react-query";
import { useIdeationStore } from "@/stores/ideationStore";
import { ideationKeys } from "./useIdeation";

/**
 * Schema for session title updated event payload
 */
const SessionTitleUpdatedEventSchema = z.object({
  sessionId: z.string(),
  title: z.string().nullable(),
});

/**
 * Hook to listen for ideation events from the backend
 *
 * Listens to 'ideation:session_title_updated' events and updates the
 * ideation store accordingly. This enables real-time session title updates
 * when the session-namer agent generates a title.
 *
 * @example
 * ```tsx
 * function App() {
 *   useIdeationEvents(); // Sets up listener automatically
 *   return <IdeationView />;
 * }
 * ```
 */
export function useIdeationEvents() {
  const updateSession = useIdeationStore((s) => s.updateSession);
  const queryClient = useQueryClient();

  useEffect(() => {
    console.log("[IdeationEvents] Setting up session_title_updated listener");

    // Listen for session title updates (from session-namer agent)
    const unlistenTitleUpdated: Promise<UnlistenFn> = listen<unknown>(
      "ideation:session_title_updated",
      (event) => {
        console.log("[IdeationEvents] Received event:", event.payload);
        const parsed = SessionTitleUpdatedEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error(
            "Invalid ideation:session_title_updated event:",
            parsed.error.message
          );
          return;
        }

        console.log("[IdeationEvents] Updating session title:", parsed.data.sessionId, "->", parsed.data.title);
        // Update the session in the store with the new title
        updateSession(parsed.data.sessionId, { title: parsed.data.title });
        // Also invalidate React Query cache so components using useIdeationSessions re-render
        queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
      }
    );

    return () => {
      unlistenTitleUpdated.then((fn) => fn());
    };
  }, [updateSession, queryClient]);
}
