import { useQuery } from "@tanstack/react-query";
import { planBranchApi } from "@/api/plan-branch";
import type { PlanBranch } from "@/api/plan-branch.types";

export const planBranchKeys = {
  all: ["plan-branch"] as const,
  byTask: (taskId: string) => [...planBranchKeys.all, "task", taskId] as const,
};

export function usePlanBranchForTask(taskId: string, options?: { enabled?: boolean }) {
  return useQuery<PlanBranch | null>({
    queryKey: planBranchKeys.byTask(taskId),
    queryFn: () => planBranchApi.getByTaskId(taskId),
    staleTime: 10000,
    refetchInterval: 15000, // Poll PR status every 15s for live views
    enabled: options?.enabled !== false,
  });
}
