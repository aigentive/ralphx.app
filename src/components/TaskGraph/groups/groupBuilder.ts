import type { Node } from "@xyflow/react";
import type { TaskGraphNode, PlanGroupInfo } from "@/api/task-graph.types";
import {
  calculateGroupBoundingBoxes,
  expandBoundingBox,
  boundingBoxToGroupNode,
  GROUP_PADDING,
  HEADER_HEIGHT,
} from "./groupUtils";
import { createPlanGroupNode, type PlanGroupNode } from "./PlanGroup";
import {
  createTierGroupNode,
  type TierGroupNode,
  TIER_HEADER_HEIGHT,
} from "./TierGroup";
import { UNGROUPED_PLAN_ID, type TierGroupInfo } from "./tierGroupUtils";

/** Collapsed group dimensions */
export const COLLAPSED_GROUP_WIDTH = 420;
export const COLLAPSED_GROUP_HEIGHT = HEADER_HEIGHT + 8;
export const COLLAPSED_TIER_WIDTH = COLLAPSED_GROUP_WIDTH;
export const COLLAPSED_TIER_HEIGHT = TIER_HEADER_HEIGHT + 8;

interface PlanGroupBuilderArgs {
  taskNodes: Node[];
  planGroups: PlanGroupInfo[];
  collapsedPlanIds: Set<string>;
  tierGroups: TierGroupInfo[];
  collapsedTierIds: Set<string>;
  positions: Map<string, { x: number; y: number }>;
  graphNodes: TaskGraphNode[];
  nodeWidth: number;
  nodeHeight: number;
  onToggleCollapse?: (planArtifactId: string) => void;
  onToggleAllTiers?: (planArtifactId: string, action: "expand" | "collapse") => void;
  includeUncategorized?: boolean;
  projectId?: string;
  onNavigateToTask?: (taskId: string) => void;
  onDeletePlan?: (planArtifactId: string) => void;
  onRemoveAll?: (sessionId: string) => void;
}

interface TierGroupBuilderArgs {
  taskNodes: Node[];
  tierGroups: TierGroupInfo[];
  collapsedTierIds: Set<string>;
  collapsedPlanIds: Set<string>;
  planGroupBounds: Map<string, { position: { x: number; y: number }; width: number }>;
  positions: Map<string, { x: number; y: number }>;
  nodeWidth: number;
  nodeHeight: number;
  onToggleCollapse?: (tierGroupId: string) => void;
}

export function buildPlanGroupNodes({
  taskNodes,
  planGroups,
  collapsedPlanIds,
  tierGroups,
  collapsedTierIds,
  positions,
  graphNodes,
  nodeWidth,
  nodeHeight,
  onToggleCollapse,
  onToggleAllTiers,
  includeUncategorized = true,
  projectId,
  onNavigateToTask,
  onDeletePlan,
  onRemoveAll,
}: PlanGroupBuilderArgs): PlanGroupNode[] {
  if (planGroups.length === 0 && !includeUncategorized) {
    return [];
  }

  const groupedTaskIds = new Set<string>();
  for (const pg of planGroups) {
    for (const taskId of pg.taskIds) {
      groupedTaskIds.add(taskId);
    }
  }

  const ungroupedTaskIds: string[] = [];
  for (const node of taskNodes) {
    if (!groupedTaskIds.has(node.id)) {
      ungroupedTaskIds.push(node.id);
    }
  }

  const groupNodes: PlanGroupNode[] = [];

  const tiersByPlan = new Map<string, string[]>();
  for (const tg of tierGroups) {
    const existing = tiersByPlan.get(tg.planArtifactId);
    if (existing) {
      existing.push(tg.id);
    } else {
      tiersByPlan.set(tg.planArtifactId, [tg.id]);
    }
  }

  for (const pg of planGroups) {
    const isCollapsed = collapsedPlanIds.has(pg.planArtifactId);
    const tierGroupIds = tiersByPlan.get(pg.planArtifactId) ?? [];
    const hasTierGroups = tierGroupIds.length > 0;
    const anyTierCollapsed = hasTierGroups
      ? tierGroupIds.some((id) => collapsedTierIds.has(id))
      : false;
    const allTiersCollapsed = hasTierGroups
      ? tierGroupIds.every((id) => collapsedTierIds.has(id))
      : false;

    let position: { x: number; y: number };
    let width: number;
    let height: number;

    if (isCollapsed) {
      const placeholderPos = positions.get(`__group_position_${pg.planArtifactId}__`);
      position = placeholderPos ?? { x: 0, y: 0 };
      width = COLLAPSED_GROUP_WIDTH;
      height = COLLAPSED_GROUP_HEIGHT;
    } else {
      const groupTaskNodes = taskNodes.filter((n) => pg.taskIds.includes(n.id));
      if (groupTaskNodes.length === 0) continue;

      const singleGroupMap = new Map<string, string[]>();
      singleGroupMap.set(pg.planArtifactId, pg.taskIds);
      const boundingBoxes = calculateGroupBoundingBoxes(
        groupTaskNodes,
        singleGroupMap,
        nodeWidth,
        nodeHeight
      );
      const bbox = boundingBoxes[0];
      if (!bbox) continue;

      const expanded = expandBoundingBox(bbox, GROUP_PADDING, HEADER_HEIGHT);
      const groupDims = boundingBoxToGroupNode(expanded);
      position = groupDims.position;
      width = groupDims.width;
      height = groupDims.height;
    }

    const groupNode = createPlanGroupNode(
      pg.planArtifactId,
      pg.sessionId,
      pg.sessionTitle,
      pg.taskIds,
      pg.statusSummary,
      position,
      width,
      height,
      isCollapsed,
      onToggleCollapse,
      hasTierGroups ? tierGroupIds : undefined,
      anyTierCollapsed,
      allTiersCollapsed,
      hasTierGroups && onToggleAllTiers ? onToggleAllTiers : undefined,
      projectId,
      onNavigateToTask,
      onDeletePlan,
      onRemoveAll
    );

    groupNodes.push(groupNode);
  }

  if (includeUncategorized && ungroupedTaskIds.length > 0) {
    const isUngroupedCollapsed = collapsedPlanIds.has(UNGROUPED_PLAN_ID);
    const ungroupedTierIds = tiersByPlan.get(UNGROUPED_PLAN_ID) ?? [];
    const hasTierGroups = ungroupedTierIds.length > 0;
    const anyTierCollapsed = hasTierGroups
      ? ungroupedTierIds.some((id) => collapsedTierIds.has(id))
      : false;
    const allTiersCollapsed = hasTierGroups
      ? ungroupedTierIds.every((id) => collapsedTierIds.has(id))
      : false;

    let position: { x: number; y: number };
    let width: number;
    let height: number;

    if (isUngroupedCollapsed) {
      const placeholderPos = positions.get(`__group_position_${UNGROUPED_PLAN_ID}__`);
      position = placeholderPos ?? { x: 0, y: 0 };
      width = COLLAPSED_GROUP_WIDTH;
      height = COLLAPSED_GROUP_HEIGHT;
    } else {
      const ungroupedTaskNodes = taskNodes.filter((n) => ungroupedTaskIds.includes(n.id));
      if (ungroupedTaskNodes.length === 0) {
        return groupNodes;
      }
      const ungroupedMap = new Map<string, string[]>();
      ungroupedMap.set(UNGROUPED_PLAN_ID, ungroupedTaskIds);
      const ungroupedBoxes = calculateGroupBoundingBoxes(
        ungroupedTaskNodes,
        ungroupedMap,
        nodeWidth,
        nodeHeight
      );
      const ungroupedBbox = ungroupedBoxes[0];
      if (!ungroupedBbox) {
        return groupNodes;
      }
      const expanded = expandBoundingBox(ungroupedBbox, GROUP_PADDING, HEADER_HEIGHT);
      const groupDims = boundingBoxToGroupNode(expanded);
      position = groupDims.position;
      width = groupDims.width;
      height = groupDims.height;
    }

    const ungroupedSummary = {
      backlog: 0,
      ready: 0,
      blocked: 0,
      executing: 0,
      qa: 0,
      review: 0,
      merge: 0,
      completed: 0,
      terminal: 0,
    };

    for (const taskId of ungroupedTaskIds) {
      const node = graphNodes.find((n) => n.taskId === taskId);
      if (!node) continue;

      const status = node.internalStatus;
      if (status === "backlog") ungroupedSummary.backlog++;
      else if (status === "ready") ungroupedSummary.ready++;
      else if (status === "blocked" || status === "paused") ungroupedSummary.blocked++;
      else if (status === "executing" || status === "re_executing") ungroupedSummary.executing++;
      else if (status.startsWith("qa_")) ungroupedSummary.qa++;
      else if (
        status === "pending_review" ||
        status === "reviewing" ||
        status === "review_passed" ||
        status === "escalated" ||
        status === "revision_needed"
      ) {
        ungroupedSummary.review++;
      } else if (status === "approved") ungroupedSummary.merge++;
      else if (status === "merged") ungroupedSummary.completed++;
      else if (status === "failed" || status === "cancelled" || status === "stopped")
        ungroupedSummary.terminal++;
    }

    const groupNode = createPlanGroupNode(
      UNGROUPED_PLAN_ID,
      "",
      "Uncategorized",
      ungroupedTaskIds,
      ungroupedSummary,
      position,
      width,
      height,
      isUngroupedCollapsed,
      onToggleCollapse,
      hasTierGroups ? ungroupedTierIds : undefined,
      anyTierCollapsed,
      allTiersCollapsed,
      hasTierGroups && onToggleAllTiers ? onToggleAllTiers : undefined,
      projectId,
      onNavigateToTask,
      undefined, // onDeletePlan — not applicable for uncategorized
      onRemoveAll
    );

    groupNodes.push(groupNode);
  }

  return groupNodes;
}

export function buildTierGroupNodes({
  taskNodes,
  tierGroups,
  collapsedTierIds,
  collapsedPlanIds,
  planGroupBounds,
  positions,
  nodeWidth,
  nodeHeight,
  onToggleCollapse,
}: TierGroupBuilderArgs): TierGroupNode[] {
  if (tierGroups.length === 0) {
    return [];
  }

  const groupNodes: TierGroupNode[] = [];

  for (const tg of tierGroups) {
    if (collapsedPlanIds.has(tg.planArtifactId)) {
      continue;
    }

    const isCollapsed = collapsedTierIds.has(tg.id);
    let position: { x: number; y: number };
    let width: number;
    let height: number;

    const planBounds = planGroupBounds.get(tg.planArtifactId);
    if (isCollapsed) {
      const placeholderPos = positions.get(`__tier_group_position_${tg.id}__`);
      const desiredWidth =
        planBounds?.width !== undefined
          ? Math.max(planBounds.width - GROUP_PADDING * 2, COLLAPSED_TIER_WIDTH)
          : COLLAPSED_TIER_WIDTH;
      const centeredX =
        planBounds?.position.x !== undefined && planBounds.width !== undefined
          ? planBounds.position.x + (planBounds.width - desiredWidth) / 2
          : placeholderPos?.x ?? 0;
      position = {
        x: centeredX,
        y: placeholderPos?.y ?? 0,
      };
      width = desiredWidth;
      height = COLLAPSED_TIER_HEIGHT;
    } else {
      const tierTaskNodes = taskNodes.filter((n) => tg.taskIds.includes(n.id));
      if (tierTaskNodes.length === 0) continue;

      const singleGroupMap = new Map<string, string[]>();
      singleGroupMap.set(tg.id, tg.taskIds);
      const boundingBoxes = calculateGroupBoundingBoxes(
        tierTaskNodes,
        singleGroupMap,
        nodeWidth,
        nodeHeight
      );
      const bbox = boundingBoxes[0];
      if (!bbox) continue;

      const expanded = expandBoundingBox(bbox, GROUP_PADDING, TIER_HEADER_HEIGHT);
      const groupDims = boundingBoxToGroupNode(expanded);
      const desiredWidth =
        planBounds?.width !== undefined
          ? Math.max(planBounds.width - GROUP_PADDING * 2, groupDims.width)
          : groupDims.width;
      const centeredX =
        planBounds?.position.x !== undefined && planBounds.width !== undefined
          ? planBounds.position.x + (planBounds.width - desiredWidth) / 2
          : groupDims.position.x;
      position = {
        x: centeredX,
        y: groupDims.position.y,
      };
      width = desiredWidth;
      height = groupDims.height;
    }

    groupNodes.push(
      createTierGroupNode(
        tg.id,
        tg.planArtifactId,
        tg.tier,
        tg.taskIds,
        position,
        width,
        height,
        isCollapsed,
        onToggleCollapse
      )
    );
  }

  return groupNodes;
}

export { getGroupNodeId } from "./groupTypes";
