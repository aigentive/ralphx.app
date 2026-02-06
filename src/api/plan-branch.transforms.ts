// Transform functions for plan-branch API (snake_case -> camelCase)

import type { z } from "zod";
import type { PlanBranchSchema } from "./plan-branch.schemas";
import type { PlanBranch } from "./plan-branch.types";

type RawPlanBranch = z.infer<typeof PlanBranchSchema>;

export function transformPlanBranch(raw: RawPlanBranch): PlanBranch {
  return {
    id: raw.id,
    planArtifactId: raw.plan_artifact_id,
    sessionId: raw.session_id,
    projectId: raw.project_id,
    branchName: raw.branch_name,
    sourceBranch: raw.source_branch,
    status: raw.status,
    mergeTaskId: raw.merge_task_id,
    createdAt: raw.created_at,
    mergedAt: raw.merged_at,
  };
}
