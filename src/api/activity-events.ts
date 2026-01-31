// Tauri invoke wrappers for activity events with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { ActivityEventPageResponseSchema } from "./activity-events.schemas";
import {
  transformActivityEventPage,
  transformFilterToBackend,
} from "./activity-events.transforms";
import type {
  ActivityEventPageResponse,
  ActivityEventFilter,
} from "./activity-events.types";

// Re-export types for convenience
export type {
  ActivityEventResponse,
  ActivityEventPageResponse,
  ActivityEventFilter,
  ActivityEventType,
  ActivityEventRole,
} from "./activity-events.types";

// ============================================================================
// Typed Invoke Helper
// ============================================================================

async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

// ============================================================================
// API Object
// ============================================================================

/**
 * Activity Events API wrappers for Tauri commands
 *
 * Provides paginated access to persistent activity events for tasks and ideation sessions.
 * Events are ordered by created_at DESC (newest first).
 */
export const activityEventsApi = {
  /**
   * Task activity event operations
   */
  task: {
    /**
     * List activity events for a task with cursor-based pagination
     * @param taskId The task ID to get events for
     * @param options Pagination and filter options
     * @returns A page of events with cursor for next page
     */
    list: async (
      taskId: string,
      options?: {
        cursor?: string;
        limit?: number;
        filter?: ActivityEventFilter;
      }
    ): Promise<ActivityEventPageResponse> => {
      const raw = await typedInvoke(
        "list_task_activity_events",
        {
          taskId,
          cursor: options?.cursor ?? null,
          limit: options?.limit ?? null,
          filter: options?.filter
            ? transformFilterToBackend(options.filter)
            : null,
        },
        ActivityEventPageResponseSchema
      );
      return transformActivityEventPage(raw);
    },

    /**
     * Count activity events for a task
     * @param taskId The task ID
     * @param filter Optional filter criteria
     * @returns Total count of matching events
     */
    count: async (
      taskId: string,
      filter?: ActivityEventFilter
    ): Promise<number> => {
      return typedInvoke(
        "count_task_activity_events",
        {
          taskId,
          filter: filter ? transformFilterToBackend(filter) : null,
        },
        z.number()
      );
    },
  },

  /**
   * Session activity event operations
   */
  session: {
    /**
     * List activity events for an ideation session with cursor-based pagination
     * @param sessionId The ideation session ID to get events for
     * @param options Pagination and filter options
     * @returns A page of events with cursor for next page
     */
    list: async (
      sessionId: string,
      options?: {
        cursor?: string;
        limit?: number;
        filter?: ActivityEventFilter;
      }
    ): Promise<ActivityEventPageResponse> => {
      const raw = await typedInvoke(
        "list_session_activity_events",
        {
          sessionId,
          cursor: options?.cursor ?? null,
          limit: options?.limit ?? null,
          filter: options?.filter
            ? transformFilterToBackend(options.filter)
            : null,
        },
        ActivityEventPageResponseSchema
      );
      return transformActivityEventPage(raw);
    },

    /**
     * Count activity events for an ideation session
     * @param sessionId The session ID
     * @param filter Optional filter criteria
     * @returns Total count of matching events
     */
    count: async (
      sessionId: string,
      filter?: ActivityEventFilter
    ): Promise<number> => {
      return typedInvoke(
        "count_session_activity_events",
        {
          sessionId,
          filter: filter ? transformFilterToBackend(filter) : null,
        },
        z.number()
      );
    },
  },
} as const;
