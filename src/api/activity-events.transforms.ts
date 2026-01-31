// Transform functions for converting snake_case API responses to camelCase frontend types

import { z } from "zod";
import {
  ActivityEventResponseSchema,
  ActivityEventPageResponseSchema,
} from "./activity-events.schemas";
import type {
  ActivityEventResponse,
  ActivityEventPageResponse,
  ActivityEventType,
  ActivityEventRole,
  ActivityEventFilter,
} from "./activity-events.types";

/**
 * Transform a single activity event from snake_case to camelCase
 */
export function transformActivityEvent(
  raw: z.infer<typeof ActivityEventResponseSchema>
): ActivityEventResponse {
  return {
    id: raw.id,
    taskId: raw.task_id,
    ideationSessionId: raw.ideation_session_id,
    internalStatus: raw.internal_status,
    eventType: raw.event_type as ActivityEventType,
    role: raw.role as ActivityEventRole,
    content: raw.content,
    metadata: raw.metadata,
    createdAt: raw.created_at,
  };
}

/**
 * Transform a page of activity events from snake_case to camelCase
 */
export function transformActivityEventPage(
  raw: z.infer<typeof ActivityEventPageResponseSchema>
): ActivityEventPageResponse {
  return {
    events: raw.events.map(transformActivityEvent),
    cursor: raw.cursor,
    hasMore: raw.has_more,
  };
}

/**
 * Transform frontend filter to backend format (camelCase to snake_case)
 */
export function transformFilterToBackend(
  filter: ActivityEventFilter
): Record<string, unknown> {
  return {
    event_types: filter.eventTypes,
    roles: filter.roles,
    statuses: filter.statuses,
    task_id: filter.taskId,
    session_id: filter.sessionId,
  };
}
