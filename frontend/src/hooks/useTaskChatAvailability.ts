import { useQuery } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import { chatKeys } from "@/hooks/useChat";
import { useTasks, taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import { buildStoreKey } from "@/lib/chat-context-registry";
import { useChatStore, selectIsAgentRunning } from "@/stores/chatStore";
import { useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import type { Task } from "@/types/task";
import type { InternalStatus } from "@/types/status";
import { MERGE_STATUSES } from "@/types/status";

const EMPTY_TASKS: Task[] = [];

const LIVE_EXECUTION_CHAT_STATUSES = [
  "executing",
  "re_executing",
  "qa_refining",
  "qa_testing",
] as const satisfies readonly InternalStatus[];

const EXECUTION_RESULT_CHAT_STATUSES = [
  "qa_passed",
  "qa_failed",
] as const satisfies readonly InternalStatus[];

const LIVE_REVIEW_CHAT_STATUSES = [
  "pending_review",
  "reviewing",
] as const satisfies readonly InternalStatus[];

const REVIEW_RESULT_CHAT_STATUSES = [
  "review_passed",
  "escalated",
  "approved",
] as const satisfies readonly InternalStatus[];

function includesStatus(
  statuses: readonly InternalStatus[],
  status: InternalStatus | undefined
) {
  return status ? statuses.includes(status) : false;
}

export interface TaskChatVisibilityState {
  status?: InternalStatus | undefined;
  isHistoryMode?: boolean;
  hasHistoryConversation?: boolean;
  executionAgentRunning?: boolean;
  reviewAgentRunning?: boolean;
  mergeAgentRunning?: boolean;
  executionConversationCount?: number;
  reviewConversationCount?: number;
  mergeConversationCount?: number;
}

export function shouldShowTaskChatForState({
  status,
  isHistoryMode = false,
  hasHistoryConversation = false,
  executionAgentRunning = false,
  reviewAgentRunning = false,
  mergeAgentRunning = false,
  executionConversationCount = 0,
  reviewConversationCount = 0,
  mergeConversationCount = 0,
}: TaskChatVisibilityState): boolean {
  if (isHistoryMode) {
    return hasHistoryConversation;
  }

  if (
    executionAgentRunning ||
    includesStatus(LIVE_EXECUTION_CHAT_STATUSES, status)
  ) {
    return true;
  }

  if (
    includesStatus(EXECUTION_RESULT_CHAT_STATUSES, status) &&
    executionConversationCount > 0
  ) {
    return true;
  }

  if (
    reviewAgentRunning ||
    includesStatus(LIVE_REVIEW_CHAT_STATUSES, status)
  ) {
    return true;
  }

  if (
    includesStatus(REVIEW_RESULT_CHAT_STATUSES, status) &&
    reviewConversationCount > 0
  ) {
    return true;
  }

  return mergeAgentRunning || mergeConversationCount > 0;
}

export function useTaskChatAvailability(projectId: string): boolean {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const taskHistoryState = useUiStore((s) => s.taskHistoryState);
  const isHistoryMode = taskHistoryState !== null;
  const hasHistoryConversation = Boolean(taskHistoryState?.conversationId);

  const { data: tasks = EMPTY_TASKS } = useTasks(projectId, {
    enabled: Boolean(selectedTaskId),
  });

  const taskFromStore = useTaskStore((state) =>
    selectedTaskId ? state.tasks[selectedTaskId] : undefined
  );
  const taskFromList = selectedTaskId
    ? tasks.find((task) => task.id === selectedTaskId)
    : undefined;

  const { data: taskFromDetail } = useQuery<Task, Error>({
    queryKey: taskKeys.detail(selectedTaskId ?? ""),
    queryFn: () => api.tasks.get(selectedTaskId!),
    enabled: Boolean(selectedTaskId) && !taskFromStore && !taskFromList,
  });

  const selectedTask = taskFromStore ?? taskFromList ?? taskFromDetail;
  const status = taskHistoryState?.status ?? selectedTask?.internalStatus;

  const executionKey = selectedTaskId
    ? buildStoreKey("task_execution", selectedTaskId)
    : "";
  const reviewKey = selectedTaskId ? buildStoreKey("review", selectedTaskId) : "";
  const mergeKey = selectedTaskId ? buildStoreKey("merge", selectedTaskId) : "";

  const executionAgentRunning = useChatStore(selectIsAgentRunning(executionKey));
  const reviewAgentRunning = useChatStore(selectIsAgentRunning(reviewKey));
  const mergeAgentRunning = useChatStore(selectIsAgentRunning(mergeKey));

  const shouldCheckExecutionConversations =
    Boolean(selectedTaskId) &&
    !isHistoryMode &&
    includesStatus(EXECUTION_RESULT_CHAT_STATUSES, status);

  const shouldCheckReviewConversations =
    Boolean(selectedTaskId) &&
    !isHistoryMode &&
    includesStatus(REVIEW_RESULT_CHAT_STATUSES, status);

  const shouldCheckMergeConversations =
    Boolean(selectedTaskId) &&
    !isHistoryMode &&
    (mergeAgentRunning || includesStatus(MERGE_STATUSES, status));

  const executionConversations = useQuery({
    queryKey: chatKeys.conversationList("task_execution", selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations("task_execution", selectedTaskId!),
    enabled: shouldCheckExecutionConversations,
    staleTime: 0,
  });

  const reviewConversations = useQuery({
    queryKey: chatKeys.conversationList("review", selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations("review", selectedTaskId!),
    enabled: shouldCheckReviewConversations,
    staleTime: 0,
  });

  const mergeConversations = useQuery({
    queryKey: chatKeys.conversationList("merge", selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations("merge", selectedTaskId!),
    enabled: shouldCheckMergeConversations,
    staleTime: 0,
  });

  if (!selectedTaskId) {
    return false;
  }

  return shouldShowTaskChatForState({
    status,
    isHistoryMode,
    hasHistoryConversation,
    executionAgentRunning,
    reviewAgentRunning,
    mergeAgentRunning,
    executionConversationCount: executionConversations.data?.length ?? 0,
    reviewConversationCount: reviewConversations.data?.length ?? 0,
    mergeConversationCount: mergeConversations.data?.length ?? 0,
  });
}
