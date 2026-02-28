import { invoke } from "@tauri-apps/api/core";

export const executionPlanApi = {
  getActiveExecutionPlan: (projectId: string): Promise<string | null> =>
    invoke<string | null>("get_active_execution_plan", { projectId }),
} as const;
