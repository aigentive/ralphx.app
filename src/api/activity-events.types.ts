// Frontend types for activity events API responses (camelCase)

/**
 * Activity event type values
 */
export type ActivityEventType =
  | "thinking"
  | "tool_call"
  | "tool_result"
  | "text"
  | "error";

/**
 * Activity event role values
 */
export type ActivityEventRole = "agent" | "system" | "user";

/**
 * Activity event response (camelCase for frontend)
 */
export interface ActivityEventResponse {
  id: string;
  taskId: string | null;
  ideationSessionId: string | null;
  internalStatus: string | null;
  eventType: ActivityEventType;
  role: ActivityEventRole;
  content: string;
  metadata: string | null;
  createdAt: string;
}

/**
 * Paginated response for activity events
 */
export interface ActivityEventPageResponse {
  events: ActivityEventResponse[];
  cursor: string | null;
  hasMore: boolean;
}

/**
 * Filter input for activity event queries (camelCase for frontend API)
 */
export interface ActivityEventFilter {
  eventTypes?: ActivityEventType[];
  roles?: ActivityEventRole[];
  statuses?: string[];
}
