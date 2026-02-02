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
