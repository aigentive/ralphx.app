/**
 * timelineFilters.ts - Filter definitions and helpers for ExecutionTimeline
 *
 * Provides category-based filtering for timeline events, allowing users to
 * filter by event type (status changes, plan events) and by status category
 * (execution, reviews, escalations, QA, merge, etc.).
 *
 * @see specs/plans/task_graph_view.md section "Task D.6"
 */

import type { TimelineEvent, TimelineEventType } from "@/api/task-graph.types";
import { getStatusCategory } from "../nodes/nodeStyles";
import type { InternalStatus } from "@/types/status";

// ============================================================================
// Types
// ============================================================================

/**
 * Filter categories for timeline events
 * These are user-facing filter options that map to status categories
 */
export type TimelineFilterCategory =
  | "all"
  | "execution"    // executing, re_executing
  | "reviews"      // pending_review, reviewing, review_passed
  | "escalations"  // escalated, revision_needed
  | "qa"           // qa_refining, qa_testing, qa_passed, qa_failed
  | "merge"        // pending_merge, merging, merge_conflict
  | "completed"    // approved, merged
  | "blocked"      // blocked
  | "plans";       // plan_accepted, plan_completed

/**
 * Filter state for the timeline
 */
export interface TimelineFilterState {
  /** Selected filter categories (empty = show all) */
  categories: TimelineFilterCategory[];
}

/**
 * Filter option for UI rendering
 */
export interface TimelineFilterOption {
  id: TimelineFilterCategory;
  label: string;
  description: string;
  /** Color for visual indicator (from nodeStyles) */
  color: string;
}

// ============================================================================
// Constants
// ============================================================================

/**
 * Color mappings for filter categories (matching nodeStyles)
 */
const FILTER_COLORS: Record<TimelineFilterCategory, string> = {
  all: "hsl(220 10% 60%)",
  execution: "hsl(14 100% 55%)",      // Executing orange
  reviews: "hsl(220 80% 60%)",        // Review blue
  escalations: "hsl(0 70% 55%)",      // Escalation red (alert)
  qa: "hsl(280 60% 55%)",             // QA purple
  merge: "hsl(180 60% 50%)",          // Merge cyan
  completed: "hsl(145 60% 45%)",      // Complete green
  blocked: "hsl(45 90% 55%)",         // Blocked amber
  plans: "hsl(220 10% 60%)",          // Plan events gray
};

/**
 * Available filter options for the UI
 */
export const TIMELINE_FILTER_OPTIONS: TimelineFilterOption[] = [
  {
    id: "all",
    label: "All",
    description: "Show all timeline events",
    color: FILTER_COLORS.all,
  },
  {
    id: "execution",
    label: "Execution",
    description: "Task execution started/resumed",
    color: FILTER_COLORS.execution,
  },
  {
    id: "reviews",
    label: "Reviews",
    description: "Pending review, reviewing, review passed",
    color: FILTER_COLORS.reviews,
  },
  {
    id: "escalations",
    label: "Escalations",
    description: "Escalated or revision needed",
    color: FILTER_COLORS.escalations,
  },
  {
    id: "qa",
    label: "QA",
    description: "QA testing and results",
    color: FILTER_COLORS.qa,
  },
  {
    id: "merge",
    label: "Merge",
    description: "Merge workflow events",
    color: FILTER_COLORS.merge,
  },
  {
    id: "completed",
    label: "Completed",
    description: "Approved or merged tasks",
    color: FILTER_COLORS.completed,
  },
  {
    id: "blocked",
    label: "Blocked",
    description: "Tasks blocked by dependencies",
    color: FILTER_COLORS.blocked,
  },
  {
    id: "plans",
    label: "Plans",
    description: "Plan accepted/completed events",
    color: FILTER_COLORS.plans,
  },
];

/**
 * Default filter state (show all events)
 */
export const DEFAULT_FILTER_STATE: TimelineFilterState = {
  categories: [],
};

// ============================================================================
// Status Mappings
// ============================================================================

/**
 * Maps filter categories to specific status values
 */
const CATEGORY_TO_STATUSES: Record<TimelineFilterCategory, InternalStatus[] | null> = {
  all: null,
  execution: ["executing", "re_executing"],
  reviews: ["pending_review", "reviewing", "review_passed"],
  escalations: ["escalated", "revision_needed"],
  qa: ["qa_refining", "qa_testing", "qa_passed", "qa_failed"],
  merge: ["pending_merge", "merging", "merge_conflict"],
  completed: ["approved", "merged"],
  blocked: ["blocked"],
  plans: null, // Plan events are not status_change events
};

/**
 * Maps filter categories to event types
 */
const CATEGORY_TO_EVENT_TYPES: Record<TimelineFilterCategory, TimelineEventType[] | null> = {
  all: null,
  execution: ["status_change"],
  reviews: ["status_change"],
  escalations: ["status_change"],
  qa: ["status_change"],
  merge: ["status_change"],
  completed: ["status_change"],
  blocked: ["status_change"],
  plans: ["plan_accepted", "plan_completed"],
};

// ============================================================================
// Filter Functions
// ============================================================================

/**
 * Check if an event matches the given filter categories
 *
 * @param event - The timeline event to check
 * @param categories - Active filter categories (empty = all events pass)
 * @returns true if the event should be included
 */
export function eventMatchesFilters(
  event: TimelineEvent,
  categories: TimelineFilterCategory[]
): boolean {
  // Empty filter = show all
  if (categories.length === 0 || categories.includes("all")) {
    return true;
  }

  // Check each active filter category
  return categories.some((category) => {
    // Plan events filter
    if (category === "plans") {
      return event.eventType === "plan_accepted" || event.eventType === "plan_completed";
    }

    // Status change filters - check the toStatus
    if (event.eventType === "status_change" && event.toStatus) {
      const matchingStatuses = CATEGORY_TO_STATUSES[category];
      if (matchingStatuses) {
        return matchingStatuses.includes(event.toStatus as InternalStatus);
      }
    }

    return false;
  });
}

/**
 * Filter an array of timeline events by categories
 *
 * @param events - Array of timeline events
 * @param categories - Active filter categories
 * @returns Filtered array of events
 */
export function filterTimelineEvents(
  events: TimelineEvent[],
  categories: TimelineFilterCategory[]
): TimelineEvent[] {
  if (categories.length === 0 || categories.includes("all")) {
    return events;
  }
  return events.filter((event) => eventMatchesFilters(event, categories));
}

/**
 * Get the filter category for a timeline event
 * Used for displaying category badges on events
 *
 * @param event - The timeline event
 * @returns The primary filter category for this event
 */
export function getEventCategory(event: TimelineEvent): TimelineFilterCategory {
  // Plan events
  if (event.eventType === "plan_accepted" || event.eventType === "plan_completed") {
    return "plans";
  }

  // Status change events - determine by toStatus
  if (event.eventType === "status_change" && event.toStatus) {
    const status = event.toStatus as InternalStatus;

    // Check escalations first (subset of review)
    if (status === "escalated" || status === "revision_needed") {
      return "escalations";
    }

    // Map status category to filter category
    const statusCategory = getStatusCategory(status);
    switch (statusCategory) {
      case "executing":
        return "execution";
      case "review":
        return "reviews";
      case "qa":
        return "qa";
      case "merge":
        return "merge";
      case "complete":
        return "completed";
      case "blocked":
        return "blocked";
      default:
        return "all";
    }
  }

  return "all";
}

/**
 * Get the color for a filter category
 *
 * @param category - The filter category
 * @returns HSL color string
 */
export function getFilterColor(category: TimelineFilterCategory): string {
  return FILTER_COLORS[category];
}

/**
 * Get filter option by ID
 *
 * @param id - The filter category ID
 * @returns The filter option or undefined
 */
export function getFilterOption(id: TimelineFilterCategory): TimelineFilterOption | undefined {
  return TIMELINE_FILTER_OPTIONS.find((option) => option.id === id);
}

/**
 * Check if any status-based filters are active
 * (excludes "all" and "plans")
 */
export function hasStatusFilters(categories: TimelineFilterCategory[]): boolean {
  return categories.some(
    (cat) => cat !== "all" && cat !== "plans"
  );
}

/**
 * Check if plan events filter is active
 */
export function hasPlanFilter(categories: TimelineFilterCategory[]): boolean {
  return categories.includes("plans");
}

/**
 * Convert filter state to API-compatible filter format
 * Used by useExecutionTimeline hook
 */
export function toApiFilters(
  categories: TimelineFilterCategory[]
): { eventTypes?: TimelineEventType[] } {
  if (categories.length === 0 || categories.includes("all")) {
    return {};
  }

  const eventTypes = new Set<TimelineEventType>();

  for (const category of categories) {
    const types = CATEGORY_TO_EVENT_TYPES[category];
    if (types) {
      types.forEach((t) => eventTypes.add(t));
    }
  }

  // If we have event types, return them; otherwise return empty (all)
  if (eventTypes.size > 0) {
    return { eventTypes: Array.from(eventTypes) };
  }

  return {};
}
