export type ExecutionPlanStatus = "active" | "superseded";

export interface ExecutionPlan {
  id: string;
  sessionId: string;
  status: ExecutionPlanStatus;
  createdAt: string;
}
