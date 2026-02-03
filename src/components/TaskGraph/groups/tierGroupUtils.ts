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

function collectUngroupedTaskIds(nodes: TaskGraphNode[], planGroups: PlanGroupInfo[]): string[] {
  const groupedTaskIds = new Set<string>();
  for (const pg of planGroups) {
    for (const taskId of pg.taskIds) {
      groupedTaskIds.add(taskId);
    }
  }

  return nodes.filter((node) => !groupedTaskIds.has(node.taskId)).map((node) => node.taskId);
}

/**
 * Build tier groups for plans (and ungrouped tasks) when multiple tiers exist.
 * Plans with a single tier return no tier groups.
 */
export function buildTierGroups(
  nodes: TaskGraphNode[],
  planGroups: PlanGroupInfo[]
): TierGroupInfo[] {
  const nodeMap = new Map<string, TaskGraphNode>();
  for (const node of nodes) {
    nodeMap.set(node.taskId, node);
  }

  const planEntries = planGroups.map((pg) => ({
    planArtifactId: pg.planArtifactId,
    taskIds: pg.taskIds,
  }));

  const ungroupedTaskIds = collectUngroupedTaskIds(nodes, planGroups);
  if (ungroupedTaskIds.length > 0) {
    planEntries.push({
      planArtifactId: UNGROUPED_PLAN_ID,
      taskIds: ungroupedTaskIds,
    });
  }

  const tierGroups: TierGroupInfo[] = [];

  for (const planEntry of planEntries) {
    const tiers = new Map<number, string[]>();
    for (const taskId of planEntry.taskIds) {
      const node = nodeMap.get(taskId);
      if (!node) continue;
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
