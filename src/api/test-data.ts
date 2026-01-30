// Tauri invoke wrappers for test data operations with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// ============================================================================
// Response Schemas (inline - simple structures)
// ============================================================================

/**
 * Seed response schema (snake_case from backend)
 * Note: Backend uses camelCase for this specific response (no transform needed)
 */
const SeedResponseSchema = z.object({
  profile: z.string(),
  projectId: z.string(),
  projectName: z.string(),
  tasksCreated: z.number(),
  sessionsCreated: z.number(),
  proposalsCreated: z.number(),
});

/**
 * Seed response type
 */
export type SeedResponse = z.infer<typeof SeedResponseSchema>;

/**
 * Test data profile type
 */
export type TestDataProfile = "minimal" | "kanban" | "ideation" | "full";

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
 * Test data API wrappers for Tauri commands
 */
export const testDataApi = {
  /**
   * Seed test data with specified profile
   * @param profile - "minimal" | "kanban" | "ideation" | "full" (default: kanban)
   * @returns Seed response with counts
   */
  seed: (profile?: TestDataProfile): Promise<SeedResponse> =>
    typedInvoke("seed_test_data", { profile }, SeedResponseSchema),

  /**
   * Seed demo data for visual audits (alias for seed("kanban"))
   * Creates a test project with sample tasks in various states
   * @returns Seed response with project info and task count
   */
  seedVisualAudit: (): Promise<SeedResponse> =>
    typedInvoke("seed_visual_audit_data", {}, SeedResponseSchema),

  /**
   * Clear all test data
   * @returns Confirmation message
   */
  clear: (): Promise<string> => typedInvoke("clear_test_data", {}, z.string()),
} as const;
