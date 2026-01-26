/**
 * Event hooks - Tauri event listeners with type-safe validation
 *
 * Provides hooks for listening to backend events (task changes, agent messages,
 * supervisor alerts) and updating local stores in response.
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import {
  TaskEventSchema,
  ReviewEventSchema,
  ProposalEventSchema,
  type AgentMessageEvent,
} from "@/types/events";
import { useTaskStore } from "@/stores/taskStore";
import { useActivityStore } from "@/stores/activityStore";
import { useProposalStore } from "@/stores/proposalStore";
import { reviewKeys } from "@/hooks/useReviews";
import { taskKeys } from "@/hooks/useTasks";
import type { Task } from "@/types/task";
import type { TaskProposal } from "@/types/ideation";

/**
 * Hook to listen for task events from the backend
 *
 * Listens to 'task:event' events and updates the task store accordingly.
 * Validates incoming events using TaskEventSchema before processing.
 *
 * @example
 * ```tsx
 * function App() {
 *   useTaskEvents(); // Sets up listener automatically
 *   return <TaskBoard />;
 * }
 * ```
 */
export function useTaskEvents() {
  const addTask = useTaskStore((s) => s.addTask);
  const updateTask = useTaskStore((s) => s.updateTask);
  const removeTask = useTaskStore((s) => s.removeTask);
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<unknown>("task:event", (event) => {
      // Runtime validation of backend events
      const parsed = TaskEventSchema.safeParse(event.payload);

      if (!parsed.success) {
        console.error("Invalid task event:", parsed.error.message);
        return;
      }

      const taskEvent = parsed.data;

      switch (taskEvent.type) {
        case "created":
          addTask(taskEvent.task);
          // Invalidate task list queries to refetch
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          break;
        case "updated":
          // Cast to Partial<Task> for exactOptionalPropertyTypes compatibility
          updateTask(taskEvent.taskId, taskEvent.changes as Partial<Task>);
          // Invalidate task list queries to refetch
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          break;
        case "deleted":
          removeTask(taskEvent.taskId);
          // Invalidate task list queries to refetch
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          break;
        case "status_changed":
          updateTask(taskEvent.taskId, { internalStatus: taskEvent.to });
          // Invalidate task list queries so Kanban board refetches
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addTask, updateTask, removeTask, queryClient]);
}

/**
 * Hook to listen for agent message events
 *
 * Listens to 'agent:message' events and adds them to the activity store.
 * Can optionally filter by taskId.
 *
 * @param taskId - Optional task ID to filter messages for
 *
 * @example
 * ```tsx
 * function TaskActivityStream({ taskId }: { taskId: string }) {
 *   useAgentEvents(taskId);
 *   const messages = useActivityStore((s) => s.getMessagesForTask(taskId));
 *   return <MessageList messages={messages} />;
 * }
 * ```
 */
export function useAgentEvents(taskId?: string) {
  const addMessage = useActivityStore((s) => s.addMessage);

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<AgentMessageEvent>("agent:message", (event) => {
      if (!taskId || event.payload.taskId === taskId) {
        addMessage(event.payload);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [taskId, addMessage]);
}

/**
 * Hook to listen for supervisor alert events
 *
 * Listens to 'supervisor:alert' events and adds them to the activity store.
 *
 * @example
 * ```tsx
 * function SupervisorPanel() {
 *   useSupervisorAlerts();
 *   const alerts = useActivityStore((s) => s.alerts);
 *   return <AlertList alerts={alerts} />;
 * }
 * ```
 */
export function useSupervisorAlerts() {
  const addAlert = useActivityStore((s) => s.addAlert);

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<{
      taskId: string;
      severity: "low" | "medium" | "high" | "critical";
      type: "error" | "loop_detected" | "stuck" | "escalation";
      message: string;
    }>("supervisor:alert", (event) => {
      addAlert(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addAlert]);
}

/**
 * Hook to listen for review events
 *
 * Listens to 'review:update' events and invalidates TanStack Query caches
 * to trigger refetching of review-related data.
 *
 * @example
 * ```tsx
 * function ReviewsPanel() {
 *   useReviewEvents(); // Auto-refreshes review data on backend events
 *   const { data } = usePendingReviews(projectId);
 *   return <ReviewList reviews={data} />;
 * }
 * ```
 */
export function useReviewEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<unknown>("review:update", (event) => {
      // Runtime validation of backend events
      const parsed = ReviewEventSchema.safeParse(event.payload);

      if (!parsed.success) {
        console.error("Invalid review event:", parsed.error.message);
        return;
      }

      const reviewEvent = parsed.data;

      // Always invalidate pending reviews (all events affect this)
      queryClient.invalidateQueries({
        queryKey: reviewKeys.pending(),
      });

      // For completed events, also invalidate task-specific queries
      if (reviewEvent.type === "completed") {
        queryClient.invalidateQueries({
          queryKey: reviewKeys.byTaskId(reviewEvent.taskId),
        });
        queryClient.invalidateQueries({
          queryKey: reviewKeys.stateHistoryById(reviewEvent.taskId),
        });
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [queryClient]);
}

/**
 * Hook to listen for file change events
 *
 * Listens to 'file:change' events for file system updates.
 * This is a placeholder for future implementation.
 */
export function useFileChangeEvents() {
  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<unknown>("file:change", (_event) => {
      // TODO: Implement file change handling
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}

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

// Re-export useStepEvents from its own file
export { useStepEvents } from "./useStepEvents";
