// TaskGraph component exports
export { TaskGraphView } from "./TaskGraphView";
export { useTaskGraph, taskGraphKeys } from "./hooks/useTaskGraph";
export { useTaskGraphLayout, DEFAULT_CONFIG as DEFAULT_LAYOUT_CONFIG } from "./hooks/useTaskGraphLayout";
export type { LayoutConfig, LayoutResult } from "./hooks/useTaskGraphLayout";

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
