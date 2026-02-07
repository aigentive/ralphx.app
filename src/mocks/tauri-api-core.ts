/**
 * Mock implementation of @tauri-apps/api/core for web mode
 *
 * In web mode, invoke() calls go through the api proxy which uses mockApi.
 * This mock provides command handlers that return proper mock data.
 */

import { mockWorkflowsApi, mockProjectsApi, mockGetGitBranches, mockGetGitDefaultBranch } from "@/api-mock/projects";
import { mockTasksApi } from "@/api-mock/tasks";
import { mockTaskGraphApi } from "@/api-mock/task-graph";
import { mockListConversations, mockGetConversation } from "@/api-mock/chat";
import { mockReviewsApi } from "@/api-mock/reviews";
import { mockIdeationApi } from "@/api-mock/ideation";
import { mockExecutionApi } from "@/api-mock/execution";
import { mockPlanBranchApi, toSnakeCasePlanBranch } from "@/api-mock/plan-branch";
import type { ContextType } from "@/types/chat-conversation";

/**
 * Command handlers map - routes Tauri commands to mock implementations
 */
const commandHandlers: Record<
  string,
  (args: Record<string, unknown>) => Promise<unknown>
> = {
  // Workflow commands
  get_active_workflow_columns: async () => {
    const columns = await mockWorkflowsApi.getActiveColumns();
    // Transform to snake_case as backend would return
    return columns.map((col) => ({
      id: col.id,
      name: col.name,
      maps_to: col.mapsTo,
      color: col.color,
      icon: col.icon,
    }));
  },
  list_workflows: async () => mockWorkflowsApi.list(),

  // Project commands
  list_projects: async () => mockProjectsApi.list(),
  get_project: async (args) => mockProjectsApi.get(args.projectId as string),
  get_git_branches: async (args) => mockGetGitBranches(args.workingDirectory as string),
  get_git_default_branch: async (args) => mockGetGitDefaultBranch(args.workingDirectory as string),

  // Task commands
  list_tasks: async (args) => {
    // Build params object, only including defined properties
    const params: {
      projectId: string;
      statuses?: string[];
      offset?: number;
      limit?: number;
      includeArchived?: boolean;
    } = { projectId: args.projectId as string };

    if (args.statuses !== undefined) params.statuses = args.statuses as string[];
    if (args.offset !== undefined) params.offset = args.offset as number;
    if (args.limit !== undefined) params.limit = args.limit as number;
    if (args.includeArchived !== undefined) params.includeArchived = args.includeArchived as boolean;

    const response = await mockTasksApi.list(params);
    // Transform to snake_case as backend would return
    return {
      tasks: response.tasks.map((t) => ({
        id: t.id,
        project_id: t.projectId,
        category: t.category,
        title: t.title,
        description: t.description,
        internal_status: t.internalStatus,
        priority: t.priority,
        needs_review_point: t.needsReviewPoint,
        created_at: t.createdAt,
        updated_at: t.updatedAt,
        started_at: t.startedAt,
        completed_at: t.completedAt,
        archived_at: t.archivedAt,
        blocked_reason: t.blockedReason,
      })),
      total: response.total,
      offset: response.offset,
      has_more: response.hasMore,
    };
  },
  get_tasks_awaiting_review: async (args) => {
    const response = await mockTasksApi.getTasksAwaitingReview(args.project_id as string);
    // Convert to snake_case for Tauri response
    return response.map((task) => ({
      id: task.id,
      title: task.title,
      description: task.description,
      category: task.category,
      priority: task.priority,
      internal_status: task.internalStatus,
      created_at: task.createdAt,
      updated_at: task.updatedAt,
      project_id: task.projectId,
      blocked_reason: task.blockedReason,
    }));
  },

  // Chat commands
  list_agent_conversations: async (args) =>
    mockListConversations(
      args.contextType as ContextType,
      args.contextId as string
    ),
  get_conversation: async (args) =>
    mockGetConversation(args.conversationId as string),

  // Ideation commands
  list_ideation_sessions: async (args) => {
    const sessions = await mockIdeationApi.sessions.list(args.projectId as string);
    // Transform to snake_case as backend would return
    return sessions.map((s) => ({
      id: s.id,
      project_id: s.projectId,
      title: s.title,
      status: s.status,
      plan_artifact_id: s.planArtifactId,
      seed_task_id: s.seedTaskId,
      created_at: s.createdAt,
      updated_at: s.updatedAt,
      archived_at: s.archivedAt,
      converted_at: s.convertedAt,
    }));
  },
  get_ideation_session: async (args) => {
    const session = await mockIdeationApi.sessions.get(args.sessionId as string);
    if (!session) return null;
    // Transform to snake_case as backend would return
    return {
      id: session.id,
      project_id: session.projectId,
      title: session.title,
      status: session.status,
      plan_artifact_id: session.planArtifactId,
      seed_task_id: session.seedTaskId,
      created_at: session.createdAt,
      updated_at: session.updatedAt,
      archived_at: session.archivedAt,
      converted_at: session.convertedAt,
    };
  },
  get_ideation_session_with_data: async (args) => {
    const data = await mockIdeationApi.sessions.getWithData(args.id as string);
    if (!data) return null;
    // Transform to snake_case as backend would return
    return {
      session: {
        id: data.session.id,
        project_id: data.session.projectId,
        title: data.session.title,
        status: data.session.status,
        plan_artifact_id: data.session.planArtifactId,
        seed_task_id: data.session.seedTaskId,
        created_at: data.session.createdAt,
        updated_at: data.session.updatedAt,
        archived_at: data.session.archivedAt,
        converted_at: data.session.convertedAt,
      },
      proposals: data.proposals.map((p) => ({
        id: p.id,
        session_id: p.sessionId,
        title: p.title,
        description: p.description,
        category: p.category,
        steps: p.steps,
        acceptance_criteria: p.acceptanceCriteria,
        suggested_priority: p.suggestedPriority,
        priority_score: p.priorityScore,
        priority_reason: p.priorityReason,
        estimated_complexity: p.estimatedComplexity,
        user_priority: p.userPriority,
        user_modified: p.userModified,
        status: p.status,
        created_task_id: p.createdTaskId,
        plan_artifact_id: p.planArtifactId,
        plan_version_at_creation: p.planVersionAtCreation,
        sort_order: p.sortOrder,
        created_at: p.createdAt,
        updated_at: p.updatedAt,
      })),
      messages: data.messages,
    };
  },
  list_session_proposals: async (args) => {
    const proposals = await mockIdeationApi.proposals.list(args.session_id as string);
    // Transform to snake_case as backend would return
    return proposals.map((p) => ({
      id: p.id,
      session_id: p.sessionId,
      title: p.title,
      description: p.description,
      category: p.category,
      steps: p.steps,
      acceptance_criteria: p.acceptanceCriteria,
      suggested_priority: p.suggestedPriority,
      priority_score: p.priorityScore,
      priority_reason: p.priorityReason,
      estimated_complexity: p.estimatedComplexity,
      user_priority: p.userPriority,
      user_modified: p.userModified,
      status: p.status,
      created_task_id: p.createdTaskId,
      plan_artifact_id: p.planArtifactId,
      plan_version_at_creation: p.planVersionAtCreation,
      sort_order: p.sortOrder,
      created_at: p.createdAt,
      updated_at: p.updatedAt,
    }));
  },

  // Review commands
  list_reviews: async (args) => mockReviewsApi.getPending(args.projectId as string),

  // Task graph commands
  get_task_dependency_graph: async (args) =>
    mockTaskGraphApi.getDependencyGraph(args.projectId as string),
  get_task_timeline_events: async (args) =>
    mockTaskGraphApi.getTimelineEvents(
      args.projectId as string,
      (args.limit as number | undefined) ?? 50,
      (args.offset as number | undefined) ?? 0
    ),

  // Execution commands (Phase 82)
  get_execution_status: async (args) => {
    const status = await mockExecutionApi.getStatus(args.projectId as string | undefined);
    // Transform to snake_case as backend would return
    return {
      is_paused: status.isPaused,
      running_count: status.runningCount,
      max_concurrent: status.maxConcurrent,
      global_max_concurrent: status.globalMaxConcurrent,
      queued_count: status.queuedCount,
      can_start_task: status.canStartTask,
    };
  },
  pause_execution: async (args) => {
    const response = await mockExecutionApi.pause(args.projectId as string | undefined);
    return {
      success: response.success,
      status: {
        is_paused: response.status.isPaused,
        running_count: response.status.runningCount,
        max_concurrent: response.status.maxConcurrent,
        global_max_concurrent: response.status.globalMaxConcurrent,
        queued_count: response.status.queuedCount,
        can_start_task: response.status.canStartTask,
      },
    };
  },
  resume_execution: async (args) => {
    const response = await mockExecutionApi.resume(args.projectId as string | undefined);
    return {
      success: response.success,
      status: {
        is_paused: response.status.isPaused,
        running_count: response.status.runningCount,
        max_concurrent: response.status.maxConcurrent,
        global_max_concurrent: response.status.globalMaxConcurrent,
        queued_count: response.status.queuedCount,
        can_start_task: response.status.canStartTask,
      },
    };
  },
  stop_execution: async (args) => {
    const response = await mockExecutionApi.stop(args.projectId as string | undefined);
    return {
      success: response.success,
      status: {
        is_paused: response.status.isPaused,
        running_count: response.status.runningCount,
        max_concurrent: response.status.maxConcurrent,
        global_max_concurrent: response.status.globalMaxConcurrent,
        queued_count: response.status.queuedCount,
        can_start_task: response.status.canStartTask,
      },
    };
  },
  get_execution_settings: async (args) => {
    const settings = await mockExecutionApi.getSettings(args.projectId as string | undefined);
    // Transform to snake_case as backend would return
    return {
      max_concurrent_tasks: settings.maxConcurrentTasks,
      auto_commit: settings.autoCommit,
      pause_on_failure: settings.pauseOnFailure,
    };
  },
  update_execution_settings: async (args) => {
    const input = args.input as { max_concurrent_tasks: number; auto_commit: boolean; pause_on_failure: boolean };
    const settings = await mockExecutionApi.updateSettings({
      maxConcurrentTasks: input.max_concurrent_tasks,
      autoCommit: input.auto_commit,
      pauseOnFailure: input.pause_on_failure,
    }, args.projectId as string | undefined);
    return {
      max_concurrent_tasks: settings.maxConcurrentTasks,
      auto_commit: settings.autoCommit,
      pause_on_failure: settings.pauseOnFailure,
    };
  },
  set_active_project: async (args) => {
    await mockExecutionApi.setActiveProject(args.projectId as string | undefined);
  },
  get_global_execution_settings: async () => {
    const settings = await mockExecutionApi.getGlobalSettings();
    // Transform to snake_case as backend would return
    return {
      global_max_concurrent: settings.globalMaxConcurrent,
    };
  },
  update_global_execution_settings: async (args) => {
    const input = args.input as { global_max_concurrent: number };
    const settings = await mockExecutionApi.updateGlobalSettings({
      globalMaxConcurrent: input.global_max_concurrent,
    });
    return {
      global_max_concurrent: settings.globalMaxConcurrent,
    };
  },

  // Plan branch commands
  get_plan_branch: async (args) => {
    const branch = await mockPlanBranchApi.getByPlan(args.planArtifactId as string);
    return branch ? toSnakeCasePlanBranch(branch) : null;
  },
  get_project_plan_branches: async (args) => {
    const branches = await mockPlanBranchApi.getByProject(args.projectId as string);
    return branches.map(toSnakeCasePlanBranch);
  },
  enable_feature_branch: async (args) => {
    const input = args.input as { plan_artifact_id: string; session_id: string; project_id: string };
    const branch = await mockPlanBranchApi.enable({
      planArtifactId: input.plan_artifact_id,
      sessionId: input.session_id,
      projectId: input.project_id,
    });
    return toSnakeCasePlanBranch(branch);
  },
  disable_feature_branch: async (args) =>
    mockPlanBranchApi.disable(args.planArtifactId as string),
  update_project_feature_branch_setting: async (args) =>
    mockPlanBranchApi.updateProjectSetting(
      args.projectId as string,
      args.enabled as boolean
    ),

  // Health check
  health_check: async () => ({ status: "ok" }),
};

/**
 * Mock invoke function
 *
 * Routes commands to appropriate mock handlers.
 * Falls back to returning empty/null for unknown commands.
 * Respects window.__mockInvokeDelay for testing loading states.
 */
export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  // Add delay if configured (for testing loading states)
  const delay = (window as Window & { __mockInvokeDelay?: number }).__mockInvokeDelay;
  if (delay && delay > 0) {
    await new Promise((resolve) => setTimeout(resolve, delay));
  }

  const handler = commandHandlers[cmd];

  if (handler) {
    console.debug(`[mock] invoke("${cmd}") - using mock handler`);
    const result = await handler(args ?? {});
    return result as T;
  }

  // Unknown command - log warning and return sensible defaults
  console.debug(`[mock] invoke("${cmd}", ${JSON.stringify(args)}) - no handler`);
  console.warn(
    `[web-mode] No mock handler for "${cmd}". ` +
      `Add handler to tauri-api-core.ts or use api.* methods.`
  );

  // Return empty arrays for list commands, null otherwise
  if (cmd.startsWith("list_") || cmd.startsWith("get_all_")) {
    return [] as T;
  }
  return null as T;
}

/**
 * Mock transformCallback - used internally by Tauri for callbacks
 */
export function transformCallback<T>(
  callback?: (response: T) => void,
  _once?: boolean
): number {
  if (callback) {
    console.debug("[mock] transformCallback registered");
  }
  return 0;
}

/**
 * Mock Channel class - used for streaming responses
 */
export class Channel<T = unknown> {
  id: number = 0;
  private _onmessage: ((response: T) => void) | undefined;

  set onmessage(handler: (response: T) => void) {
    this._onmessage = handler;
  }

  get onmessage(): ((response: T) => void) | undefined {
    return this._onmessage;
  }

  toJSON(): string {
    return `__CHANNEL__:${this.id}`;
  }
}

/**
 * Mock Resource class - used for managed resources
 */
export class Resource {
  readonly rid: number;

  constructor(rid: number) {
    this.rid = rid;
  }

  async close(): Promise<void> {
    console.debug(`[mock] Resource.close(${this.rid})`);
  }
}

/**
 * Mock PluginListener - used for plugin event listeners
 */
export class PluginListener {
  plugin: string;
  event: string;
  channelId: number;

  constructor(plugin: string, event: string, channelId: number) {
    this.plugin = plugin;
    this.event = event;
    this.channelId = channelId;
  }

  async unregister(): Promise<void> {
    console.debug(`[mock] PluginListener.unregister(${this.plugin}:${this.event})`);
  }
}

/**
 * Mock addPluginListener - register plugin event listeners
 */
export async function addPluginListener<T>(
  plugin: string,
  event: string,
  _handler: (payload: T) => void
): Promise<PluginListener> {
  console.debug(`[mock] addPluginListener(${plugin}, ${event})`);
  return new PluginListener(plugin, event, 0);
}

/**
 * Mock isTauri - always returns false in web mode
 */
export function isTauri(): boolean {
  return false;
}

/**
 * Mock convertFileSrc - returns the path as-is (can't convert without Tauri)
 */
export function convertFileSrc(filePath: string, _protocol?: string): string {
  console.debug(`[mock] convertFileSrc(${filePath}) - returning path as-is`);
  return filePath;
}
