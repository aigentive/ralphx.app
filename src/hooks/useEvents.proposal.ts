/**
 * Proposal event hooks - Tauri proposal event listeners with type-safe validation
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { ProposalEventSchema, ProposalsReorderedEventSchema } from "@/types/events";
import { useProposalStore } from "@/stores/proposalStore";
import { ideationKeys } from "./useIdeation";
import type { TaskProposal } from "@/types/ideation";

/**
 * Hook to listen for proposal events from the backend
 *
 * Listens to 'proposal:created', 'proposal:updated', and 'proposal:deleted' events
 * and updates the proposal store accordingly. Validates incoming events using
 * ProposalEventSchema before processing.
 *
 * @example
 * ```tsx
 * function App() {
 *   useProposalEvents(); // Sets up listener automatically
 *   return <IdeationView />;
 * }
 * ```
 */
export function useProposalEvents() {
  const addProposal = useProposalStore((s) => s.addProposal);
  const updateProposal = useProposalStore((s) => s.updateProposal);
  const removeProposal = useProposalStore((s) => s.removeProposal);
  const queryClient = useQueryClient();

  useEffect(() => {
    // Listen for created events
    const unlistenCreated: Promise<UnlistenFn> = listen<unknown>("proposal:created", (event) => {
      const parsed = ProposalEventSchema.safeParse({ type: "created", ...(event.payload as Record<string, unknown>) });

      if (!parsed.success) {
        console.error("Invalid proposal:created event:", parsed.error.message);
        return;
      }

      if (parsed.data.type === "created") {
        // Transform the proposal data from snake_case to camelCase
        const p = parsed.data.proposal;
        const proposal: TaskProposal = {
          id: p.id,
          sessionId: p.session_id,
          title: p.title,
          description: p.description,
          category: p.category as TaskProposal["category"],
          steps: p.steps,
          acceptanceCriteria: p.acceptance_criteria,
          suggestedPriority: p.suggested_priority as TaskProposal["suggestedPriority"],
          priorityScore: p.priority_score,
          priorityReason: p.priority_reason,
          estimatedComplexity: p.estimated_complexity as TaskProposal["estimatedComplexity"],
          userPriority: p.user_priority as TaskProposal["userPriority"],
          userModified: p.user_modified,
          status: p.status as TaskProposal["status"],
          selected: p.selected,
          createdTaskId: p.created_task_id,
          planArtifactId: p.plan_artifact_id,
          planVersionAtCreation: p.plan_version_at_creation,
          sortOrder: p.sort_order,
          createdAt: p.created_at,
          updatedAt: p.updated_at,
        };
        addProposal(proposal);
        // Invalidate session query to ensure data consistency
        queryClient.invalidateQueries({
          queryKey: ideationKeys.sessionWithData(proposal.sessionId),
        });
      }
    });

    // Listen for updated events
    const unlistenUpdated: Promise<UnlistenFn> = listen<unknown>("proposal:updated", (event) => {
      const parsed = ProposalEventSchema.safeParse({ type: "updated", ...(event.payload as Record<string, unknown>) });

      if (!parsed.success) {
        console.error("Invalid proposal:updated event:", parsed.error.message);
        return;
      }

      if (parsed.data.type === "updated") {
        // Transform the proposal data from snake_case to camelCase
        const p = parsed.data.proposal;
        const proposal: TaskProposal = {
          id: p.id,
          sessionId: p.session_id,
          title: p.title,
          description: p.description,
          category: p.category as TaskProposal["category"],
          steps: p.steps,
          acceptanceCriteria: p.acceptance_criteria,
          suggestedPriority: p.suggested_priority as TaskProposal["suggestedPriority"],
          priorityScore: p.priority_score,
          priorityReason: p.priority_reason,
          estimatedComplexity: p.estimated_complexity as TaskProposal["estimatedComplexity"],
          userPriority: p.user_priority as TaskProposal["userPriority"],
          userModified: p.user_modified,
          status: p.status as TaskProposal["status"],
          selected: p.selected,
          createdTaskId: p.created_task_id,
          planArtifactId: p.plan_artifact_id,
          planVersionAtCreation: p.plan_version_at_creation,
          sortOrder: p.sort_order,
          createdAt: p.created_at,
          updatedAt: p.updated_at,
        };
        // Use updateProposal to merge changes (or replace the whole proposal)
        updateProposal(proposal.id, proposal);
        // Invalidate session query to ensure data consistency
        queryClient.invalidateQueries({
          queryKey: ideationKeys.sessionWithData(proposal.sessionId),
        });
      }
    });

    // Listen for deleted events
    const unlistenDeleted: Promise<UnlistenFn> = listen<unknown>("proposal:deleted", (event) => {
      const parsed = ProposalEventSchema.safeParse({ type: "deleted", ...(event.payload as Record<string, unknown>) });

      if (!parsed.success) {
        console.error("Invalid proposal:deleted event:", parsed.error.message);
        return;
      }

      if (parsed.data.type === "deleted") {
        removeProposal(parsed.data.proposalId);
      }
    });

    // Listen for reordered events
    const unlistenReordered: Promise<UnlistenFn> = listen<unknown>("proposals:reordered", (event) => {
      const parsed = ProposalsReorderedEventSchema.safeParse(event.payload);

      if (!parsed.success) {
        console.error("Invalid proposals:reordered event:", parsed.error.message);
        return;
      }

      // Update each proposal with the new sortOrder from the backend
      for (const p of parsed.data.proposals) {
        const proposal: TaskProposal = {
          id: p.id,
          sessionId: p.session_id,
          title: p.title,
          description: p.description,
          category: p.category as TaskProposal["category"],
          steps: p.steps,
          acceptanceCriteria: p.acceptance_criteria,
          suggestedPriority: p.suggested_priority as TaskProposal["suggestedPriority"],
          priorityScore: p.priority_score,
          priorityReason: p.priority_reason,
          estimatedComplexity: p.estimated_complexity as TaskProposal["estimatedComplexity"],
          userPriority: p.user_priority as TaskProposal["userPriority"],
          userModified: p.user_modified,
          status: p.status as TaskProposal["status"],
          selected: p.selected,
          createdTaskId: p.created_task_id,
          planArtifactId: p.plan_artifact_id,
          planVersionAtCreation: p.plan_version_at_creation,
          sortOrder: p.sort_order,
          createdAt: p.created_at,
          updatedAt: p.updated_at,
        };
        updateProposal(proposal.id, proposal);
      }

      // Invalidate session query to ensure data consistency
      if (parsed.data.session_id) {
        queryClient.invalidateQueries({
          queryKey: ideationKeys.sessionWithData(parsed.data.session_id),
        });
      }
    });

    return () => {
      unlistenCreated.then((fn) => fn());
      unlistenUpdated.then((fn) => fn());
      unlistenDeleted.then((fn) => fn());
      unlistenReordered.then((fn) => fn());
    };
  }, [addProposal, updateProposal, removeProposal, queryClient]);
}
