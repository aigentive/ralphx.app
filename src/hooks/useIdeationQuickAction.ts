/**
 * useIdeationQuickAction - Factory hook for creating ideation sessions from command palette
 *
 * Returns a QuickAction object that:
 * - Creates a new ideation session via mutation
 * - Adds it to the ideation store
 * - Sets it as active
 * - Fire-and-forget: sends first message + spawns namer agent
 * - Provides navigation to the created session
 *
 * Visible when search query is non-empty.
 */

import { useMemo } from "react";
import { Lightbulb } from "lucide-react";
import type { QuickAction } from "@/types/quick-action";
import { useCreateIdeationSession } from "./useIdeation";
import { useIdeationStore } from "@/stores/ideationStore";
import { useUiStore } from "@/stores/uiStore";
import { chatApi } from "@/api/chat";
import { ideationApi } from "@/api/ideation";

/**
 * Hook that returns a QuickAction for creating ideation sessions
 *
 * @param projectId - The project ID to create the session in
 * @returns Memoized QuickAction object
 *
 * @example
 * ```tsx
 * const ideationAction = useIdeationQuickAction(projectId);
 *
 * // Check visibility
 * if (ideationAction.isVisible(query)) {
 *   // Render action
 * }
 *
 * // Execute
 * const sessionId = await ideationAction.execute(query);
 *
 * // Navigate
 * ideationAction.navigateTo(sessionId);
 * ```
 */
export function useIdeationQuickAction(projectId: string): QuickAction {
  const createSession = useCreateIdeationSession();
  const addSession = useIdeationStore((s) => s.addSession);
  const setActiveSession = useIdeationStore((s) => s.setActiveSession);
  const setCurrentView = useUiStore((s) => s.setCurrentView);

  return useMemo(
    () => ({
      id: "ideation",
      label: "Start new ideation session",
      icon: Lightbulb,
      description: (query) => `"${query}"`,
      isVisible: (query) => query.trim().length > 0,
      execute: async (query) => {
        // Create session via mutation
        const session = await createSession.mutateAsync({ projectId });

        // Add to store and set active
        addSession(session);
        setActiveSession(session.id);

        // Fire-and-forget: send first message
        chatApi.sendAgentMessage("ideation", session.id, query).catch(() => {
          // Silently ignore errors - fire-and-forget
        });

        // Fire-and-forget: spawn session namer
        ideationApi.sessions.spawnSessionNamer(session.id, query).catch(() => {
          // Silently ignore errors - fire-and-forget
        });

        return session.id;
      },
      creatingLabel: "Creating your ideation session...",
      successLabel: "Session created!",
      viewLabel: "View Session",
      navigateTo: (sessionId) => {
        setActiveSession(sessionId);
        setCurrentView("ideation");
      },
    }),
    [projectId, createSession, addSession, setActiveSession, setCurrentView]
  );
}
