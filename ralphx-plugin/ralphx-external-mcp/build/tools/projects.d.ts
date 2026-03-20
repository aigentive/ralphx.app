/**
 * Project setup tools for ralphx-external-mcp
 *
 * Handles project registration via v1_register_project.
 * Requires CREATE_PROJECT permission (bit 8).
 */
import type { ApiKeyContext } from "../types.js";
export interface RegisterProjectArgs {
    working_directory: string;
    name?: string;
}
export interface ToolResult {
    text: string;
    isError: boolean;
}
/**
 * Register a folder as a RalphX project.
 * Creates directory if needed, initializes git if needed.
 * Requires CREATE_PROJECT permission.
 */
export declare function handleRegisterProject(args: Record<string, unknown>, context: ApiKeyContext): Promise<ToolResult>;
//# sourceMappingURL=projects.d.ts.map