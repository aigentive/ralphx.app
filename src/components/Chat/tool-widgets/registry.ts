/**
 * Widget Registry — maps tool names to specialized widget components.
 *
 * ToolCallIndicator looks up tools here before falling back to generic rendering.
 */

import type { ComponentType } from "react";
import type { ToolCall } from "../ToolCallIndicator";

export interface ToolWidgetProps {
  toolCall: ToolCall;
  className?: string;
  compact?: boolean;
}

type WidgetComponent = ComponentType<ToolWidgetProps>;

const registry = new Map<string, WidgetComponent>();

/**
 * Register a widget component for one or more tool names.
 */
export function registerWidget(toolNames: string[], component: WidgetComponent): void {
  for (const name of toolNames) {
    registry.set(name, component);
  }
}

/**
 * Look up a specialized widget for a tool name.
 * Returns undefined if no specialized widget is registered.
 */
export function getWidget(toolName: string): WidgetComponent | undefined {
  return registry.get(toolName);
}
