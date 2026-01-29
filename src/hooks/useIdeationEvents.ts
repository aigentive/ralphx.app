/**
 * Ideation event hooks - Tauri ideation event listeners with type-safe validation
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { z } from "zod";
import { useIdeationStore } from "@/stores/ideationStore";

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

  useEffect(() => {
    // Listen for session title updates (from session-namer agent)
    const unlistenTitleUpdated: Promise<UnlistenFn> = listen<unknown>(
      "ideation:session_title_updated",
      (event) => {
        const parsed = SessionTitleUpdatedEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error(
            "Invalid ideation:session_title_updated event:",
            parsed.error.message
          );
          return;
        }

        // Update the session in the store with the new title
        updateSession(parsed.data.sessionId, { title: parsed.data.title });
      }
    );

    return () => {
      unlistenTitleUpdated.then((fn) => fn());
    };
  }, [updateSession]);
}
