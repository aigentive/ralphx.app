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

/** Registry type: tool name (lowercase) → React component */
export type ToolCallWidgetRegistry = Record<string, ComponentType<ToolCallWidgetProps>>;

/**
 * The widget registry. Subsequent tasks will populate this with specialized widgets.
 * Tool names should be lowercase to match normalized lookup in ToolCallIndicator.
 */
export const TOOL_CALL_WIDGETS: ToolCallWidgetRegistry = {
  // Populated by subsequent tasks:
  // "bash": BashWidget,
  // "read": ReadWidget,
  // "grep": GrepWidget,
  // "glob": GlobWidget,
  // "start_step": StepIndicatorWidget,
  // "complete_step": StepIndicatorWidget,
  // etc.
};

/**
 * Look up a specialized widget for a tool name.
 * Returns undefined if no specialized widget is registered.
 */
export function getToolCallWidget(toolName: string): ComponentType<ToolCallWidgetProps> | undefined {
  return TOOL_CALL_WIDGETS[toolName.toLowerCase()];
}
