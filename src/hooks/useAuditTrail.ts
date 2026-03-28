/**
 * useAuditTrail - Unified audit trail hook
 * Merges state transitions, review notes, and activity events
 * into a chronological audit trail with phase derivation.
 */

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, type ReviewNoteResponse } from "@/lib/tauri";
import { activityEventsApi } from "@/api/activity-events";
import { stateTransitionKeys } from "@/hooks/useTaskStateTransitions";
import type { ActivityEventResponse } from "@/api/activity-events.types";
import type { StateTransition } from "@/api/tasks";
import { reviewKeys } from "@/hooks/useReviews";

// ============================================================================
// Types
// ============================================================================

export interface AuditEntry {
  id: string;
  timestamp: string;
  source: "transition" | "review" | "activity";
  type: string;
  actor: string;
  description: string;
  metadata?: string | undefined;
  status?: string | undefined;
  fromStatus?: string | undefined;
  toStatus?: string | undefined;
  phaseId?: string | undefined;
  severity?: "info" | "warning" | "error" | "success" | undefined;
  followupSessionId?: string | undefined;
}

export interface AuditPhase {
  id: string;
  label: string;
  type: "execution" | "review" | "merge" | "idle" | "qa";
  status: string;
  startTime: number;
  endTime: number | null;
  entryCount: number;
  reviewOutcome?: string | undefined;
  conversationId?: string | undefined;
  agentRunId?: string | undefined;
}

// ============================================================================
// Query Keys
// ============================================================================

const auditTrailKeys = {
  all: ["auditTrail"] as const,
  byTask: (taskId: string) => [...auditTrailKeys.all, taskId] as const,
};

// ============================================================================
// State Group Classification
// ============================================================================

type PhaseGroup = "execution" | "review" | "merge" | "idle";

const EXECUTION_STATES = new Set<string>([
  "executing", "re_executing", "qa_refining", "qa_testing",
]);
const REVIEW_STATES = new Set<string>([
  "pending_review", "reviewing", "review_passed", "revision_needed", "escalated",
]);
const MERGE_STATES = new Set<string>([
  "pending_merge", "merging", "merged", "merge_conflict", "merge_incomplete",
]);

function getStateGroup(status: string): PhaseGroup {
  if (EXECUTION_STATES.has(status)) return "execution";
  if (REVIEW_STATES.has(status)) return "review";
  if (MERGE_STATES.has(status)) return "merge";
  return "idle";
}

// ============================================================================
// Phase Derivation (pure function, exported for testing)
// ============================================================================

export function derivePhases(transitions: StateTransition[]): AuditPhase[] {
  const phases: AuditPhase[] = [];
  let execCount = 0;
  let reviewCount = 0;
  let currentGroup: PhaseGroup | null = null;

  for (const t of transitions) {
    const group = getStateGroup(t.toStatus);
    const ts = new Date(t.timestamp).getTime();

    if (group === "idle") {
      if (phases.length > 0) {
        phases[phases.length - 1]!.endTime = ts;
      }
      continue;
    }

    if (group !== currentGroup) {
      // Close previous phase
      if (phases.length > 0 && phases[phases.length - 1]!.endTime === null) {
        phases[phases.length - 1]!.endTime = ts;
      }
      currentGroup = group;

      let label: string;
      let id: string;
      if (group === "execution") {
        execCount++;
        label = `Execution #${execCount}`;
        id = `phase-execution-${execCount}`;
      } else if (group === "review") {
        reviewCount++;
        label = `Review #${reviewCount}`;
        id = `phase-review-${reviewCount}`;
      } else {
        label = "Merge";
        id = "phase-merge";
      }

      phases.push({
        id,
        label,
        type: group,
        status: t.toStatus,
        startTime: ts,
        endTime: null,
        entryCount: 0,
        conversationId: t.conversationId,
        agentRunId: t.agentRunId,
      });
    } else {
      // Same group — update current phase
      const current = phases[phases.length - 1]!;
      current.status = t.toStatus;
      if (t.conversationId) current.conversationId = t.conversationId;
      if (t.agentRunId) current.agentRunId = t.agentRunId;
    }
  }

  return phases;
}

// ============================================================================
// Phase ID Assignment
// ============================================================================

function assignPhaseId(entry: AuditEntry, phases: AuditPhase[]): string | undefined {
  const ts = new Date(entry.timestamp).getTime();
  // Search newest-to-oldest so at boundaries the newer phase wins
  for (let i = phases.length - 1; i >= 0; i--) {
    const p = phases[i]!;
    if (ts >= p.startTime && (p.endTime === null || ts <= p.endTime)) {
      return p.id;
    }
  }
  return undefined;
}

// ============================================================================
// Mappers
// ============================================================================

function formatStatus(status: string): string {
  return status
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

function mapTransitionToAuditEntry(t: StateTransition, index: number): AuditEntry {
  const triggerLabels: Record<string, string> = {
    user: "User",
    agent: "Agent",
    system: "System",
  };
  const from = t.fromStatus ? formatStatus(t.fromStatus) : "Created";
  const to = formatStatus(t.toStatus);

  return {
    id: `transition-${index}`,
    timestamp: t.timestamp,
    source: "transition",
    type: "State Change",
    actor: triggerLabels[t.trigger] ?? t.trigger,
    description: `${from} \u2192 ${to}`,
    fromStatus: t.fromStatus ?? undefined,
    toStatus: t.toStatus,
  };
}

function mapReviewToAuditEntry(review: ReviewNoteResponse): AuditEntry {
  const actorLabel = review.reviewer === "ai" ? "AI Reviewer" : review.reviewer === "system" ? "System Escalation" : "Human Reviewer";
  const outcomeLabels: Record<string, string> = {
    approved: "Approved",
    changes_requested: "Changes Requested",
    rejected: "Rejected",
  };

  return {
    id: `review-${review.id}`,
    timestamp: review.created_at,
    source: "review",
    type: outcomeLabels[review.outcome] ?? review.outcome,
    actor: actorLabel,
    description: review.notes ?? review.summary ?? "",
    metadata: review.issues?.length
      ? `${review.issues.length} issue${review.issues.length === 1 ? "" : "s"} found`
      : undefined,
    followupSessionId: review.followup_session_id ?? undefined,
  };
}

function mapActivityToAuditEntry(event: ActivityEventResponse): AuditEntry {
  const roleLabels: Record<string, string> = {
    agent: "Agent",
    system: "System",
    user: "User",
  };

  return {
    id: `activity-${event.id}`,
    timestamp: event.createdAt,
    source: "activity",
    type: event.eventType,
    actor: roleLabels[event.role] ?? event.role,
    description: event.content,
    metadata: event.metadata ?? undefined,
    status: event.internalStatus ?? undefined,
  };
}

// ============================================================================
// Hook
// ============================================================================

export function useAuditTrail(
  taskId: string,
  options: { enabled?: boolean } = {}
) {
  const { enabled = true } = options;

  const reviewQuery = useQuery<ReviewNoteResponse[], Error>({
    queryKey: reviewKeys.stateHistoryById(taskId),
    queryFn: () => api.reviews.getTaskStateHistory(taskId),
    enabled: enabled && !!taskId,
    staleTime: 30_000,
    gcTime: 5 * 60_000,
    refetchOnWindowFocus: false,
  });

  const transitionsQuery = useQuery<StateTransition[], Error>({
    queryKey: stateTransitionKeys.task(taskId),
    queryFn: () => api.tasks.getStateTransitions(taskId),
    enabled: enabled && !!taskId,
    staleTime: 30_000,
    gcTime: 5 * 60_000,
    refetchOnWindowFocus: false,
  });

  const activityQuery = useQuery({
    queryKey: auditTrailKeys.byTask(taskId),
    queryFn: () => activityEventsApi.task.list(taskId, { limit: 500 }),
    enabled: enabled && !!taskId,
    staleTime: 30_000,
    gcTime: 5 * 60_000,
    refetchOnWindowFocus: false,
  });

  const { entries, phases } = useMemo(() => {
    const rawPhases = derivePhases(transitionsQuery.data ?? []);

    const transitionEntries = (transitionsQuery.data ?? []).map(mapTransitionToAuditEntry);
    const reviewEntries = (reviewQuery.data ?? []).map(mapReviewToAuditEntry);
    const activityEntries = (activityQuery.data?.events ?? []).map(mapActivityToAuditEntry);

    const all = [...transitionEntries, ...reviewEntries, ...activityEntries].sort(
      (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );

    // Assign phase IDs
    for (const entry of all) {
      entry.phaseId = assignPhaseId(entry, rawPhases);
    }

    // Count entries per phase
    const counts = new Map<string, number>();
    for (const entry of all) {
      if (entry.phaseId) {
        counts.set(entry.phaseId, (counts.get(entry.phaseId) ?? 0) + 1);
      }
    }

    const phasesWithCounts = rawPhases.map((p) => ({
      ...p,
      entryCount: counts.get(p.id) ?? 0,
    }));

    return { entries: all, phases: phasesWithCounts };
  }, [reviewQuery.data, transitionsQuery.data, activityQuery.data]);

  const isLoading = reviewQuery.isLoading || transitionsQuery.isLoading || activityQuery.isLoading;
  const isEmpty = entries.length === 0;
  const error = reviewQuery.error?.message ?? transitionsQuery.error?.message ?? activityQuery.error?.message ?? null;

  return {
    entries,
    phases,
    isLoading,
    isEmpty,
    error,
    refetch: () => {
      reviewQuery.refetch();
      transitionsQuery.refetch();
      activityQuery.refetch();
    },
  };
}
