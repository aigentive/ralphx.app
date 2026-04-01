import type { PlanGroupInfo, TaskGraphNode } from "@/api/task-graph.types";

export const UNGROUPED_PLAN_ID = "__ungrouped__";

export interface TierGroupInfo {
  id: string;
  planArtifactId: string;
  tier: number;
  taskIds: string[];
}

export function getTierGroupId(planArtifactId: string, tier: number): string {
  return `tier-${planArtifactId}-${tier}`;
}

export function getTierLabel(tier: number): string {
  switch (tier) {
    case 0:
      return "Foundation";
    case 1:
      return "Core";
    default:
      return "Integration";
  }
}

/**
 * Build tier groups for plan groups when multiple tiers exist.
 * Plans with a single tier return no tier groups.
 * Uncategorized tasks are always flat (no tier sub-groups).
 */
export function buildTierGroups(
  nodes: TaskGraphNode[],
  planGroups: PlanGroupInfo[],
  options: { enabled?: boolean; includeUngrouped?: boolean } = {}
): TierGroupInfo[] {
  if (options.enabled === false) return [];

  const nodeMap = new Map<string, TaskGraphNode>();
  for (const node of nodes) {
    nodeMap.set(node.taskId, node);
  }

  const planEntries = planGroups.map((pg) => ({
    planArtifactId: pg.planArtifactId,
    taskIds: pg.taskIds,
  }));

  const tierGroups: TierGroupInfo[] = [];

  for (const planEntry of planEntries) {
    const tiers = new Map<number, string[]>();
    for (const taskId of planEntry.taskIds) {
      const node = nodeMap.get(taskId);
      if (!node) continue;
      if (node.category === "plan_merge") continue;
      const tierTasks = tiers.get(node.tier);
      if (tierTasks) {
        tierTasks.push(taskId);
      } else {
        tiers.set(node.tier, [taskId]);
      }
    }

    const tierNumbers = [...tiers.keys()].sort((a, b) => a - b);
    if (tierNumbers.length <= 1) continue;

    for (const tier of tierNumbers) {
      const taskIds = tiers.get(tier) ?? [];
      if (taskIds.length === 0) continue;
      tierGroups.push({
        id: getTierGroupId(planEntry.planArtifactId, tier),
        planArtifactId: planEntry.planArtifactId,
        tier,
        taskIds,
      });
    }
  }

  return tierGroups;
}
