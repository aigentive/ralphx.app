/**
 * useIdeationQuickAction - Factory hook for ideation-specific QuickAction
 *
 * Creates a QuickAction that:
 * - Is visible when search query is non-empty
 * - Creates a new ideation session with the query as first message
 * - Fire-and-forget sends the message and spawns a session namer
 * - Navigates to the ideation view with the new session active
 */

import { useMemo } from "react";
import { Lightbulb } from "lucide-react";
import { useCreateIdeationSession } from "@/hooks/useIdeation";
import { useIdeationStore } from "@/stores/ideationStore";
import { useUiStore } from "@/stores/uiStore";
import { chatApi } from "@/api/chat";
import { ideationApi } from "@/api/ideation";
import type { LucideIcon } from "lucide-react";

// ============================================================================
// Types
// ============================================================================

/**
 * QuickAction configuration for command palette integration
 */
export interface QuickAction {
  /** Unique identifier for this action */
  id: string;
  /** Display label for the action button */
  label: string;
  /** Icon component to render */
  icon: LucideIcon;
  /** Function to generate description from search query */
  description: (query: string) => string;
  /** Function to determine if action should be visible for given query */
  isVisible: (query: string) => boolean;
  /** Function to execute the action, returns entity ID */
  execute: (query: string) => Promise<string>;
  /** Label to show while action is executing */
  creatingLabel: string;
  /** Label to show on success */
  successLabel: string;
  /** Label for the "View" button after success */
  viewLabel: string;
  /** Function to navigate to the created entity */
  navigateTo: (entityId: string) => void;
}

// ============================================================================
// Hook
// ============================================================================

/**
 * Hook to create an ideation-specific QuickAction
 *
 * @param projectId - The project ID for creating sessions
 * @returns QuickAction configuration
 *
 * @example
 * ```tsx
 * const ideationAction = useIdeationQuickAction("project-123");
 *
 * if (ideationAction.isVisible(searchQuery)) {
 *   <QuickActionRow
 *     action={ideationAction}
 *     searchQuery={searchQuery}
 *     ...
 *   />
 * }
 * ```
 */
export function useIdeationQuickAction(projectId: string): QuickAction {
  const createSession = useCreateIdeationSession();
  const selectSession = useIdeationStore((state) => state.selectSession);
  const setActiveSession = useIdeationStore((state) => state.setActiveSession);
  const setCurrentView = useUiStore((state) => state.setCurrentView);

  return useMemo<QuickAction>(
    () => ({
      id: "ideation",
      label: "Start new ideation session",
      icon: Lightbulb,
      description: (query: string) => `"${query}"`,
      isVisible: (query: string) => query.trim().length > 0,

      execute: async (query: string): Promise<string> => {
        // Create the session
        const session = await createSession.mutateAsync({
          projectId,
        });

        // Add to store and set as active
        selectSession(session);

        // Fire-and-forget: send initial message and spawn namer
        // Don't await these - let them happen in background
        chatApi
          .sendAgentMessage("ideation", session.id, query)
          .catch((error) => {
            console.error("Failed to send initial ideation message:", error);
          });

        ideationApi.sessions
          .spawnSessionNamer(session.id, query)
          .catch((error) => {
            console.error("Failed to spawn session namer:", error);
          });

        return session.id;
      },

      creatingLabel: "Creating your ideation session...",
      successLabel: "Session created!",
      viewLabel: "View Session",

      navigateTo: (sessionId: string) => {
        setActiveSession(sessionId);
        setCurrentView("ideation");
      },
    }),
    [projectId, createSession, selectSession, setActiveSession, setCurrentView]
  );
}
