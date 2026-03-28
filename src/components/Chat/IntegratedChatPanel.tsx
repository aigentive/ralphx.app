/**
 * IntegratedChatPanel - Context-aware chat panel for split-screen layout
 *
 * This is a refactored version of ChatPanel that:
 * - Is part of the layout, not fixed positioned
 * - Supports context switching based on selected task
 * - No slide animations (instant show/hide)
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

import { useState, useRef, useEffect, useLayoutEffect, useMemo, useCallback } from "react";
import { useShallow } from "zustand/react/shallow";
import { type VirtuosoHandle } from "react-virtuoso";
import { useChat, useConversation, chatKeys } from "@/hooks/useChat";
import {
  useChatStore,
  selectQueuedMessages,
  selectAgentStatus,
  selectIsAgentRunning,
  selectIsSending,
  selectToolCallStartTimes,
  selectLastAgentEventTimestamp,
} from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore } from "@/stores/taskStore";
import { useTasks, taskKeys } from "@/hooks/useTasks";
import { useChatPanelContext } from "@/hooks/useChatPanelContext";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import { api } from "@/lib/tauri";
import { getContextConfig, buildStoreKey } from "@/lib/chat-context-registry";
import type { Task } from "@/types/task";
import type { ContextType } from "@/types/chat-conversation";
import { ALL_REVIEW_STATUSES, EXECUTION_STATUSES, MERGE_STATUSES } from "@/types/status";
import { AGENT_WORKER, AGENT_REVIEWER } from "@/constants/agents";
import { StatusActivityBadge, type AgentType } from "./StatusActivityBadge";
import { ConversationSelector } from "./ConversationSelector";
import { QueuedMessageList } from "./QueuedMessageList";
import { ChatInput } from "./ChatInput";
import { ChatMessageList } from "./ChatMessageList";
import {
  EmptyState,
  LoadingState,
  ContextIndicator,
  PreviousRunBanner,
  animationStyles,
  HistoryEmptyState,
} from "./IntegratedChatPanel.components";
import { useChatActions } from "@/hooks/useChatActions";
import { useChatEvents } from "@/hooks/useChatEvents";
import { useChatRecovery } from "@/hooks/useChatRecovery";
// useAgentEvents is already called inside useChat — no direct import needed
import { useAskUserQuestion } from "@/hooks/useAskUserQuestion";
import { useQuestionInput } from "@/hooks/useQuestionInput";
import { QuestionInputBanner } from "./QuestionInputBanner";
import { RecoveryPromptDialog } from "@/components/recovery/RecoveryPromptDialog";
import { useEventBus } from "@/providers/EventProvider";
import { logger } from "@/lib/logger";
import { ChildSessionNotification } from "./ChildSessionNotification";
import { useIdeationStore } from "@/stores/ideationStore";
import { useChatAttachments } from "@/hooks/useChatAttachments";
import { ideationApi } from "@/api/ideation";
import { selectIsTeamActive } from "@/stores/chatStore";
import { useTeamStore, selectTeammates, selectActiveTeam, selectTeammateByName, type TeammateStatus } from "@/stores/teamStore";
import { useTeamEvents } from "@/hooks/useTeamEvents";
import { useTeamActions } from "@/hooks/useTeamActions";
import { TeamContextBar } from "./TeamContextBar";
import { TeamPlanApproval } from "./TeamPlanApproval";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { isDiffToolCall } from "./DiffToolCallView.utils";
import { TeamFilterTabs, type TeamFilterValue } from "./TeamFilterTabs";
import { useTeamHistory } from "@/hooks/useTeamHistory";
import { getTeamStatus } from "@/api/team";
import { TimeoutWarning } from "./TimeoutWarning";
import { ChildSessionNavigationContext } from "./tool-widgets/ChildSessionNavigationContext";
import { toast } from "sonner";

// Stable empty array to avoid new reference on every render when tasks query returns undefined
const EMPTY_TASKS: never[] = [];

// ============================================================================
// Main Component
// ============================================================================

interface IntegratedChatPanelProps {
  /** Project ID for context */
  projectId: string;
  /** Optional ideation session ID - when set, uses ideation context */
  ideationSessionId?: string;
  /** Custom empty state component */
  emptyState?: React.ReactNode;
  /** Always show helper text under input */
  showHelperTextAlways?: boolean;
  /** Custom class for input container */
  inputContainerClassName?: string;
  /** Custom header content to replace default context indicator */
  headerContent?: React.ReactNode;
  /** Called when Escape is pressed with input blurred - used to close the panel */
  onClose?: () => void;
  /** Whether to autofocus chat input on mount */
  autoFocusInput?: boolean;
  /** Whether this panel is currently visible (used in dual-panel mode to suppress toasts on hidden panel) */
  isVisible?: boolean;
}

export function IntegratedChatPanel({
  projectId,
  ideationSessionId,
  emptyState,
  showHelperTextAlways = false,
  inputContainerClassName,
  headerContent,
  onClose,
  autoFocusInput = true,
  isVisible = true,
}: IntegratedChatPanelProps) {
  const bus = useEventBus();
  const queryClient = useQueryClient();
  const pollStartRef = useRef<number | null>(null);
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  // History state from store - shared with TaskDetailOverlay for time-travel feature
  const taskHistoryState = useUiStore((s) => s.taskHistoryState);
  const isHistoryMode = !!taskHistoryState;
  const hasHistoryConversation = !!taskHistoryState?.conversationId;

  // Get task data from React Query (useTasks) which has full task data
  const { data: tasks = EMPTY_TASKS } = useTasks(projectId);

  // Read from Zustand store (event-updated, sync) — same pattern as TaskDetailOverlay
  const taskFromStore = useTaskStore((state) =>
    selectedTaskId ? state.tasks[selectedTaskId] : undefined
  );

  // Find from list query
  const taskFromList = selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) : undefined;

  // Fallback: fetch the specific task by ID when not found in store or list
  const { data: taskFromDetail } = useQuery<Task, Error>({
    queryKey: taskKeys.detail(selectedTaskId ?? ""),
    queryFn: () => api.tasks.get(selectedTaskId!),
    enabled: Boolean(selectedTaskId) && !taskFromStore && !taskFromList,
  });

  const selectedTask: Task | undefined = taskFromStore ?? taskFromList ?? taskFromDetail;

  // Determine effective status - use historical status in history mode, otherwise current status
  const effectiveStatus = taskHistoryState?.status ?? selectedTask?.internalStatus;

  // Agent-status-aware overrides: keep mode active while agent is still running,
  // even if task status has already transitioned
  const executionKey = selectedTaskId ? buildStoreKey("task_execution", selectedTaskId) : "";
  const executionAgentRunning = useChatStore(selectIsAgentRunning(executionKey));
  const reviewKey = selectedTaskId ? buildStoreKey("review", selectedTaskId) : "";
  const reviewAgentRunning = useChatStore(selectIsAgentRunning(reviewKey));
  const mergeKey = selectedTaskId ? buildStoreKey("merge", selectedTaskId) : "";
  const mergeAgentRunning = useChatStore(selectIsAgentRunning(mergeKey));

  // Execution states: worker agent is running (only when NOT in ideation mode)
  // Agent-status override is gated on !taskHistoryState: in history mode, no agent
  // is running so the override is always false, but the explicit guard prevents
  // stale agentStatus entries from activating mode flags for historical contexts.
  const isExecutionMode = !ideationSessionId && !!selectedTaskId && (
    (effectiveStatus ? (EXECUTION_STATUSES as readonly string[]).includes(effectiveStatus) : false)
    || (!taskHistoryState && executionAgentRunning)
  );

  // Review states: reviewer agent conversation (only when NOT in ideation mode)
  // Include 'approved' so historical view loads the reviewer's conversation
  const isReviewMode = !ideationSessionId && !!selectedTaskId && (
    (effectiveStatus ? ((ALL_REVIEW_STATUSES as readonly string[]).includes(effectiveStatus) || effectiveStatus === "approved") : false)
    || (!taskHistoryState && reviewAgentRunning)
  );

  // Merge states: merger agent conversation (only when NOT in ideation mode)
  const isMergeMode = !ideationSessionId && !!selectedTaskId && (
    (effectiveStatus ? (MERGE_STATUSES as readonly string[]).includes(effectiveStatus) : false)
    || (!taskHistoryState && mergeAgentRunning)
  );

  // Use extracted context management hook
  const {
    chatContext,
    storeContextKey,
    currentContextType,
    currentContextId,
    activeConversationId,
    streamingToolCalls,
    setStreamingToolCalls,
    streamingContentBlocks,
    setStreamingContentBlocks,
    streamingTasks,
    setStreamingTasks,
    isFinalizing,
    setIsFinalizing,
    autoSelectConversation,
    // overrideAgentRunId is available but we use taskHistoryState.timestamp for scroll positioning
  } = useChatPanelContext({
    projectId,
    ideationSessionId,
    selectedTaskId: selectedTaskId ?? undefined,
    isExecutionMode,
    isReviewMode,
    isMergeMode,
    isHistoryMode,
    // Pass history mode overrides for conversation selection
    overrideConversationId: taskHistoryState?.conversationId,
    overrideAgentRunId: taskHistoryState?.agentRunId,
    isVisible,
  });

  const setActiveConversation = useChatStore((s) => s.setActiveConversation);

  // Refs for stable agent:run_started handler — prevent stale closure writes during context transitions.
  // useLayoutEffect keeps refs synchronised before any Tauri IPC events can arrive.
  const storeContextKeyRef = useRef(storeContextKey);
  const currentContextTypeRef = useRef(currentContextType);
  const currentContextIdRef = useRef(currentContextId);
  const isHistoryModeRef = useRef(isHistoryMode);
  useLayoutEffect(() => {
    storeContextKeyRef.current = storeContextKey;
    currentContextTypeRef.current = currentContextType;
    currentContextIdRef.current = currentContextId;
    isHistoryModeRef.current = isHistoryMode;
  }, [storeContextKey, currentContextType, currentContextId, isHistoryMode]);

  // Team mode state
  const isTeamActiveSelector = useMemo(() => selectIsTeamActive(storeContextKey), [storeContextKey]);
  const isTeamActive = useChatStore(isTeamActiveSelector);
  const teammatesSelector = useMemo(() => selectTeammates(storeContextKey), [storeContextKey]);
  const teammates = useTeamStore(teammatesSelector);
  const pendingPlan = useTeamStore((s) => s.pendingPlans[storeContextKey]);
  const [teamFilter, setTeamFilter] = useState<TeamFilterValue>("lead");
  const sendTarget = teamFilter === "lead" || !teamFilter ? "lead" : teamFilter;

  // Teammate tab: resolve the teammate's conversation_id for standard chat pipeline
  const isTeammateTab = !!teamFilter && teamFilter !== "lead";
  const activeTeammateSelector = useMemo(
    () => isTeammateTab ? selectTeammateByName(storeContextKey, teamFilter) : () => null,
    [storeContextKey, teamFilter, isTeammateTab],
  );
  const activeTeammate = useTeamStore(activeTeammateSelector);
  const teammateConversationId = isTeammateTab ? (activeTeammate?.conversationId ?? null) : null;

  // Track whether the team in this context is historical (hydrated from backend)
  const activeTeamSelector = useMemo(() => selectActiveTeam(storeContextKey), [storeContextKey]);
  const activeTeam = useTeamStore(activeTeamSelector);
  const isTeamHistorical = activeTeam?.isHistorical === true;

  // Team events subscription — always pass contextKey so team:created is never missed
  useTeamEvents(storeContextKey);

  // Rehydrate team state on mount — handles both live and historical teams.
  // If the user navigated away and missed the team:created event, isTeamActive
  // and teamName are unset. We query the most recent session from history:
  //   - disbandedAt === null → team still active → fetch live status and hydrate as live
  //     (unlocks Effect 2 in useTeamEvents and useTeamStatus polling)
  //   - disbandedAt !== null → team done → hydrate as historical
  const { data: teamHistory } = useTeamHistory(currentContextType, currentContextId);
  const hydrateFromHistory = useTeamStore((s) => s.hydrateFromHistory);
  const createTeam = useTeamStore((s) => s.createTeam);
  const addTeammate = useTeamStore((s) => s.addTeammate);
  const setTeamActive = useChatStore((s) => s.setTeamActive);

  useEffect(() => {
    if (!teamHistory?.session || isTeamActive) return;

    const session = teamHistory.session;

    if (session.disbandedAt) {
      // Team is disbanded — hydrate as historical view
      hydrateFromHistory(storeContextKey, teamHistory);
      setTeamActive(storeContextKey, true);
      return;
    }

    // Team still active in backend — rehydrate as live
    let cancelled = false;
    void getTeamStatus(session.teamName)
      .then((liveStatus) => {
        if (cancelled) return;
        if (!liveStatus) {
          // Team no longer in live tracker (e.g. app restarted) — fall back to historical
          hydrateFromHistory(storeContextKey, teamHistory);
          setTeamActive(storeContextKey, true);
          return;
        }
        createTeam(storeContextKey, liveStatus.name, liveStatus.lead_name ?? liveStatus.name);
        for (const mate of liveStatus.teammates) {
          addTeammate(storeContextKey, {
            name: mate.name,
            color: mate.color,
            model: mate.model,
            roleDescription: mate.role,
            status: (mate.status as TeammateStatus) || "idle",
            currentActivity: null,
            tokensUsed: mate.cost.input_tokens + mate.cost.output_tokens,
            estimatedCostUsd: mate.cost.estimated_usd,
            conversationId: mate.conversation_id ?? null,
          });
        }
        setTeamActive(storeContextKey, true);
      })
      .catch(() => {
        if (cancelled) return;
        // On error fetching live status, fall back to historical
        hydrateFromHistory(storeContextKey, teamHistory);
        setTeamActive(storeContextKey, true);
      });
    return () => { cancelled = true; };
  }, [teamHistory, isTeamActive, storeContextKey, hydrateFromHistory, setTeamActive, createTeam, addTeammate]);

  // Team actions
  const teamActions = useTeamActions(
    currentContextType as ContextType,
    currentContextId,
  );

  // Agent lifecycle events (useAgentEvents) are handled inside useChat — no duplicate subscription needed.

  // If a new run starts in this context, switch to its conversation (live mode only).
  // Reads context values from refs to avoid stale closure writes during teardown/resubscribe window.
  useEffect(() => {
    return bus.subscribe<{
      context_type: string;
      context_id: string;
      conversation_id: string;
      teammate_name?: string | null;
    }>("agent:run_started", (payload) => {
      if (isHistoryModeRef.current) return;
      // Ignore teammate run_started — their conversations are handled via team filter tabs
      if (payload.teammate_name) return;

      // Existing exact match
      if (
        payload.context_type === currentContextTypeRef.current &&
        payload.context_id === currentContextIdRef.current &&
        payload.conversation_id
      ) {
        setActiveConversation(storeContextKeyRef.current, payload.conversation_id);
        return;
      }
      // Handle retry scenario: task context watching a new execution starting
      // When task is in failed/ready state, currentContextType is "task" but
      // the new execution emits "task_execution". Accept if task ID matches.
      // Dual-write: set on the panel's current slot (storeContextKey) so the
      // current panel immediately shows the conversation with no blank flash,
      // AND pre-populate the new execution slot so when the panel transitions
      // to task_execution context the conversation is already set.
      if (
        payload.context_type === "task_execution" &&
        currentContextTypeRef.current === "task" &&
        payload.context_id === currentContextIdRef.current &&
        payload.conversation_id
      ) {
        setActiveConversation(storeContextKeyRef.current, payload.conversation_id);
        const executionKey = buildStoreKey(payload.context_type as ContextType, payload.context_id);
        if (executionKey !== storeContextKeyRef.current) {
          setActiveConversation(executionKey, payload.conversation_id);
        }
      }
    });
  }, [bus, setActiveConversation]);

  // Subscribe to agent:conversation_created — invalidate conversation list query so new conversations appear immediately.
  useEffect(() => {
    return bus.subscribe<{
      context_id: string;
      context_type: string;
      conversation_id: string;
    }>("agent:conversation_created", (payload) => {
      if (payload.context_id !== currentContextId) return;
      void queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList(payload.context_type as ContextType, payload.context_id),
      });
    });
  }, [bus, queryClient, currentContextId]);

  // Use context-aware selectors - unified queue works for all modes
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(storeContextKey), [storeContextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const agentStatusSelector = useMemo(() => selectAgentStatus(storeContextKey), [storeContextKey]);
  const agentStatus = useChatStore(agentStatusSelector);
  const isAgentRunning = agentStatus !== "idle"; // backward-compat boolean (agent process alive)
  const lastAgentEventTsSelector = useMemo(() => selectLastAgentEventTimestamp(storeContextKey), [storeContextKey]);
  const lastAgentEventTs = useChatStore(lastAgentEventTsSelector);
  const toolCallStartTimesSelector = useMemo(
    () => selectToolCallStartTimes(storeContextKey),
    [storeContextKey],
  );
  const toolCallStartTimes = useChatStore(toolCallStartTimesSelector);
  const isSendingSelector = useMemo(() => selectIsSending(storeContextKey), [storeContextKey]);

  // Timeout warning state — track dismissed bash tool call ID
  const [dismissedTimeoutCallId, setDismissedTimeoutCallId] = useState<string | null>(null);
  const activeBashCall = streamingToolCalls.find((tc) => tc.name.toLowerCase() === "bash");
  const bashStartTime = activeBashCall ? toolCallStartTimes[activeBashCall.id] : undefined;
  // Context-aware threshold: 3600s for team mode, 600s otherwise
  const effectiveTimeoutMs = isTeamActive ? 3_600_000 : 600_000;
  const showTimeoutWarning = activeBashCall !== undefined && bashStartTime !== undefined && activeBashCall.id !== dismissedTimeoutCallId;

  // Auto-reset dismissed ID when the dismissed call is no longer active
  useEffect(() => {
    if (dismissedTimeoutCallId && !streamingToolCalls.find((tc) => tc.id === dismissedTimeoutCallId)) {
      setDismissedTimeoutCallId(null);
    }
  }, [streamingToolCalls, dismissedTimeoutCallId]);
  const isSending = useChatStore(isSendingSelector);
  const setAgentRunning = useChatStore((s) => s.setAgentRunning);

  // For execution/review mode, fetch conversations directly with specific context type
  const regularChatData = useChat(chatContext, { isVisible, storeKey: storeContextKey, disableAutoSelect: true });

  // Single dynamic query for all agent contexts (execution/review/merge)
  // When currentContextType changes, the query key changes and a fresh fetch fires
  const isAgentContext = isExecutionMode || isReviewMode || isMergeMode;

  const agentConversationsQuery = useQuery({
    queryKey: chatKeys.conversationList(currentContextType, selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations(currentContextType as ContextType, selectedTaskId ?? ""),
    enabled: isAgentContext && !!selectedTaskId,
    staleTime: 0,
  });

  // Use agent query for agent contexts, regular chat data otherwise
  const conversations = isAgentContext
    ? agentConversationsQuery
    : regularChatData.conversations;

  // Poll every 3s (up to 60s) when visible, non-agent context, and no conversations yet.
  // Drives the auto-select chain: invalidateQueries → React Query refetch → conversationsData updates → auto-select re-fires.
  const POLL_INTERVAL_MS = 3000;
  const POLL_MAX_MS = 60_000;
  useEffect(() => {
    if (!isVisible || isAgentContext) {
      pollStartRef.current = null;
      return;
    }
    if ((conversations.data?.length ?? 0) > 0) {
      pollStartRef.current = null;
      return;
    }
    pollStartRef.current = Date.now();
    const id = setInterval(() => {
      if (pollStartRef.current !== null && Date.now() - pollStartRef.current >= POLL_MAX_MS) {
        clearInterval(id);
        pollStartRef.current = null;
        return;
      }
      void queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList(currentContextType, currentContextId),
      });
    }, POLL_INTERVAL_MS);
    return () => { clearInterval(id); };
  }, [isVisible, isAgentContext, conversations.data, queryClient, currentContextType, currentContextId]);

  // Auto-select the most recent conversation in execution/review/merge modes
  // Extract stable primitives from TanStack Query result to avoid re-render on every query object change
  const conversationsData = conversations.data;
  const conversationsLoading = conversations.isLoading;
  useEffect(() => {
    autoSelectConversation({ data: conversationsData, isLoading: conversationsLoading });
  }, [autoSelectConversation, conversationsData, conversationsLoading, isVisible]);

  // Check if active conversation belongs to current context (needed by recovery effects below)
  const activeConversationContext = regularChatData.messages.data?.conversation;
  const isConversationInCurrentContext = useMemo(
    () =>
      (activeConversationContext?.contextType === currentContextType ||
       (currentContextType === "task" && activeConversationContext?.contextType === "task_execution")) &&
      activeConversationContext?.contextId === currentContextId,
    [activeConversationContext?.contextType, activeConversationContext?.contextId,
     currentContextType, currentContextId]
  );

  // Fetch agent run status for the active conversation
  const agentRunQuery = useQuery({
    queryKey: chatKeys.agentRun(activeConversationId ?? ""),
    queryFn: () => activeConversationId ? chatApi.getAgentRunStatus(activeConversationId) : null,
    enabled: !!activeConversationId,
    staleTime: 5000,
  });

  // Recovery and polling effects (extracted to hook)
  useChatRecovery({
    activeConversationId,
    storeContextKey,
    currentContextType,
    currentContextId,
    isHistoryMode,
    isAgentContext,
    isAgentRunning,
    isGenerating: agentStatus === "generating",
    isConversationInCurrentContext,
    agentRunStatus: agentRunQuery.data?.status ?? undefined,
    setAgentRunning,
    selectedTaskId: selectedTaskId ?? undefined,
    ideationSessionId,
    projectId,
    effectiveStatus,
  });

  // Track dismissed error banners by run ID
  const [dismissedErrorId, setDismissedErrorId] = useState<string | null>(null);
  const failedRun = agentRunQuery.data?.status === "failed" ? agentRunQuery.data : null;
  const showFailedBanner = failedRun && failedRun.errorMessage && failedRun.id !== dismissedErrorId;

  // Memoize failedRun prop to avoid creating a new object reference each render,
  // which would bust ChatMessageList's virtuosoComponents useMemo via the failedRun dep.
  const failedRunProp = useMemo(
    () => showFailedBanner && failedRun ? { id: failedRun.id, errorMessage: failedRun.errorMessage! } : null,
    [showFailedBanner, failedRun]
  );

  const {
    messages: activeConversation,
    sendMessage,
    switchConversation: handleSelectConversation,
    createConversation: handleNewConversation,
  } = regularChatData;

  const virtuosoRef = useRef<VirtuosoHandle>(null);

  // File attachments - use activeConversationId for attachment association
  // Only enable attachments when there's an active conversation (not in history mode)
  const {
    attachments,
    uploadFiles,
    removeAttachment,
    clearAttachments,
  } = useChatAttachments(activeConversationId ?? "");

  // Load teammate conversation messages when on a teammate tab
  const teammateConversation = useConversation(teammateConversationId);

  // Effective conversation ID: teammate's when on teammate tab, lead's otherwise
  const effectiveConversationId = isTeammateTab ? teammateConversationId : activeConversationId;

  // Memoize messagesData to avoid dependency chain issues in useEffect hooks
  // No time-based filtering needed - we switch context types based on historical state
  const messagesData = useMemo(
    () => {
      if (isTeammateTab) {
        return teammateConversation.data?.messages ?? [];
      }
      return activeConversationId && isConversationInCurrentContext
        ? (activeConversation.data?.messages ?? [])
        : [];
    },
    [isTeammateTab, teammateConversation.data?.messages, activeConversationId, isConversationInCurrentContext, activeConversation.data?.messages]
  );

  // Debug logging for history mode
  logger.debug('[IntegratedChatPanel] Context mode:', {
    isHistoryMode,
    effectiveStatus,
    isExecutionMode,
    isReviewMode,
    taskHistoryState,
  });

  const {
    handleSend: handleSendBase,
    handleEditLastQueued,
    handleDeleteQueuedMessage,
    handleEditQueuedMessage,
    handleStopAgent,
  } = useChatActions({
    contextType: currentContextType,
    contextId: currentContextId,
    storeContextKey,
    selectedTaskId: selectedTaskId ?? undefined,
    ideationSessionId,
    sendMessage,
    messageCount: messagesData.length,
  });

  // Wrap handleSend to include attachment IDs, team target, and clear attachments after send
  const handleSend = useCallback(async (message: string) => {
    // Collect attachment IDs before sending
    const attachmentIds = attachments.map(a => a.id);

    // Call the base handler with attachment IDs and team target
    await handleSendBase(
      message,
      attachmentIds.length > 0 ? attachmentIds : undefined,
      isTeamActive ? sendTarget : undefined
    );

    // Clear attachments after successful send
    // Note: If send fails, attachments are preserved for retry
    if (attachmentIds.length > 0) {
      clearAttachments();
    }
  }, [attachments, handleSendBase, clearAttachments, isTeamActive, sendTarget]);

  // Wrapper for handleEditLastQueued that provides the queued messages
  const handleEditLastQueuedWrapper = () => {
    handleEditLastQueued(queuedMessages);
  };

  // Handle stopping agent - clear streaming state
  const handleStopAgentWrapper = useCallback(async () => {
    // Stop all teammates when team is active, otherwise just stop the lead agent
    if (isTeamActive) {
      teamActions.stopTeam.mutate();
    }
    await handleStopAgent();
    setStreamingToolCalls(prev => prev.length === 0 ? prev : []);
    setStreamingContentBlocks(prev => prev.length === 0 ? prev : []);
    setStreamingTasks(prev => prev.size === 0 ? prev : new Map());
  }, [isTeamActive, teamActions, handleStopAgent, setStreamingToolCalls, setStreamingContentBlocks, setStreamingTasks]);

  useChatEvents({
    activeConversationId: effectiveConversationId,
    contextId: currentContextId,
    contextType: currentContextType,
    setStreamingToolCalls,
    setStreamingContentBlocks,
    setStreamingTasks,
    setIsFinalizing,
    storeKey: storeContextKey,
  });

  // Ask user question state — scoped to current context (ideation session, task, or project)
  const {
    activeQuestion,
    answeredQuestion,
    submitAnswer,
    dismissQuestion,
    clearAnswered,
    isLoading: isSubmittingAnswer,
  } = useAskUserQuestion(currentContextId);

  // Question UI state — chip selection, input sync, question-aware send
  const {
    selectedOptions,
    questionInputValue,
    setQuestionInputValue,
    handleChipClick,
    handleMatchedOptions,
    handleQuestionSend,
  } = useQuestionInput({
    activeQuestion: activeQuestion ?? null,
    submitAnswer,
    handleSend,
  });

  // Ideation store for session navigation
  const selectSession = useIdeationStore((s) => s.selectSession);
  const allSessions = useIdeationStore(useShallow((s) => Object.values(s.sessions)));

  // Handler for navigating to child session
  // Fetches from backend if session not in local store (e.g., newly created child)
  const handleNavigateToChildSession = useCallback(async (childSessionId: string) => {
    // First check local store
    const session = allSessions.find((s) => s.id === childSessionId);
    if (session) {
      selectSession(session);
      return;
    }

    // Session not in store - fetch from backend
    try {
      const fetchedSession = await ideationApi.sessions.get(childSessionId);
      if (fetchedSession) {
        selectSession(fetchedSession);
      } else {
        logger.warn("[IntegratedChatPanel] Child session not found:", childSessionId);
        toast.error("Session not found");
      }
    } catch (error) {
      logger.warn("[IntegratedChatPanel] Failed to fetch child session:", { childSessionId, error });
      toast.error("Session not found");
    }
  }, [allSessions, selectSession]);

  // Handle Escape key to close panel
  useEffect(() => {
    if (!onClose) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  // Sort messages by createdAt always. Secondary sort by id provides stable
  // tiebreaking when timestamps are equal (e.g. optimistic + DB messages share ms).
  const sortedMessages = useMemo(() => {
    return [...messagesData]
      // Hide session recovery rehydration prompts from UI.
      // Primary: metadata flag set by backend. Fallback: content prefix for pre-existing rows.
      .filter((msg) => {
        if (msg.metadata) {
          try {
            const meta = JSON.parse(msg.metadata);
            if (meta.recovery_context) return false;
          } catch { /* not JSON, keep message */ }
        }
        if (msg.role === "user" && msg.content.startsWith("<instructions>")) return false;
        return true;
      })
      .sort((a, b) => {
        const timeDiff = new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
        if (timeDiff !== 0) return timeDiff;
        return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
      });
  }, [messagesData]);

  // Loading state: show skeleton when conversations list is loading OR active conversation is loading
  const isConversationsLoading = conversations.isLoading;
  const isActiveConversationLoading = activeConversationId ? activeConversation.isLoading : false;
  const isLoading = isConversationsLoading || isActiveConversationLoading;

  // Status badge helpers - disabled in history mode (no live agent)
  // isAgentActive: only true when actively generating (not waiting_for_input)
  const isAgentActive = !isHistoryMode && (isSending || agentStatus === "generating");
  const agentType: AgentType = isHistoryMode
    ? "idle"
    : isExecutionMode
      ? AGENT_WORKER
      : isReviewMode
        ? AGENT_REVIEWER
        : (isSending || agentStatus === "generating")
          ? "agent"
          : "idle";

  // Empty state: only show when we KNOW there are no messages (not while loading)
  // Also don't show empty if conversations are loading - we might auto-select one
  const hasNoConversations = !isConversationsLoading && (conversations.data?.length ?? 0) === 0;
  const hasEmptyConversation = !isLoading && activeConversationId && sortedMessages.length === 0;
  const isEmpty = hasNoConversations || hasEmptyConversation;

  // Recency guard: suppress PreviousRunBanner if the agent was active within the last 10s.
  // Aligned with agentRunQuery.staleTime (10s) to avoid banner flash during run_completed transition.
  const [isRecentlyActive, setIsRecentlyActive] = useState(false);
  useEffect(() => {
    if (lastAgentEventTs <= 0) { setIsRecentlyActive(false); return; }
    const elapsed = Date.now() - lastAgentEventTs;
    if (elapsed >= 10_000) { setIsRecentlyActive(false); return; }
    setIsRecentlyActive(true);
    const timer = setTimeout(() => setIsRecentlyActive(false), 10_000 - elapsed);
    return () => clearTimeout(timer);
  }, [lastAgentEventTs]);

  return (
    <>
      <style>{animationStyles}</style>
      <RecoveryPromptDialog surface="chat" taskId={selectedTaskId ?? undefined} />
      {/* Outer container - matches main content bg for unified surface */}
      <div
        data-testid="integrated-chat-panel"
        className="h-full flex flex-col overflow-hidden"
        style={{
          backgroundColor: "transparent", /* Let parent bg show through */
          padding: "8px", /* Equal padding all sides - floating glass element */
        }}
      >
        {/* Inner rounded container - flat with blur */}
        <div
          className="flex-1 flex flex-col overflow-hidden"
          style={{
            borderRadius: "10px",
            /* FLAT semi-transparent (no gradient) */
            background: "hsla(220 10% 10% / 0.92)",
            backdropFilter: "blur(20px) saturate(180%)",
            WebkitBackdropFilter: "blur(20px) saturate(180%)",
            /* Luminous perimeter edge */
            border: "1px solid hsla(220 20% 100% / 0.08)",
            boxShadow: `
              0 4px 16px hsla(220 20% 0% / 0.4),
              0 12px 32px hsla(220 20% 0% / 0.3)
            `,
          }}
        >
          {/* Header - subtle separation within glass container */}
          <div
            data-testid="integrated-chat-header"
            className="flex items-center justify-between h-11 px-3 shrink-0"
            style={{
              backgroundColor: "hsla(220 15% 5% / 0.5)",
              borderBottom: "1px solid hsla(220 20% 100% / 0.04)",
            }}
          >
            {headerContent ?? <ContextIndicator context={chatContext} isExecutionMode={isExecutionMode} isReviewMode={isReviewMode} />}

            {/* Unified status + activity badge */}
            <StatusActivityBadge
              isAgentActive={isAgentActive}
              agentType={agentType}
              contextType={currentContextType as ContextType}
              contextId={ideationSessionId || selectedTaskId || null}
              agentStatus={isHistoryMode ? "idle" : agentStatus}
              storeKey={storeContextKey}
            />

            {/* Conversation Selector */}
            <ConversationSelector
              contextType={
                ideationSessionId
                  ? "ideation"
                  : isMergeMode
                    ? "merge"
                    : isExecutionMode
                      ? "task_execution"
                      : isReviewMode
                        ? "review"
                        : selectedTaskId
                          ? "task"
                          : "project"
              }
              contextId={ideationSessionId || selectedTaskId || projectId}
              conversations={conversations.data ?? []}
              activeConversationId={activeConversationId}
              onSelectConversation={handleSelectConversation}
              onNewConversation={handleNewConversation}
              isLoading={conversations.isLoading}
            />
          </div>

          {/* Team Context Bar (team mode only) */}
          {isTeamActive && teammates.length > 0 && (
            <TeamContextBar
              contextKey={storeContextKey}
              activeFilter={teamFilter}
              isHistorical={isTeamHistorical}
              onStopTeammate={(name) => {
                teamActions.stopTeammate.mutate(name);
              }}
            />
          )}

          {/* Timeout Warning Banner — shown when bash tool call approaches timeout */}
          {showTimeoutWarning && (
            <TimeoutWarning
              toolCallStartTime={bashStartTime!}
              effectiveTimeoutMs={effectiveTimeoutMs}
              onDismiss={() => setDismissedTimeoutCallId(activeBashCall!.id)}
            />
          )}

          {/* Messages Area — wrapped with navigation context for child session widgets */}
          <ChildSessionNavigationContext.Provider value={handleNavigateToChildSession}>
          {isLoading ? (
            <div className="flex-1 flex items-center justify-center" data-testid="integrated-chat-messages">
              <LoadingState />
            </div>
          ) : isEmpty ? (
            <div className="flex-1 flex items-center justify-center" data-testid="integrated-chat-messages">
              {emptyState ??
                (isHistoryMode && !hasHistoryConversation ? (
                  <HistoryEmptyState />
                ) : (
                  <EmptyState />
                ))}
            </div>
          ) : (
            <ChatMessageList
              ref={virtuosoRef}
              messages={sortedMessages}
              conversationId={effectiveConversationId}
              failedRun={failedRunProp}
              onDismissFailedRun={setDismissedErrorId}
              isSending={isSending}
              isAgentRunning={agentStatus === "generating"}
              streamingToolCalls={streamingToolCalls}
              streamingTasks={streamingTasks}
              streamingContentBlocks={streamingContentBlocks}
              scrollToTimestamp={isHistoryMode ? taskHistoryState?.timestamp : null}
              isFinalizing={isFinalizing}
              teamFilter={activeTeam ? teamFilter : undefined}
              contextKey={activeTeam ? storeContextKey : undefined}
            />
          )}

          {/* StreamingToolIndicator — outside scroll container so it's always visible.
              Filters out Task calls (shown as TaskSubagentCard), diff calls (shown inline),
              and any tool calls already rendered inline via streamingContentBlocks to avoid duplication. */}
          {(isSending || agentStatus === "generating") && (() => {
            // IDs of tool calls already rendered inline from streamingContentBlocks
            const inlineToolIds = new Set(
              streamingContentBlocks
                ?.filter((b) => b.type === "tool_use")
                .map((b) => b.type === "tool_use" ? b.toolCall.id : "") ?? []
            );
            const otherToolCalls = streamingToolCalls.filter(
              (tc) => !inlineToolIds.has(tc.id) &&
                      tc.name.toLowerCase() !== "task" &&
                      (!isDiffToolCall(tc.name) || tc.arguments == null)
            );
            return otherToolCalls.length > 0 ? (
              <div className="shrink-0 px-3 pb-2">
                <StreamingToolIndicator toolCalls={otherToolCalls} isActive={true} toolCallStartTimes={toolCallStartTimes} />
              </div>
            ) : null;
          })()}

          {/* Team Plan Approval (shown when lead requests plan approval) */}
          {pendingPlan && (
            <TeamPlanApproval
              plan={pendingPlan}
              contextKey={storeContextKey}
            />
          )}

          {/* Child Session Notification - shows when follow-up is created (ideation mode only) */}
          {ideationSessionId && !isHistoryMode && (
            <ChildSessionNotification
              sessionId={ideationSessionId}
            />
          )}
          </ChildSessionNavigationContext.Provider>

          {/* Previous Run Banner - shown when viewing stale agent conversation */}
          {isAgentContext && !isHistoryMode && agentStatus === "idle" && agentRunQuery.data?.status !== "running" && !isSending && sortedMessages.length > 0 && !isRecentlyActive && (
            <PreviousRunBanner
              agentRunStatus={agentRunQuery.data?.status ?? null}
              contextType={isMergeMode ? "merge" : isReviewMode ? "review" : "execution"}
            />
          )}

          {/* Team Filter Tabs (team mode — above input area) */}
          {isTeamActive && teammates.length > 0 && (
            <TeamFilterTabs
              teammates={teammates}
              activeFilter={teamFilter}
              onFilterChange={setTeamFilter}
            />
          )}

          {/* Input Area - subtle separation within glass container */}
          <div
            className={inputContainerClassName ?? "shrink-0"}
            style={inputContainerClassName ? undefined : {
              backgroundColor: "hsla(220 15% 5% / 0.5)",
              borderTop: "1px solid hsla(220 20% 100% / 0.04)",
            }}
          >
            {/* Queued Messages - unified queue with context-aware keys */}
            {queuedMessages.length > 0 && (
              <div className="p-3 pb-0">
                <QueuedMessageList
                  messages={queuedMessages}
                  onEdit={handleEditQueuedMessage}
                  onDelete={handleDeleteQueuedMessage}
                />
              </div>
            )}

            {/* Question Input Banner - renders above ChatInput when question is active */}
            {(activeQuestion || answeredQuestion) && (
              <QuestionInputBanner
                key={activeQuestion?.requestId ?? 'answered'}
                question={activeQuestion ?? null}
                selectedIndices={selectedOptions}
                onChipClick={handleChipClick}
                onDismiss={dismissQuestion}
                answeredValue={answeredQuestion}
                onDismissAnswered={clearAnswered}
              />
            )}

            {/* Chat Input */}
            <div className="p-3">
              <ChatInput
                onSend={activeQuestion ? handleQuestionSend : handleSend}
                onStop={handleStopAgentWrapper}
                agentStatus={agentStatus}
                isSending={isSending || isSubmittingAnswer}
                hasQueuedMessages={queuedMessages.length > 0}
                onEditLastQueued={handleEditLastQueuedWrapper}
                isReadOnly={isHistoryMode}
                placeholder={getContextConfig(currentContextType).placeholder}
                showHelperText={showHelperTextAlways}
                {...(activeQuestion ? {
                  value: questionInputValue,
                  onChange: setQuestionInputValue,
                  questionMode: {
                    optionCount: activeQuestion.options.length,
                    multiSelect: activeQuestion.multiSelect,
                    onMatchedOptions: handleMatchedOptions,
                  },
                } : {})}
                autoFocus={autoFocusInput}
                enableAttachments={!!activeConversationId && !isHistoryMode}
                attachments={attachments}
                onFilesSelected={uploadFiles}
                onRemoveAttachment={removeAttachment}
              />
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
