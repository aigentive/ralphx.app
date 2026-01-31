/**
 * Mock Activity Events API
 *
 * Mirrors the interface of src/api/activity-events.ts with mock implementations.
 */

import type {
  ActivityEventPageResponse,
  ActivityEventFilter,
} from "@/api/activity-events.types";

// ============================================================================
// Mock Activity Events API
// ============================================================================

export const mockActivityEventsApi = {
  task: {
    list: async (
      _taskId: string,
      _options?: {
        cursor?: string;
        limit?: number;
        filter?: ActivityEventFilter;
      }
    ): Promise<ActivityEventPageResponse> => {
      return {
        events: [],
        cursor: null,
        hasMore: false,
      };
    },

    count: async (
      _taskId: string,
      _filter?: ActivityEventFilter
    ): Promise<number> => {
      return 0;
    },
  },

  session: {
    list: async (
      _sessionId: string,
      _options?: {
        cursor?: string;
        limit?: number;
        filter?: ActivityEventFilter;
      }
    ): Promise<ActivityEventPageResponse> => {
      return {
        events: [],
        cursor: null,
        hasMore: false,
      };
    },

    count: async (
      _sessionId: string,
      _filter?: ActivityEventFilter
    ): Promise<number> => {
      return 0;
    },
  },
} as const;
