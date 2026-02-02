// Frontend types for task graph API (camelCase)

/**
 * Node in the task dependency graph - frontend representation (camelCase)
 */
export interface TaskGraphNode {
  taskId: string;
  title: string;
  internalStatus: string;
  priority: number;
  inDegree: number;
  outDegree: number;
  tier: number;
  planArtifactId: string | null;
  sourceProposalId: string | null;
}

/**
 * Edge in the task dependency graph - frontend representation (camelCase)
 */
export interface TaskGraphEdge {
  source: string;
  target: string;
  isCriticalPath: boolean;
}

/**
 * Status summary for a plan group - frontend representation (camelCase)
 */
export interface StatusSummary {
  backlog: number;
  ready: number;
  blocked: number;
  executing: number;
  qa: number;
  review: number;
  merge: number;
  completed: number;
  terminal: number;
}

/**
 * Information about a plan group in the graph - frontend representation (camelCase)
 */
export interface PlanGroupInfo {
  planArtifactId: string;
  sessionId: string;
  sessionTitle: string | null;
  taskIds: string[];
  statusSummary: StatusSummary;
}

/**
 * Full task dependency graph response - frontend representation (camelCase)
 */
export interface TaskDependencyGraphResponse {
  nodes: TaskGraphNode[];
  edges: TaskGraphEdge[];
  planGroups: PlanGroupInfo[];
  criticalPath: string[];
  hasCycles: boolean;
}

// ============================================================================
// Timeline Event Types (Phase 67 - Task D.2)
// ============================================================================

/**
 * Event type enum for timeline entries - frontend representation
 */
export type TimelineEventType = "status_change" | "plan_accepted" | "plan_completed";

/**
 * Single event in the execution timeline - frontend representation (camelCase)
 */
export interface TimelineEvent {
  id: string;
  timestamp: string;
  taskId: string | null;
  taskTitle: string | null;
  eventType: TimelineEventType;
  fromStatus: string | null;
  toStatus: string | null;
  description: string;
  trigger: string | null;
  planArtifactId: string | null;
  sessionTitle: string | null;
}

/**
 * Response for timeline events query - frontend representation (camelCase)
 */
export interface TimelineEventsResponse {
  events: TimelineEvent[];
  total: number;
  hasMore: boolean;
}
