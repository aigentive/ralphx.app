import { memo } from "react";

import type {
  AgentConversationWorkspace,
  AgentConversationWorkspaceMode,
} from "@/api/chat";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import type {
  AgentArtifactTab,
  AgentRuntimeSelection,
} from "@/stores/agentSessionStore";

import {
  getAgentConversationStoreKey,
  type AgentConversation,
} from "./agentConversations";
import { AgentComposerProjectLine, AgentComposerSurface } from "./AgentComposerSurface";
import { AgentConversationBaseLine } from "./AgentConversationBaseLine";
import { AgentsChatHeaderController } from "./AgentsChatHeaderController";
import {
  AGENT_CONVERSATION_MODE_OPTIONS,
} from "./agentConversationMode";
import {
  AGENT_MODEL_OPTIONS,
  AGENT_PROVIDER_OPTIONS,
} from "./agentOptions";
import { AgentsTerminalDockHost } from "./AgentsTerminalRegion";

const AGENTS_CHAT_CONTENT_WIDTH_CLASS = "max-w-[980px]";

interface AgentComposerOption {
  id: string;
  label: string;
  description?: string;
}

interface AgentsActiveConversationPanelProps {
  activeConversation: AgentConversation;
  activeConversationMode: AgentConversationWorkspaceMode | null;
  activeConversationModeLocked: boolean;
  activeProjectId: string;
  activeProjectOptions: AgentComposerOption[];
  activeWorkspace: AgentConversationWorkspace | null;
  attachedIdeationSessionId: string | null;
  hasAutoOpenArtifacts: boolean;
  normalizedActiveRuntime: AgentRuntimeSelection;
  onActiveConversationModeChange: (mode: AgentConversationWorkspaceMode) => void;
  onActiveModelChange: (modelId: string) => void;
  onAgentUserMessageSent: (event: {
    content: string;
    result: { conversationId: string };
  }) => void;
  onOpenPublishPane: () => void;
  onPreloadArtifacts: () => void;
  onPublishWorkspace: (conversationId: string) => Promise<void>;
  onRenameConversation: (conversationId: string, title: string) => Promise<void>;
  onSelectArtifact: (tab: AgentArtifactTab) => void;
  onToggleArtifacts: (conversationId: string) => void;
  publishShortcutLabel: string;
  publishingConversationId: string | null;
  selectedConversationId: string;
  setTerminalChatDockElement: (element: HTMLDivElement | null) => void;
  switchingConversationModeId: string | null;
  terminalUnavailableReason: string | null;
}

export const AgentsActiveConversationPanel = memo(function AgentsActiveConversationPanel({
  activeConversation,
  activeConversationMode,
  activeConversationModeLocked,
  activeProjectId,
  activeProjectOptions,
  activeWorkspace,
  attachedIdeationSessionId,
  hasAutoOpenArtifacts,
  normalizedActiveRuntime,
  onActiveConversationModeChange,
  onActiveModelChange,
  onAgentUserMessageSent,
  onOpenPublishPane,
  onPreloadArtifacts,
  onPublishWorkspace,
  onRenameConversation,
  onSelectArtifact,
  onToggleArtifacts,
  publishShortcutLabel,
  publishingConversationId,
  selectedConversationId,
  setTerminalChatDockElement,
  switchingConversationModeId,
  terminalUnavailableReason,
}: AgentsActiveConversationPanelProps) {
  return (
    <div className="flex-1 min-w-0 h-full flex flex-col">
      <div className="min-h-0 flex-1">
        <IntegratedChatPanel
          key={selectedConversationId}
          projectId={activeProjectId}
          {...(activeConversation.contextType === "ideation"
            ? { ideationSessionId: activeConversation.contextId }
            : {})}
          conversationIdOverride={selectedConversationId}
          selectedTaskIdOverride={null}
          storeContextKeyOverride={getAgentConversationStoreKey(activeConversation)}
          agentProcessContextIdOverride={
            activeConversation.contextType === "project"
              ? selectedConversationId
              : undefined
          }
          sendOptions={{
            conversationId: selectedConversationId,
            providerHarness: normalizedActiveRuntime.provider,
            modelId: normalizedActiveRuntime.modelId,
          }}
          onUserMessageSent={onAgentUserMessageSent}
          hideHeaderSessionControls
          hideSessionToolbar
          surfaceBackground="var(--bg-base)"
          contentWidthClassName={AGENTS_CHAT_CONTENT_WIDTH_CLASS}
          inputContainerClassName="shrink-0 bg-transparent px-4 pb-4 pt-3"
          renderComposer={(composerProps) => (
            <>
              <AgentComposerSurface
                dataTestId="agents-conversation-composer"
                actionTestId="agents-conversation-submit"
                onSend={composerProps.onSend}
                onStop={composerProps.onStop}
                agentStatus={composerProps.agentStatus}
                isSubmitting={composerProps.isSending}
                isReadOnly={composerProps.isReadOnly}
                autoFocus={composerProps.autoFocus}
                placeholder="Ask the agent to plan, build, debug, or review something"
                showHelperText={false}
                hasQueuedMessages={composerProps.hasQueuedMessages}
                onEditLastQueued={composerProps.onEditLastQueued}
                attachments={composerProps.attachments}
                enableAttachments={composerProps.enableAttachments}
                onFilesSelected={composerProps.onFilesSelected}
                onRemoveAttachment={composerProps.onRemoveAttachment}
                attachmentsUploading={composerProps.attachmentsUploading}
                {...(composerProps.value !== undefined
                  ? {
                      value: composerProps.value,
                      onChange: composerProps.onChange,
                    }
                  : {})}
                {...(composerProps.questionMode !== undefined
                  ? { questionMode: composerProps.questionMode }
                  : {})}
                submitLabel="Send"
                {...(activeConversationMode
                  ? {
                      mode: {
                        value: activeConversationMode,
                        onValueChange: (value: string) =>
                          onActiveConversationModeChange(value as AgentConversationWorkspaceMode),
                        options: AGENT_CONVERSATION_MODE_OPTIONS,
                        disabled:
                          activeConversationModeLocked ||
                          composerProps.agentStatus !== "idle" ||
                          switchingConversationModeId === selectedConversationId,
                      },
                    }
                  : {})}
                project={{
                  value: activeProjectId,
                  onValueChange: () => undefined,
                  options: activeProjectOptions,
                  placeholder: "Current project",
                  disabled: true,
                }}
                provider={{
                  value: normalizedActiveRuntime.provider,
                  onValueChange: () => undefined,
                  options: AGENT_PROVIDER_OPTIONS,
                  disabled: true,
                }}
                model={{
                  value: normalizedActiveRuntime.modelId,
                  onValueChange: onActiveModelChange,
                  options: AGENT_MODEL_OPTIONS[normalizedActiveRuntime.provider],
                }}
              />
              <div className="mt-2 flex w-full flex-wrap items-center justify-between gap-2 px-2">
                <AgentComposerProjectLine
                  value={activeProjectId}
                  onValueChange={() => undefined}
                  options={activeProjectOptions}
                  placeholder="Current project"
                  disabled
                />
                <AgentConversationBaseLine
                  workspace={activeWorkspace}
                />
              </div>
            </>
          )}
          {...(activeConversation.contextType === "project" && attachedIdeationSessionId
            ? { additionalQuestionSessionIds: [attachedIdeationSessionId] }
            : {})}
          headerContent={
            <AgentsChatHeaderController
              conversation={activeConversation}
              workspace={activeWorkspace}
              hasAutoOpenArtifacts={hasAutoOpenArtifacts}
              terminalUnavailableReason={terminalUnavailableReason}
              onRenameConversation={onRenameConversation}
              onPublishWorkspace={onPublishWorkspace}
              onOpenPublishPane={onOpenPublishPane}
              onPreloadArtifacts={onPreloadArtifacts}
              publishShortcutLabel={publishShortcutLabel}
              isPublishingWorkspace={publishingConversationId === selectedConversationId}
              onToggleArtifacts={onToggleArtifacts}
              onSelectArtifact={onSelectArtifact}
            />
          }
          emptyState={<div />}
        />
      </div>
      <AgentsTerminalDockHost
        dock="chat"
        conversationId={selectedConversationId}
        workspace={activeWorkspace}
        terminalUnavailableReason={terminalUnavailableReason}
        hasAutoOpenArtifacts={hasAutoOpenArtifacts}
        setDockElement={setTerminalChatDockElement}
      />
    </div>
  );
});
