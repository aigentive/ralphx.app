/**
 * Discovery tool handlers — Flow 1 (Phase 4)
 *
 * These tools expose project/pipeline read-only data to external API key holders.
 * All backend calls go to :3847; the BackendClient injects the project scope header.
 */
import type { ApiKeyContext } from "../types.js";
/**
 * v1_list_projects — list projects accessible to this API key.
 * GET /api/external/projects
 * Backend filters by X-RalphX-Project-Scope header.
 */
export declare function handleListProjects(_args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_get_project_status — get project details, task counts, and running agent status.
 * GET /api/external/project/:project_id/status
 */
export declare function handleGetProjectStatus(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_get_pipeline_overview — get tasks grouped by pipeline stage with counts.
 * GET /api/external/pipeline/:project_id
 */
export declare function handleGetPipelineOverview(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
//# sourceMappingURL=discovery.d.ts.map