// TaskGraph component exports
export { TaskGraphView } from "./TaskGraphView";
export { useTaskGraph, taskGraphKeys } from "./hooks/useTaskGraph";
export { useTaskGraphLayout, DEFAULT_CONFIG as DEFAULT_LAYOUT_CONFIG } from "./hooks/useTaskGraphLayout";
export type { LayoutConfig, LayoutResult } from "./hooks/useTaskGraphLayout";

// Node components
export { TaskNode } from "./nodes/TaskNode";
export type { TaskNodeData, TaskNodeType } from "./nodes/TaskNode";

// Node styles
export {
  getNodeStyle,
  getStatusBorderColor,
  getStatusBackground,
  getStatusCategory,
  getCategoryColor,
  STATUS_LEGEND_GROUPS,
  CATEGORY_LABELS,
} from "./nodes/nodeStyles";
export type { NodeStyle, StatusCategory, LegendItem } from "./nodes/nodeStyles";

// Edge components
export { DependencyEdge } from "./edges/DependencyEdge";
export type { DependencyEdgeData } from "./edges/DependencyEdge";

// Edge styles
export {
  getEdgeType,
  getEdgeStyle,
  getEdgeStyleForEdge,
  NORMAL_EDGE_STYLE,
  CRITICAL_EDGE_STYLE,
  ACTIVE_EDGE_STYLE,
} from "./edges/edgeStyles";
export type { EdgeStyle, EdgeType } from "./edges/edgeStyles";

// Controls
export { GraphLegend } from "./controls/GraphLegend";
export type { GraphLegendProps } from "./controls/GraphLegend";

// Groups
export { PlanGroupHeader } from "./groups/PlanGroupHeader";
export type { PlanGroupHeaderProps } from "./groups/PlanGroupHeader";

export {
  PlanGroup,
  createPlanGroupNode,
  PLAN_GROUP_NODE_TYPE,
} from "./groups/PlanGroup";
export type { PlanGroupData, PlanGroupNode, PlanGroupProps } from "./groups/PlanGroup";

// Group utilities
export {
  calculateBoundingBox,
  calculateGroupBoundingBoxes,
  expandBoundingBox,
  boundingBoxToGroupNode,
  GROUP_PADDING,
  HEADER_HEIGHT,
  MIN_GROUP_WIDTH,
  MIN_GROUP_HEIGHT,
} from "./groups/groupUtils";
export type { BoundingBox, GroupBoundingBox } from "./groups/groupUtils";
