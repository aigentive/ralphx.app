// TypeScript types for plan-branch API (camelCase)

export type PlanBranchStatus = "active" | "merged" | "abandoned";

export interface PlanBranch {
  id: string;
  planArtifactId: string;
  sessionId: string;
  projectId: string;
  branchName: string;
  sourceBranch: string;
  status: PlanBranchStatus;
  mergeTaskId: string | null;
  createdAt: string;
  mergedAt: string | null;
}

export interface EnableFeatureBranchInput {
  planArtifactId: string;
  sessionId: string;
  projectId: string;
}
