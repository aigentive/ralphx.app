import type { ActiveStreamingTaskResponse } from "@/api/chat";
import type { StreamingTask } from "@/types/streaming-task";

function mapActiveTaskToStreamingTask(
  task: ActiveStreamingTaskResponse,
  existing?: StreamingTask,
): StreamingTask {
  const isDelegated = task.delegated_job_id != null;
  const status = task.status as StreamingTask["status"];

  return {
    toolUseId: task.tool_use_id,
    toolName: existing?.toolName ?? (isDelegated ? "delegate_start" : "Task"),
    description: task.description ?? existing?.description ?? "",
    subagentType:
      task.subagent_type
      ?? existing?.subagentType
      ?? (isDelegated ? "delegated" : "unknown"),
    model:
      task.model
      ?? task.effective_model_id
      ?? task.logical_model
      ?? existing?.model
      ?? "unknown",
    status,
    startedAt: existing?.startedAt ?? Date.now(),
    childToolCalls: existing?.childToolCalls ?? [],
    ...(existing?.completedAt != null ? { completedAt: existing.completedAt } : {}),
    ...(task.total_tokens != null
      ? { totalTokens: task.total_tokens }
      : existing?.totalTokens != null
        ? { totalTokens: existing.totalTokens }
        : {}),
    ...(task.total_tool_uses != null
      ? { totalToolUseCount: task.total_tool_uses }
      : existing?.totalToolUseCount != null
        ? { totalToolUseCount: existing.totalToolUseCount }
        : {}),
    ...(task.duration_ms != null
      ? { totalDurationMs: task.duration_ms }
      : existing?.totalDurationMs != null
        ? { totalDurationMs: existing.totalDurationMs }
        : {}),
    ...(task.agent_id != null ? { agentId: task.agent_id } : existing?.agentId ? { agentId: existing.agentId } : {}),
    ...(task.delegated_job_id != null
      ? { delegatedJobId: task.delegated_job_id }
      : existing?.delegatedJobId
        ? { delegatedJobId: existing.delegatedJobId }
        : {}),
    ...(task.delegated_session_id != null
      ? { delegatedSessionId: task.delegated_session_id }
      : existing?.delegatedSessionId
        ? { delegatedSessionId: existing.delegatedSessionId }
        : {}),
    ...(task.delegated_conversation_id != null
      ? { delegatedConversationId: task.delegated_conversation_id }
      : existing?.delegatedConversationId
        ? { delegatedConversationId: existing.delegatedConversationId }
        : {}),
    ...(task.delegated_agent_run_id != null
      ? { delegatedAgentRunId: task.delegated_agent_run_id }
      : existing?.delegatedAgentRunId
        ? { delegatedAgentRunId: existing.delegatedAgentRunId }
        : {}),
    ...(task.provider_harness != null
      ? { providerHarness: task.provider_harness }
      : existing?.providerHarness
        ? { providerHarness: existing.providerHarness }
        : {}),
    ...(task.provider_session_id != null
      ? { providerSessionId: task.provider_session_id }
      : existing?.providerSessionId
        ? { providerSessionId: existing.providerSessionId }
        : {}),
    ...(task.upstream_provider != null
      ? { upstreamProvider: task.upstream_provider }
      : existing?.upstreamProvider
        ? { upstreamProvider: existing.upstreamProvider }
        : {}),
    ...(task.provider_profile != null
      ? { providerProfile: task.provider_profile }
      : existing?.providerProfile
        ? { providerProfile: existing.providerProfile }
        : {}),
    ...(task.logical_model != null
      ? { logicalModel: task.logical_model }
      : existing?.logicalModel
        ? { logicalModel: existing.logicalModel }
        : {}),
    ...(task.effective_model_id != null
      ? { effectiveModelId: task.effective_model_id }
      : existing?.effectiveModelId
        ? { effectiveModelId: existing.effectiveModelId }
        : {}),
    ...(task.logical_effort != null
      ? { logicalEffort: task.logical_effort }
      : existing?.logicalEffort
        ? { logicalEffort: existing.logicalEffort }
        : {}),
    ...(task.effective_effort != null
      ? { effectiveEffort: task.effective_effort }
      : existing?.effectiveEffort
        ? { effectiveEffort: existing.effectiveEffort }
        : {}),
    ...(task.approval_policy != null
      ? { approvalPolicy: task.approval_policy }
      : existing?.approvalPolicy
        ? { approvalPolicy: existing.approvalPolicy }
        : {}),
    ...(task.sandbox_mode != null
      ? { sandboxMode: task.sandbox_mode }
      : existing?.sandboxMode
        ? { sandboxMode: existing.sandboxMode }
        : {}),
    ...(task.input_tokens != null
      ? { inputTokens: task.input_tokens }
      : existing?.inputTokens != null
        ? { inputTokens: existing.inputTokens }
        : {}),
    ...(task.output_tokens != null
      ? { outputTokens: task.output_tokens }
      : existing?.outputTokens != null
        ? { outputTokens: existing.outputTokens }
        : {}),
    ...(task.cache_creation_tokens != null
      ? { cacheCreationTokens: task.cache_creation_tokens }
      : existing?.cacheCreationTokens != null
        ? { cacheCreationTokens: existing.cacheCreationTokens }
        : {}),
    ...(task.cache_read_tokens != null
      ? { cacheReadTokens: task.cache_read_tokens }
      : existing?.cacheReadTokens != null
        ? { cacheReadTokens: existing.cacheReadTokens }
        : {}),
    ...(task.estimated_usd != null
      ? { estimatedUsd: task.estimated_usd }
      : existing?.estimatedUsd != null
        ? { estimatedUsd: existing.estimatedUsd }
        : {}),
    ...(task.text_output != null
      ? { textOutput: task.text_output }
      : existing?.textOutput
        ? { textOutput: existing.textOutput }
        : {}),
    ...(existing?.seq != null ? { seq: existing.seq } : {}),
  };
}

export function mergeActiveStreamingTasks(
  previous: Map<string, StreamingTask>,
  tasks: ActiveStreamingTaskResponse[],
): Map<string, StreamingTask> {
  if (tasks.length === 0) {
    return previous;
  }

  const next = new Map(previous);
  for (const task of tasks) {
    next.set(task.tool_use_id, mapActiveTaskToStreamingTask(task, previous.get(task.tool_use_id)));
  }
  return next;
}
