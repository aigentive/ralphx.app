/**
 * Widget Registry - Maps tool names to specialized widget components
 *
 * Lookup: ToolCallIndicator checks this registry before falling back to generic rendering.
 * Pattern: same as TASK_DETAIL_VIEWS registry in task-detail-views.
 */

import type { ComponentType } from "react";
import type { ToolCall } from "../ToolCallIndicator";

/** Props that all widget components receive */
export interface WidgetProps {
  toolCall: ToolCall;
  compact?: boolean;
}

/** Registry of tool name → specialized widget component */
const WIDGET_REGISTRY: Record<string, ComponentType<WidgetProps>> = {};

/**
 * Lazy-load widgets to avoid circular imports and keep bundle splitting.
 * Returns the widget component for a tool name, or null if no specialized widget.
 */
export function getWidgetForTool(toolName: string): ComponentType<WidgetProps> | null {
  return WIDGET_REGISTRY[toolName] ?? null;
}

/**
 * Register a widget for one or more tool names.
 * Called at module level by each widget file.
 */
export function registerWidget(toolNames: string[], component: ComponentType<WidgetProps>): void {
  for (const name of toolNames) {
    WIDGET_REGISTRY[name] = component;
  }
}

/**
 * Check if a tool has a specialized widget registered.
 */
export function hasWidget(toolName: string): boolean {
  return toolName in WIDGET_REGISTRY;
}
