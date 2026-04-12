import { useConversation } from "@/hooks/useChat";
import {
  buildTaskCardTranscriptEntriesFromConversation,
  TaskCardTranscriptView,
} from "./TaskCardTranscript";

function FallbackText({ text }: { text: string }) {
  return (
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
      {text}
    </pre>
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
      <FallbackText text={fallbackText} />
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

  const entries = buildTaskCardTranscriptEntriesFromConversation(messages);

  if (entries.length === 0) {
    return fallbackText ? <FallbackText text={fallbackText} /> : null;
  }

  return (
    <div className="space-y-3">
      <div
        className="text-[10px] uppercase tracking-[0.08em]"
        style={{ color: "var(--text-muted, hsl(220 10% 50%))" }}
      >
        Delegated conversation
      </div>
      <TaskCardTranscriptView
        entries={entries}
        dataTestId="delegated-conversation-transcript"
      />
    </div>
  );
}
