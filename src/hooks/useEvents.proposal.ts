/**
 * Proposal event hooks - Tauri proposal event listeners with type-safe validation
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ProposalEventSchema } from "@/types/events";
import { useProposalStore } from "@/stores/proposalStore";
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
        const proposal: TaskProposal = {
          id: parsed.data.proposal.id,
          sessionId: parsed.data.proposal.sessionId,
          title: parsed.data.proposal.title,
          description: parsed.data.proposal.description,
          category: parsed.data.proposal.category as TaskProposal["category"],
          steps: parsed.data.proposal.steps,
          acceptanceCriteria: parsed.data.proposal.acceptanceCriteria,
          suggestedPriority: parsed.data.proposal.suggestedPriority as TaskProposal["suggestedPriority"],
          priorityScore: parsed.data.proposal.priorityScore,
          priorityReason: parsed.data.proposal.priorityReason,
          estimatedComplexity: parsed.data.proposal.estimatedComplexity as TaskProposal["estimatedComplexity"],
          userPriority: parsed.data.proposal.userPriority as TaskProposal["userPriority"],
          userModified: parsed.data.proposal.userModified,
          status: parsed.data.proposal.status as TaskProposal["status"],
          selected: parsed.data.proposal.selected,
          createdTaskId: parsed.data.proposal.createdTaskId,
          planArtifactId: parsed.data.proposal.planArtifactId,
          planVersionAtCreation: parsed.data.proposal.planVersionAtCreation,
          sortOrder: parsed.data.proposal.sortOrder,
          createdAt: parsed.data.proposal.createdAt,
          updatedAt: parsed.data.proposal.updatedAt,
        };
        addProposal(proposal);
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
        const proposal: TaskProposal = {
          id: parsed.data.proposal.id,
          sessionId: parsed.data.proposal.sessionId,
          title: parsed.data.proposal.title,
          description: parsed.data.proposal.description,
          category: parsed.data.proposal.category as TaskProposal["category"],
          steps: parsed.data.proposal.steps,
          acceptanceCriteria: parsed.data.proposal.acceptanceCriteria,
          suggestedPriority: parsed.data.proposal.suggestedPriority as TaskProposal["suggestedPriority"],
          priorityScore: parsed.data.proposal.priorityScore,
          priorityReason: parsed.data.proposal.priorityReason,
          estimatedComplexity: parsed.data.proposal.estimatedComplexity as TaskProposal["estimatedComplexity"],
          userPriority: parsed.data.proposal.userPriority as TaskProposal["userPriority"],
          userModified: parsed.data.proposal.userModified,
          status: parsed.data.proposal.status as TaskProposal["status"],
          selected: parsed.data.proposal.selected,
          createdTaskId: parsed.data.proposal.createdTaskId,
          planArtifactId: parsed.data.proposal.planArtifactId,
          planVersionAtCreation: parsed.data.proposal.planVersionAtCreation,
          sortOrder: parsed.data.proposal.sortOrder,
          createdAt: parsed.data.proposal.createdAt,
          updatedAt: parsed.data.proposal.updatedAt,
        };
        // Use updateProposal to merge changes (or replace the whole proposal)
        updateProposal(proposal.id, proposal);
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

    return () => {
      unlistenCreated.then((fn) => fn());
      unlistenUpdated.then((fn) => fn());
      unlistenDeleted.then((fn) => fn());
    };
  }, [addProposal, updateProposal, removeProposal]);
}
