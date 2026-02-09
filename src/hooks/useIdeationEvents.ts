/**
 * Ideation event hooks - Tauri ideation event listeners with type-safe validation
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect } from "react";
import { z } from "zod";
import { useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { useIdeationStore } from "@/stores/ideationStore";
import { ideationKeys } from "./useIdeation";
import { dependencyKeys } from "./useDependencyGraph";
import type { Unsubscribe } from "@/lib/event-bus";
import { logger } from "@/lib/logger";

/**
 * Schema for session title updated event payload
 */
const SessionTitleUpdatedEventSchema = z.object({
  sessionId: z.string(),
  title: z.string().nullable(),
});

/**
 * Schema for proposal priority assessed event payload
 */
const ProposalPriorityAssessedEventSchema = z.object({
  proposalId: z.string(),
  priority: z.string(),
  score: z.number(),
  reason: z.string(),
});

/**
 * Schema for session priorities assessed event payload
 */
const SessionPrioritiesAssessedEventSchema = z.object({
  sessionId: z.string(),
  count: z.number(),
});

/**
 * Schema for dependency added/removed event payload
 */
const DependencyEventSchema = z.object({
  proposalId: z.string(),
  dependsOnId: z.string(),
});

/**
 * Schema for dependency analysis started event payload
 */
const DependencyAnalysisStartedEventSchema = z.object({
  sessionId: z.string(),
});

/**
 * Schema for dependency suggestions applied event payload
 */
const DependencySuggestionsAppliedEventSchema = z.object({
  sessionId: z.string(),
  appliedCount: z.number(),
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
  const bus = useEventBus();
  const updateSession = useIdeationStore((s) => s.updateSession);
  const queryClient = useQueryClient();

  useEffect(() => {
    logger.debug("[IdeationEvents] Setting up event listeners");
    const unsubscribes: Unsubscribe[] = [];

    // Listen for session title updates (from session-namer agent)
    unsubscribes.push(
      bus.subscribe<unknown>("ideation:session_title_updated", (payload) => {
        logger.debug("[IdeationEvents] Received session_title_updated:", payload);
        const parsed = SessionTitleUpdatedEventSchema.safeParse(payload);

        if (!parsed.success) {
          console.error(
            "Invalid ideation:session_title_updated event:",
            parsed.error.message
          );
          return;
        }

        logger.debug("[IdeationEvents] Updating session title:", parsed.data.sessionId, "->", parsed.data.title);
        updateSession(parsed.data.sessionId, { title: parsed.data.title });
        queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
      })
    );

    // Listen for single proposal priority assessment
    unsubscribes.push(
      bus.subscribe<unknown>("proposal:priority_assessed", (payload) => {
        logger.debug("[IdeationEvents] Received proposal:priority_assessed:", payload);
        const parsed = ProposalPriorityAssessedEventSchema.safeParse(payload);

        if (!parsed.success) {
          console.error(
            "Invalid proposal:priority_assessed event:",
            parsed.error.message
          );
          return;
        }

        // Invalidate proposals query to refetch with updated priority
        queryClient.invalidateQueries({ queryKey: ideationKeys.proposals() });
      })
    );

    // Listen for batch session priorities assessment
    unsubscribes.push(
      bus.subscribe<unknown>("session:priorities_assessed", (payload) => {
        logger.debug("[IdeationEvents] Received session:priorities_assessed:", payload);
        const parsed = SessionPrioritiesAssessedEventSchema.safeParse(payload);

        if (!parsed.success) {
          console.error(
            "Invalid session:priorities_assessed event:",
            parsed.error.message
          );
          return;
        }

        // Invalidate proposals query to refetch with updated priorities
        queryClient.invalidateQueries({ queryKey: ideationKeys.proposals() });
      })
    );

    // Listen for dependency added
    unsubscribes.push(
      bus.subscribe<unknown>("dependency:added", (payload) => {
        logger.debug("[IdeationEvents] Received dependency:added:", payload);
        const parsed = DependencyEventSchema.safeParse(payload);

        if (!parsed.success) {
          console.error("Invalid dependency:added event:", parsed.error.message);
          return;
        }

        // Invalidate dependency graph query
        queryClient.invalidateQueries({ queryKey: dependencyKeys.graphs() });
      })
    );

    // Listen for dependency removed
    unsubscribes.push(
      bus.subscribe<unknown>("dependency:removed", (payload) => {
        logger.debug("[IdeationEvents] Received dependency:removed:", payload);
        const parsed = DependencyEventSchema.safeParse(payload);

        if (!parsed.success) {
          console.error("Invalid dependency:removed event:", parsed.error.message);
          return;
        }

        // Invalidate dependency graph query
        queryClient.invalidateQueries({ queryKey: dependencyKeys.graphs() });
      })
    );

    // Listen for dependency analysis started (AI suggestion in progress)
    unsubscribes.push(
      bus.subscribe<unknown>("dependencies:analysis_started", (payload) => {
        logger.debug("[IdeationEvents] Received dependencies:analysis_started:", payload);
        const parsed = DependencyAnalysisStartedEventSchema.safeParse(payload);

        if (!parsed.success) {
          console.error("Invalid dependencies:analysis_started event:", parsed.error.message);
          return;
        }

        // UI components can listen for this to show loading state
        // The event is emitted for components to handle via their own listeners
      })
    );

    // Listen for dependency suggestions applied (AI suggestion completed)
    unsubscribes.push(
      bus.subscribe<unknown>("dependencies:suggestions_applied", (payload) => {
        logger.debug("[IdeationEvents] Received dependencies:suggestions_applied:", payload);
        const parsed = DependencySuggestionsAppliedEventSchema.safeParse(payload);

        if (!parsed.success) {
          console.error("Invalid dependencies:suggestions_applied event:", parsed.error.message);
          return;
        }

        // Invalidate dependency graph query to show new dependencies
        queryClient.invalidateQueries({ queryKey: dependencyKeys.graphs() });
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, updateSession, queryClient]);
}
