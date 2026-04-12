import { useConversation } from "@/hooks/useChat";
import type { ChatMessageResponse } from "@/api/chat";
import { ToolCallIndicator } from "./ToolCallIndicator";
import { TextBubble } from "./TextBubble";
import {
  mergeDelegationContentBlocks,
  mergeDelegationToolCalls,
} from "./delegation-tool-calls";

function DelegatedTranscriptMessage({ message }: { message: ChatMessageResponse }) {
  const isUser = message.role === "user";
  const contentBlocks = mergeDelegationContentBlocks(message.contentBlocks ?? []);
  const toolCalls = mergeDelegationToolCalls(message.toolCalls ?? []);

  return (
    <div className="space-y-2">
      <div
        className="text-[10px] uppercase tracking-[0.08em]"
        style={{ color: "var(--text-muted, hsl(220 10% 50%))" }}
      >
        {message.sender ?? (isUser ? "User" : message.role)}
      </div>

      {contentBlocks.length > 0 ? (
        <div className="space-y-2">
          {contentBlocks.map((block, index) => {
            if (block.type === "text" && block.text) {
              return (
                <TextBubble
                  key={`${message.id}-text-${index}`}
                  text={block.text}
                  isUser={isUser}
                />
              );
            }

            if (block.type === "tool_use" && block.name) {
              return (
                <ToolCallIndicator
                  key={block.id ?? `${message.id}-tool-${index}`}
                  compact
                  toolCall={{
                    id: block.id ?? `${message.id}-tool-${index}`,
                    name: block.name,
                    arguments: block.arguments ?? {},
                    result: block.result,
                    ...(block.parentToolUseId
                      ? { parentToolUseId: block.parentToolUseId }
                      : {}),
                  }}
                />
              );
            }

            return null;
          })}
        </div>
      ) : message.content ? (
        <TextBubble text={message.content} isUser={isUser} />
      ) : null}

      {contentBlocks.length === 0 && toolCalls.length > 0 && (
        <div className="space-y-1">
          {toolCalls.map((toolCall) => (
            <ToolCallIndicator key={toolCall.id} compact toolCall={toolCall} />
          ))}
        </div>
      )}
    </div>
  );
}

export function TaskToolCallDelegatedTranscript({
  conversationId,
  fallbackText,
}: {
  conversationId: string;
  fallbackText: string | undefined;
}) {
  const delegatedConversation = useConversation(conversationId);
  const messages = delegatedConversation.data?.messages ?? [];

  if (delegatedConversation.isLoading) {
    return (
      <div
        className="text-[11px] px-2 py-1.5 rounded"
        style={{
          backgroundColor: "var(--bg-surface, hsl(220 10% 10%))",
          color: "var(--text-muted, hsl(220 10% 50%))",
        }}
      >
        Loading delegated conversation...
      </div>
    );
  }

  if (delegatedConversation.isError) {
    return fallbackText ? (
      <pre
        className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-h-64"
        style={{
          backgroundColor: "var(--bg-surface, hsl(220 10% 10%))",
          color: "var(--text-secondary, hsl(220 10% 80%))",
          fontFamily: "var(--font-mono)",
          wordBreak: "break-word",
          whiteSpace: "pre-wrap",
        }}
      >
        {fallbackText}
      </pre>
    ) : (
      <div
        className="text-[11px] px-2 py-1.5 rounded"
        style={{
          backgroundColor: "hsla(0 70% 50% / 0.1)",
          color: "hsl(0 70% 75%)",
        }}
      >
        Unable to load delegated conversation.
      </div>
    );
  }

  if (messages.length === 0) {
    return fallbackText ? (
      <pre
        className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-h-64"
        style={{
          backgroundColor: "var(--bg-surface, hsl(220 10% 10%))",
          color: "var(--text-secondary, hsl(220 10% 80%))",
          fontFamily: "var(--font-mono)",
          wordBreak: "break-word",
          whiteSpace: "pre-wrap",
        }}
      >
        {fallbackText}
      </pre>
    ) : null;
  }

  return (
    <div className="space-y-3" data-testid="delegated-conversation-transcript">
      <div
        className="text-[10px] uppercase tracking-[0.08em]"
        style={{ color: "var(--text-muted, hsl(220 10% 50%))" }}
      >
        Delegated conversation
      </div>
      {messages.map((message) => (
        <DelegatedTranscriptMessage key={message.id} message={message} />
      ))}
    </div>
  );
}
