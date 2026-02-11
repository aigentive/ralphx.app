// Plan selection and ranking API module
// Wraps Tauri commands for global active plan feature

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// ============================================================================
// Response Schemas (matching Rust backend serialization with snake_case)
// ============================================================================

const TaskStatsSchema = z.object({
  total: z.number(),
  incomplete: z.number(),
  active_now: z.number(),
});

const InteractionStatsSchema = z.object({
  selected_count: z.number(),
  last_selected_at: z.string().nullable(),
});

const ScoreBreakdownSchema = z.object({
  interaction_score: z.number(),
  activity_score: z.number(),
  recency_score: z.number(),
  final_score: z.number(),
});

const PlanCandidateResponseSchema = z.object({
  session_id: z.string(),
  title: z.string().nullable(),
  accepted_at: z.string(),
  task_stats: TaskStatsSchema,
  interaction_stats: InteractionStatsSchema,
  score: z.number(),
  score_breakdown: ScoreBreakdownSchema.nullable(),
});

type TaskStatsResponse = z.infer<typeof TaskStatsSchema>;
type InteractionStatsResponse = z.infer<typeof InteractionStatsSchema>;
type ScoreBreakdownResponse = z.infer<typeof ScoreBreakdownSchema>;
type PlanCandidateResponse = z.infer<typeof PlanCandidateResponseSchema>;

// ============================================================================
// TypeScript Types (camelCase for frontend)
// ============================================================================

export interface TaskStats {
  total: number;
  incomplete: number;
  activeNow: number;
}

export interface InteractionStats {
  selectedCount: number;
  lastSelectedAt: string | null;
}

export interface ScoreBreakdown {
  interactionScore: number;
  activityScore: number;
  recencyScore: number;
  finalScore: number;
}

export interface PlanCandidate {
  sessionId: string;
  title: string | null;
  acceptedAt: string;
  taskStats: TaskStats;
  interactionStats: InteractionStats;
  score: number;
  scoreBreakdown: ScoreBreakdown | null;
}

export type SelectionSource = "kanban_inline" | "graph_inline" | "quick_switcher" | "ideation";

// ============================================================================
// Transform Functions (snake_case -> camelCase)
// ============================================================================

function transformTaskStats(raw: TaskStatsResponse): TaskStats {
  return {
    total: raw.total,
    incomplete: raw.incomplete,
    activeNow: raw.active_now,
  };
}

function transformInteractionStats(raw: InteractionStatsResponse): InteractionStats {
  return {
    selectedCount: raw.selected_count,
    lastSelectedAt: raw.last_selected_at,
  };
}

function transformScoreBreakdown(raw: ScoreBreakdownResponse): ScoreBreakdown {
  return {
    interactionScore: raw.interaction_score,
    activityScore: raw.activity_score,
    recencyScore: raw.recency_score,
    finalScore: raw.final_score,
  };
}

function transformPlanCandidate(raw: PlanCandidateResponse): PlanCandidate {
  return {
    sessionId: raw.session_id,
    title: raw.title,
    acceptedAt: raw.accepted_at,
    taskStats: transformTaskStats(raw.task_stats),
    interactionStats: transformInteractionStats(raw.interaction_stats),
    score: raw.score,
    scoreBreakdown: raw.score_breakdown ? transformScoreBreakdown(raw.score_breakdown) : null,
  };
}

// ============================================================================
// Typed Invoke Helpers
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
 * Plan API wrappers for Tauri commands
 * Handles active plan selection, clearing, and candidate listing
 */
export const planApi = {
  /**
   * Get the active plan for a project
   * @param projectId The project ID
   * @returns The session ID of the active plan, or null if no plan is active
   */
  getActivePlan: (projectId: string) =>
    typedInvoke(
      "get_active_plan",
      { projectId },
      z.string().nullable()
    ),

  /**
   * Set the active plan for a project
   * @param projectId The project ID
   * @param sessionId The ideation session ID to set as active
   * @param source The source of the selection (for analytics)
   */
  setActivePlan: (projectId: string, sessionId: string, source: SelectionSource) =>
    invoke("set_active_plan", {
      projectId,
      ideationSessionId: sessionId,
      source,
    }),

  /**
   * Clear the active plan for a project
   * @param projectId The project ID
   */
  clearActivePlan: (projectId: string) =>
    invoke("clear_active_plan", { projectId }),

  /**
   * List plan candidates for selection
   * @param projectId The project ID
   * @param query Optional search query to filter by title
   * @param limit Optional limit on number of results (default: 50)
   * @returns Array of plan candidates sorted by ranking score
   */
  listCandidates: async (projectId: string, query?: string, limit?: number): Promise<PlanCandidate[]> => {
    const raw = await typedInvoke(
      "list_plan_selector_candidates",
      {
        projectId,
        query: query ?? null,
        limit: limit ?? 50,
      },
      z.array(PlanCandidateResponseSchema)
    );
    return raw.map(transformPlanCandidate);
  },
} as const;
