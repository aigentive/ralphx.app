// Zod schemas for plan-branch API - matches Rust response format (snake_case)

import { z } from "zod";

export const PlanBranchStatusSchema = z.enum(["active", "merged", "abandoned"]);

export const PlanBranchSchema = z.object({
  id: z.string(),
  plan_artifact_id: z.string(),
  session_id: z.string(),
  project_id: z.string(),
  branch_name: z.string(),
  source_branch: z.string(),
  status: PlanBranchStatusSchema,
  merge_task_id: z.string().nullable(),
  created_at: z.string(),
  merged_at: z.string().nullable(),
});

export const PlanBranchListSchema = z.array(PlanBranchSchema);

export const PlanBranchNullableSchema = PlanBranchSchema.nullable();
