/**
 * Tool Call Widget Registry
 *
 * Maps tool names to specialized React widget components.
 * ToolCallIndicator checks this registry before falling back to the generic renderer.
 *
 * To register a new widget:
 *   1. Create src/components/Chat/tool-widgets/YourWidget.tsx implementing ToolCallWidgetProps
 *   2. Import and add to TOOL_CALL_WIDGETS below
 */

import type { ComponentType } from "react";
import type { ToolCallWidgetProps } from "./shared";
import { MergeWidget } from "./MergeWidget";

/** Registry type: tool name (lowercase) → React component */
export type ToolCallWidgetRegistry = Record<string, ComponentType<ToolCallWidgetProps>>;

/**
 * The widget registry. Maps tool names to specialized widget components.
 * Tool names should be lowercase to match normalized lookup in ToolCallIndicator.
 */
export const TOOL_CALL_WIDGETS: ToolCallWidgetRegistry = {
  // Merge tools → MergeWidget (success/conflict/incomplete cards + merge target)
  "mcp__ralphx__complete_merge": MergeWidget,
  "mcp__ralphx__report_conflict": MergeWidget,
  "mcp__ralphx__report_incomplete": MergeWidget,
  "mcp__ralphx__get_merge_target": MergeWidget,
  // Subsequent tasks will add:
  // StepIndicator (start_step, complete_step, add_step, skip_step, fail_step, get_step_progress)
  // BashWidget, ReadWidget, GrepWidget, GlobWidget, etc.
};

/**
 * Look up a specialized widget for a tool name.
 * Returns undefined if no specialized widget is registered.
 */
export function getToolCallWidget(toolName: string): ComponentType<ToolCallWidgetProps> | undefined {
  return TOOL_CALL_WIDGETS[toolName.toLowerCase()];
}
