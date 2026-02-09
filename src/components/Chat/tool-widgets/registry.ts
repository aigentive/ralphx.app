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
import { StepIndicator } from "./StepIndicator";
import { MergeWidget } from "./MergeWidget";

/** Registry type: tool name (lowercase) → React component */
export type ToolCallWidgetRegistry = Record<string, ComponentType<ToolCallWidgetProps>>;

/**
 * The widget registry. Maps tool names to specialized widget components.
 * Tool names should be lowercase to match normalized lookup in ToolCallIndicator.
 */
export const TOOL_CALL_WIDGETS: ToolCallWidgetRegistry = {
  // Step lifecycle tools → StepIndicator (ultra-compact inline indicators)
  "mcp__ralphx__start_step": StepIndicator,
  "mcp__ralphx__complete_step": StepIndicator,
  "mcp__ralphx__add_step": StepIndicator,
  "mcp__ralphx__skip_step": StepIndicator,
  "mcp__ralphx__fail_step": StepIndicator,
  "mcp__ralphx__get_step_progress": StepIndicator,
  // Merge tools → MergeWidget (success/conflict/incomplete cards + merge target)
  "mcp__ralphx__complete_merge": MergeWidget,
  "mcp__ralphx__report_conflict": MergeWidget,
  "mcp__ralphx__report_incomplete": MergeWidget,
  "mcp__ralphx__get_merge_target": MergeWidget,
  // Subsequent tasks will add:
  // "bash": BashWidget,
  // "read": ReadWidget,
  // "grep": GrepWidget,
  // "glob": GlobWidget,
  // etc.
};

/**
 * Look up a specialized widget for a tool name.
 * Returns undefined if no specialized widget is registered.
 */
export function getToolCallWidget(toolName: string): ComponentType<ToolCallWidgetProps> | undefined {
  return TOOL_CALL_WIDGETS[toolName.toLowerCase()];
}
