/**
 * Widget registry — maps tool names to specialized widget components.
 *
 * Exports WidgetRenderer (static component) and hasWidget (lookup check).
 */

import React from "react";
import type { ToolCall } from "../ToolCallIndicator";
import { ArtifactWidget } from "./ArtifactWidget";

// ============================================================================
// Types
// ============================================================================

export interface ToolWidgetProps {
  toolCall: ToolCall;
  compact?: boolean;
}

export type ToolWidgetComponent = React.ComponentType<ToolWidgetProps>;

// ============================================================================
// Registry
// ============================================================================

const WIDGET_MAP: Record<string, ToolWidgetComponent> = {
  get_artifact: ArtifactWidget,
  get_artifact_version: ArtifactWidget,
  get_plan_artifact: ArtifactWidget,
};

/** Check if a tool has a specialized widget. */
export function hasWidget(toolName: string): boolean {
  return toolName.toLowerCase() in WIDGET_MAP;
}

/** Static component that resolves and renders the correct widget for a tool name. Returns null if no widget matches. */
export const WidgetRenderer = React.memo(function WidgetRenderer({ toolCall, compact }: ToolWidgetProps) {
  const Component = WIDGET_MAP[toolCall.name.toLowerCase()];
  if (!Component) return null;
  return React.createElement(Component, { toolCall, compact: compact ?? false });
});
