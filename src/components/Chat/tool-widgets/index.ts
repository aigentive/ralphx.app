/**
 * Tool Widgets - Entry point
 *
 * Imports all widgets and registers them in the registry.
 * This file must be imported once to activate widget registrations.
 */

export { getWidgetForTool, hasWidget } from "./registry";
export type { WidgetProps } from "./registry";

// Register widgets
import { registerWidget } from "./registry";
import { StepsManifestWidget } from "./StepsManifestWidget";
import { IssuesSummaryWidget } from "./IssuesSummaryWidget";

registerWidget(["get_task_steps"], StepsManifestWidget);
registerWidget(["get_task_issues"], IssuesSummaryWidget);
