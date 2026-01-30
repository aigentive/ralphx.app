/**
 * Ideation event hooks - Tauri ideation event listeners with type-safe validation
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { z } from "zod";
import { useQueryClient } from "@tanstack/react-query";
import { useIdeationStore } from "@/stores/ideationStore";
import { ideationKeys } from "./useIdeation";
import { dependencyKeys } from "./useDependencyGraph";

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
  const updateSession = useIdeationStore((s) => s.updateSession);
  const queryClient = useQueryClient();

  useEffect(() => {
    console.log("[IdeationEvents] Setting up event listeners");
    const unlistenFns: Promise<UnlistenFn>[] = [];

    // Listen for session title updates (from session-namer agent)
    unlistenFns.push(
      listen<unknown>("ideation:session_title_updated", (event) => {
        console.log("[IdeationEvents] Received session_title_updated:", event.payload);
        const parsed = SessionTitleUpdatedEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error(
            "Invalid ideation:session_title_updated event:",
            parsed.error.message
          );
          return;
        }

        console.log("[IdeationEvents] Updating session title:", parsed.data.sessionId, "->", parsed.data.title);
        updateSession(parsed.data.sessionId, { title: parsed.data.title });
        queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
      })
    );

    // Listen for single proposal priority assessment
    unlistenFns.push(
      listen<unknown>("proposal:priority_assessed", (event) => {
        console.log("[IdeationEvents] Received proposal:priority_assessed:", event.payload);
        const parsed = ProposalPriorityAssessedEventSchema.safeParse(event.payload);

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
    unlistenFns.push(
      listen<unknown>("session:priorities_assessed", (event) => {
        console.log("[IdeationEvents] Received session:priorities_assessed:", event.payload);
        const parsed = SessionPrioritiesAssessedEventSchema.safeParse(event.payload);

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
    unlistenFns.push(
      listen<unknown>("dependency:added", (event) => {
        console.log("[IdeationEvents] Received dependency:added:", event.payload);
        const parsed = DependencyEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error("Invalid dependency:added event:", parsed.error.message);
          return;
        }

        // Invalidate dependency graph query
        queryClient.invalidateQueries({ queryKey: dependencyKeys.graphs() });
      })
    );

    // Listen for dependency removed
    unlistenFns.push(
      listen<unknown>("dependency:removed", (event) => {
        console.log("[IdeationEvents] Received dependency:removed:", event.payload);
        const parsed = DependencyEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error("Invalid dependency:removed event:", parsed.error.message);
          return;
        }

        // Invalidate dependency graph query
        queryClient.invalidateQueries({ queryKey: dependencyKeys.graphs() });
      })
    );

    // Listen for dependency analysis started (AI suggestion in progress)
    unlistenFns.push(
      listen<unknown>("dependencies:analysis_started", (event) => {
        console.log("[IdeationEvents] Received dependencies:analysis_started:", event.payload);
        const parsed = DependencyAnalysisStartedEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error("Invalid dependencies:analysis_started event:", parsed.error.message);
          return;
        }

        // UI components can listen for this to show loading state
        // The event is emitted for components to handle via their own listeners
      })
    );

    // Listen for dependency suggestions applied (AI suggestion completed)
    unlistenFns.push(
      listen<unknown>("dependencies:suggestions_applied", (event) => {
        console.log("[IdeationEvents] Received dependencies:suggestions_applied:", event.payload);
        const parsed = DependencySuggestionsAppliedEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error("Invalid dependencies:suggestions_applied event:", parsed.error.message);
          return;
        }

        // Invalidate dependency graph query to show new dependencies
        queryClient.invalidateQueries({ queryKey: dependencyKeys.graphs() });
        // Also invalidate proposals since their dependency counts may have changed
        queryClient.invalidateQueries({ queryKey: ideationKeys.proposals() });
      })
    );

    return () => {
      unlistenFns.forEach((unlisten) => unlisten.then((fn) => fn()));
    };
  }, [updateSession, queryClient]);
}
