// ChatPanel — resizable side panel for context-aware chat (design: specs/design/pages/chat-panel.md)

import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectActiveConversationId, selectIsTeamActive } from "@/stores/chatStore";
import { useTeamStore, selectTeammates, selectActiveTeam } from "@/stores/teamStore";
import { useUiStore } from "@/stores/uiStore";
import type { ChatContext } from "@/types/chat";


import { useTaskStore } from "@/stores/taskStore";
import { useQuery } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import { Button } from "@/components/ui/button";
import {
  MessageSquare,
  CheckSquare,
  FolderKanban,
  X,
  PanelRightClose,
  PanelRightOpen,
  Hammer,
} from "lucide-react";
import { AGENT_WORKER } from "@/constants/agents";
import { StatusActivityBadge, type AgentType } from "./StatusActivityBadge";
import { ConversationSelector } from "./ConversationSelector";
import { QueuedMessageList } from "./QueuedMessageList";
import { ChatInput } from "./ChatInput";
import { ChatMessageList } from "./ChatMessageList";
import { QuestionInputBanner } from "./QuestionInputBanner";
import { ResizeablePanel } from "./ResizeablePanel";
import { useResizePanel } from "./useResizePanel";
import { useChatActions } from "@/hooks/useChatActions";
import { useChatEvents } from "@/hooks/useChatEvents";
import { resolveContextType, buildStoreKey } from "@/lib/chat-context-registry";
import type { ToolCall } from "./ToolCallIndicator";
import type { StreamingTask, StreamingContentBlock } from "@/types/streaming-task";
import { useAskUserQuestion } from "@/hooks/useAskUserQuestion";
import { useQuestionInput } from "@/hooks/useQuestionInput";
import { useAgentHookEvents, useHookEventsStore } from "@/hooks/useAgentHookEvents";
import { useChatAttachments } from "@/hooks/useChatAttachments";
import { useTeamEvents } from "@/hooks/useTeamEvents";
import { useTeamActions } from "@/hooks/useTeamActions";
import { TeamActivityPanel } from "./TeamActivityPanel";
import { TeamPlanApproval } from "./TeamPlanApproval";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { isDiffToolCall } from "./DiffToolCallView.utils";
import { TeamFilterTabs, type TeamFilterValue } from "./TeamFilterTabs";
import { TargetSelector, type TargetValue } from "./TargetSelector";
import { useTeamHistory } from "@/hooks/useTeamHistory";

const COLLAPSED_WIDTH = 40;

const animationStyles = `
@keyframes slideInRight {
  from { transform: translateX(100%); opacity: 0.5; }
  to { transform: translateX(0); opacity: 1; }
}

@keyframes slideOutRight {
  from { transform: translateX(0); opacity: 1; }
  to { transform: translateX(100%); opacity: 0.5; }
}

@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.7; transform: scale(1.1); }
}

.chat-panel-enter {
  animation: slideInRight 250ms ease-out forwards;
}

.chat-panel-exit {
  animation: slideOutRight 200ms ease-in forwards;
}

.typing-dot {
  animation: typingBounce 1.4s ease-in-out infinite;
}

.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }

.unread-dot {
  animation: pulse 2s ease-in-out infinite;
}
`;

interface ContextIndicatorProps {
  context: ChatContext;
  isExecutionMode?: boolean;
}

function ContextIndicator({ context, isExecutionMode = false }: ContextIndicatorProps) {
  const getContextInfo = () => {
    if (isExecutionMode) {
      return { icon: Hammer, label: "Worker" };
    }

    switch (context.view) {
      case "ideation":
        return { icon: MessageSquare, label: "Chat" };
      case "kanban":
        return context.selectedTaskId
          ? { icon: CheckSquare, label: "Task" }
          : { icon: FolderKanban, label: "Project" };
      case "task_detail":
        return { icon: CheckSquare, label: "Task" };
      case "activity":
        return { icon: MessageSquare, label: "Activity" };
      case "settings":
        return { icon: MessageSquare, label: "Settings" };
      default:
        return { icon: MessageSquare, label: "Chat" };
    }
  };

  const { icon: Icon, label } = getContextInfo();

  return (
    <div className="flex items-center gap-2 min-w-0 flex-1">
      <Icon className="w-3.5 h-3.5 shrink-0 text-white/50" />
      <span className="text-[13px] font-medium truncate text-white/80">{label}</span>
    </div>
  );
}

interface CollapsedPanelProps {
  onExpand: () => void;
  hasUnread: boolean;
}

function CollapsedPanel({ onExpand, hasUnread }: CollapsedPanelProps) {
  return (
    <div
      data-testid="chat-panel-collapsed"
      className="fixed top-14 right-0 bottom-0 flex flex-col items-center justify-center"
      style={{
        width: `${COLLAPSED_WIDTH}px`,
        backgroundColor: "var(--bg-surface)",
        borderLeft: "1px solid var(--border-subtle)",
      }}
    >
      {hasUnread && (
        <div
          className="unread-dot absolute top-4 w-2 h-2 rounded-full"
          style={{ backgroundColor: "var(--accent-primary)" }}
        />
      )}
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={onExpand}
        aria-label="Expand chat panel"
      >
        <PanelRightOpen className="w-[18px] h-[18px]" />
      </Button>
    </div>
  );
}

interface ChatPanelProps {
  context: ChatContext;
}

// Wrapper component that checks visibility before rendering the full panel
// This prevents expensive hooks from running when the panel is closed
export function ChatPanel({ context }: ChatPanelProps) {
  const chatVisibleByView = useUiStore((s) => s.chatVisibleByView);
  const isVisible = chatVisibleByView[context.view];

  if (!isVisible) {
    return null;
  }

  return <ChatPanelContent context={context} />;
}

function ChatPanelContent({ context }: ChatPanelProps) {
  const width = useChatStore((s) => s.width);
  const setWidth = useChatStore((s) => s.setWidth);
  const toggleChatVisible = useUiStore((s) => s.toggleChatVisible);
  const activeConversationId = useChatStore(selectActiveConversationId);

  // Detect task status for context type resolution
  const selectedTask = useTaskStore((state) =>
    context.selectedTaskId ? state.tasks[context.selectedTaskId] : undefined
  );

  // Derive context type and store key using registry (replaces manual ternary chains)
  const contextType = useMemo(
    () => resolveContextType(
      selectedTask?.internalStatus,
      context.ideationSessionId,
      context.selectedTaskId,
    ),
    [selectedTask?.internalStatus, context.ideationSessionId, context.selectedTaskId]
  );
  const contextId = context.ideationSessionId ?? context.selectedTaskId ?? context.projectId;
  const contextKey = useMemo(
    () => buildStoreKey(contextType, contextId),
    [contextType, contextId]
  );
  const isExecutionMode = contextType === "task_execution";

  // Team mode state
  const isTeamActiveSelector = useMemo(() => selectIsTeamActive(contextKey), [contextKey]);
  const isTeamActive = useChatStore(isTeamActiveSelector);
  const teammatesSelector = useMemo(() => selectTeammates(contextKey), [contextKey]);
  const teammates = useTeamStore(teammatesSelector);
  const pendingPlan = useTeamStore((s) => s.pendingPlans[contextKey]);
  const [teamFilter, setTeamFilter] = useState<TeamFilterValue>("all");
  const [sendTarget, setSendTarget] = useState<TargetValue>("lead");

  // Track whether the team in this context is historical
  const activeTeamSelector = useMemo(() => selectActiveTeam(contextKey), [contextKey]);
  const activeTeam = useTeamStore(activeTeamSelector);
  const isTeamHistorical = activeTeam?.isHistorical === true;

  // Team events subscription — always pass contextKey so team:created is never missed
  useTeamEvents(contextKey);

  // Hydrate historical team activity when no live team is active
  const { data: teamHistory } = useTeamHistory(contextType, contextId);
  const hydrateFromHistory = useTeamStore((s) => s.hydrateFromHistory);
  const setTeamActive = useChatStore((s) => s.setTeamActive);

  useEffect(() => {
    if (!teamHistory?.session || isTeamActive) return;
    hydrateFromHistory(contextKey, teamHistory);
    setTeamActive(contextKey, true);
  }, [teamHistory, isTeamActive, contextKey, hydrateFromHistory, setTeamActive]);

  // Team actions
  const teamActions = useTeamActions(contextType, contextId);

  // Use context-aware selectors - unified queue works for all modes
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(contextKey), [contextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(contextKey), [contextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);

  // For execution mode, fetch execution conversations directly using task_execution context
  // For regular chat, use the standard useChat hook
  const regularChatData = useChat(context);

  // Fetch execution conversations when in execution mode
  const executionConversationsQuery = useQuery({
    queryKey: chatKeys.conversationList("task_execution", context.selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations("task_execution", context.selectedTaskId ?? ""),
    enabled: isExecutionMode && !!context.selectedTaskId,
  });

  // Use execution conversations when in execution mode, otherwise regular conversations
  const conversations = isExecutionMode
    ? executionConversationsQuery
    : regularChatData.conversations;

  // Fetch agent run status for the active conversation to detect failed runs
  const agentRunQuery = useQuery({
    queryKey: chatKeys.agentRun(activeConversationId ?? ""),
    queryFn: () => activeConversationId ? chatApi.getAgentRunStatus(activeConversationId) : null,
    enabled: !!activeConversationId,
    staleTime: 5000,
  });

  // Track dismissed error banners by run ID
  const [dismissedErrorId, setDismissedErrorId] = useState<string | null>(null);
  const failedRun = agentRunQuery.data?.status === "failed" ? agentRunQuery.data : null;
  const showFailedBanner = failedRun && failedRun.errorMessage && failedRun.id !== dismissedErrorId;

  const {
    messages: activeConversation,
    sendMessage,
    switchConversation: handleSelectConversation,
    createConversation: handleNewConversation,
  } = regularChatData;

  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const [hasUnread, setHasUnread] = useState(false);
  const lastMessageCountRef = useRef(0);

  // Resize panel hook
  const { ResizeHandle } = useResizePanel({
    initialWidth: width,
    onWidthChange: setWidth,
  });

  // Extract messages array from active conversation (memoized to avoid dependency chain issues)
  const messagesData = useMemo(
    () => activeConversation.data?.messages ?? [],
    [activeConversation.data?.messages]
  );

  // Track unread messages when collapsed
  useEffect(() => {
    const messageCount = messagesData.length;
    if (isCollapsed && messageCount > lastMessageCountRef.current) {
      setHasUnread(true);
    }
    lastMessageCountRef.current = messageCount;
  }, [messagesData.length, isCollapsed]);

  // Clear unread when expanded
  useEffect(() => {
    if (!isCollapsed) {
      setHasUnread(false);
    }
  }, [isCollapsed]);

  // Extract loading/sending states
  const isLoading = activeConversation.isLoading;
  const isSending = sendMessage.isPending;

  // Streaming state for real-time event display
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([]);
  const [streamingContentBlocks, setStreamingContentBlocks] = useState<StreamingContentBlock[]>([]);
  const [streamingTasks, setStreamingTasks] = useState<Map<string, StreamingTask>>(new Map());

  // Unified actions hook (replaces useChatPanelHandlers action logic)
  const {
    handleSend,
    handleQueue,
    handleStopAgent,
    handleDeleteQueuedMessage,
    handleEditQueuedMessage,
    handleEditLastQueued,
  } = useChatActions({
    contextType,
    contextId,
    storeContextKey: contextKey,
    selectedTaskId: context.selectedTaskId,
    ideationSessionId: context.ideationSessionId,
    sendMessage,
    messageCount: messagesData.length,
  });

  // Wrapper for handleEditLastQueued that provides the queued messages
  const handleEditLastQueuedWrapper = useCallback(() => {
    handleEditLastQueued(queuedMessages);
  }, [handleEditLastQueued, queuedMessages]);

  // Wrapper for stop that clears streaming state
  const handleStopAgentWrapper = useCallback(async () => {
    await handleStopAgent();
    setStreamingToolCalls([]);
    setStreamingContentBlocks([]);
    setStreamingTasks(new Map());
  }, [handleStopAgent]);

  // Ref to track conversation ID that's finalizing (between message_created and query refetch)
  const finalizingConversationRef = useRef<string | null>(null);

  // Unified event subscriptions (replaces useChatPanelHandlers event logic)
  useChatEvents({
    activeConversationId,
    contextId,
    contextType,
    setStreamingToolCalls,
    setStreamingContentBlocks,
    setStreamingTasks,
    finalizingConversationRef,
  });

  // Hook events — listen for agent:hook Tauri events scoped to active conversation
  useAgentHookEvents(activeConversationId);
  const hookEvents = useHookEventsStore((s) => s.events);
  const activeHooksMap = useHookEventsStore((s) => s.activeHooks);
  const activeHooksList = useMemo(() => Array.from(activeHooksMap.values()), [activeHooksMap]);

  // Ask user question state — scoped to current context
  const questionSessionId = context.ideationSessionId ?? context.selectedTaskId ?? context.projectId;
  const {
    activeQuestion,
    answeredQuestion,
    submitAnswer,
    dismissQuestion,
    clearAnswered,
    isLoading: isSubmittingAnswer,
  } = useAskUserQuestion(questionSessionId);

  // Question UI state — extracted to hook (chip selection, input sync, question-aware send)
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

  // File attachments state — managed per conversation
  const {
    attachments,
    uploadFiles,
    removeAttachment,
    clearAttachments,
  } = useChatAttachments(activeConversationId ?? "");

  // Wrapper for send that handles attachments and team target
  const handleSendWithAttachments = useCallback(async (content: string) => {
    // Collect attachment IDs before sending
    const attachmentIds = attachments.map((a) => a.id);

    // Send message with attachment IDs and team target
    await handleSend(
      content,
      attachmentIds.length > 0 ? attachmentIds : undefined,
      isTeamActive ? sendTarget : undefined
    );

    // Clear attachments after successful send
    clearAttachments();
  }, [handleSend, clearAttachments, attachments, isTeamActive, sendTarget]);

  // Close with animation
  const handleClose = useCallback(() => {
    setIsExiting(true);
    setTimeout(() => {
      toggleChatVisible(context.view);
      setIsExiting(false);
    }, 200);
  }, [toggleChatVisible, context.view]);

  // Escape to close panel (only runs when panel is open via wrapper)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !isCollapsed) {
        handleClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isCollapsed, handleClose]);

  if (isCollapsed) {
    return (
      <CollapsedPanel
        onExpand={() => setIsCollapsed(false)}
        hasUnread={hasUnread}
      />
    );
  }

  return (
    <>
      <style>{animationStyles}</style>
      <ResizeablePanel
        width={width}
        isExiting={isExiting}
        testId="chat-panel"
        ariaLabel="Chat panel"
        ResizeHandle={ResizeHandle}
      >
        {/* Header */}
        <div
          data-testid="chat-panel-header"
          className="flex items-center justify-between h-11 px-3 border-b shrink-0"
          style={{
            borderColor: "hsla(220 10% 100% / 0.04)",
            background: "hsla(220 10% 100% / 0.02)",
          }}
        >
          <ContextIndicator context={context} isExecutionMode={isExecutionMode} />

          {/* Unified status + activity badge */}
          <StatusActivityBadge
            isAgentActive={isSending || isAgentRunning}
            agentType={
              isExecutionMode
                ? AGENT_WORKER
                : (isSending || isAgentRunning)
                  ? "agent"
                  : "idle" as AgentType
            }
            contextType={context.view}
            contextId={
              context.view === "ideation" && context.ideationSessionId
                ? context.ideationSessionId
                : context.selectedTaskId || null
            }
          />

          <div className="flex items-center gap-1 shrink-0">
            {/* Conversation Selector */}
            <ConversationSelector
              contextType={contextType}
              contextId={contextId}
              conversations={conversations.data ?? []}
              activeConversationId={activeConversationId}
              onSelectConversation={handleSelectConversation}
              onNewConversation={handleNewConversation}
              isLoading={conversations.isLoading}
            />
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => setIsCollapsed(true)}
              aria-label="Collapse chat panel"
            >
              <PanelRightClose className="w-[18px] h-[18px]" />
            </Button>
            <Button
              data-testid="chat-panel-close"
              variant="ghost"
              size="icon-sm"
              onClick={handleClose}
              aria-label="Close chat panel"
            >
              <X className="w-[18px] h-[18px]" />
            </Button>
          </div>
        </div>

        {/* Team Filter Tabs (team mode only) */}
        {isTeamActive && teammates.length > 0 && (
          <TeamFilterTabs
            teammates={teammates}
            activeFilter={teamFilter}
            onFilterChange={setTeamFilter}
          />
        )}

        {/* Messages Area */}
        {isLoading ? (
          <div data-testid="chat-panel-messages" className="flex-1 flex items-center justify-center">
            <div data-testid="chat-panel-loading" className="flex flex-col items-center gap-2">
              <div className="typing-dot w-2 h-2 rounded-full" style={{ backgroundColor: "var(--text-muted)" }} />
              <span className="text-xs" style={{ color: "var(--text-muted)" }}>Loading messages...</span>
            </div>
          </div>
        ) : messagesData.length === 0 ? (
          <div data-testid="chat-panel-messages" className="flex-1 flex items-center justify-center">
            <div data-testid="chat-panel-empty" className="text-center px-6">
              <span className="text-sm" style={{ color: "var(--text-muted)" }}>Start a conversation</span>
            </div>
          </div>
        ) : (
          <div data-testid="chat-panel-messages" className="flex-1 overflow-hidden">
            <ChatMessageList
              messages={messagesData}
              conversationId={activeConversationId}
              failedRun={showFailedBanner && failedRun ? { id: failedRun.id, errorMessage: failedRun.errorMessage! } : null}
              onDismissFailedRun={setDismissedErrorId}
              isSending={isSending}
              isAgentRunning={isAgentRunning}
              streamingToolCalls={streamingToolCalls}
              streamingTasks={streamingTasks}
              streamingContentBlocks={streamingContentBlocks}
              hookEvents={hookEvents}
              activeHooks={activeHooksList}
              finalizingConversationRef={finalizingConversationRef}
              teamFilter={activeTeam ? teamFilter : undefined}
              contextKey={activeTeam ? contextKey : undefined}
            />
          </div>
        )}

        {/* StreamingToolIndicator — outside scroll container so it's always visible.
            Filters out Task calls (shown as TaskSubagentCard) and diff calls (shown inline). */}
        {(isSending || isAgentRunning) && (() => {
          const otherToolCalls = streamingToolCalls.filter(
            (tc) => tc.name.toLowerCase() !== "task" &&
                    (!isDiffToolCall(tc.name) || tc.arguments == null)
          );
          return otherToolCalls.length > 0 ? (
            <div className="shrink-0 px-3 pb-2">
              <StreamingToolIndicator toolCalls={otherToolCalls} isActive={true} />
            </div>
          ) : null;
        })()}

        {/* Team Plan Approval (shown when lead requests plan approval) */}
        {pendingPlan && (
          <TeamPlanApproval
            plan={pendingPlan}
            contextKey={contextKey}
          />
        )}

        {/* Team Activity Panel (team mode only) */}
        {isTeamActive && teammates.length > 0 && (
          <TeamActivityPanel
            contextKey={contextKey}
            isHistorical={isTeamHistorical}
            onMessageTeammate={(name) => {
              setSendTarget(name);
            }}
            onStopTeammate={(name) => {
              teamActions.stopTeammate.mutate(name);
            }}
            onStopAll={() => {
              teamActions.stopTeam.mutate();
            }}
          />
        )}

        {/* Input Area */}
        <div className="border-t" style={{ borderColor: "var(--border-subtle)" }}>
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

          {/* Target Selector (team mode only) */}
          {isTeamActive && teammates.length > 0 && (
            <div className="px-3 pt-2">
              <TargetSelector
                teammates={teammates}
                value={sendTarget}
                onChange={setSendTarget}
              />
            </div>
          )}

          {/* Chat Input */}
          <div className="p-3">
            <ChatInput
              onSend={activeQuestion ? handleQuestionSend : handleSendWithAttachments}
              onQueue={isTeamActive ? (content) => handleQueue(content, sendTarget) : handleQueue}
              onStop={handleStopAgentWrapper}
              isAgentRunning={isAgentRunning}
              isSending={isSending || isSubmittingAnswer}
              hasQueuedMessages={queuedMessages.length > 0}
              onEditLastQueued={handleEditLastQueuedWrapper}
              placeholder={
                isExecutionMode
                  ? "Message worker... (will be sent when current response completes)"
                  : "Send a message..."
              }
              showHelperText={queuedMessages.length > 0 || !!activeQuestion}
              enableAttachments={true}
              attachments={attachments}
              onFilesSelected={uploadFiles}
              onRemoveAttachment={removeAttachment}
              {...(activeQuestion ? {
                value: questionInputValue,
                onChange: setQuestionInputValue,
                questionMode: {
                  optionCount: activeQuestion.options.length,
                  multiSelect: activeQuestion.multiSelect,
                  onMatchedOptions: handleMatchedOptions,
                },
              } : {})}
              autoFocus
            />
          </div>
        </div>
      </ResizeablePanel>
    </>
  );
}
