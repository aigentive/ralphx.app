import type { ReactNode } from "react";
import type { ChatMessageResponse } from "@/api/chat";
import type { StreamingTask } from "@/types/streaming-task";
import { TextBubble } from "./TextBubble";
import { ToolCallIndicator, type ToolCall } from "./ToolCallIndicator";
import { normalizeDelegationTranscriptPayload } from "./delegation-tool-calls";

export type TaskCardTranscriptBlock =
  | { type: "text"; text: string }
  | { type: "tool_call"; toolCall: ToolCall }
  | { type: "activity"; label: string };

export interface TaskCardTranscriptEntry {
  id: string;
  role: "user" | "assistant";
  speakerLabel?: string;
  blocks: TaskCardTranscriptBlock[];
}

function normalizeSpeakerLabel(
  role: string,
  sender: string | null | undefined,
): string {
  if (sender) return sender;
  return role === "user" ? "User" : role;
}

function makeTextBlock(text: string | undefined): TaskCardTranscriptBlock | null {
  if (!text) return null;
  const trimmed = text.trim();
  return trimmed ? { type: "text", text: trimmed } : null;
}

export function buildTaskCardTranscriptEntryFromToolCall({
  entryId,
  bodyText,
  childToolCalls,
  speakerLabel,
}: {
  entryId: string;
  bodyText?: string | undefined;
  childToolCalls: ToolCall[];
  speakerLabel?: string | undefined;
}): TaskCardTranscriptEntry {
  const blocks: TaskCardTranscriptBlock[] = childToolCalls.map((toolCall) => ({
    type: "tool_call",
    toolCall,
  }));
  const textBlock = makeTextBlock(bodyText);
  if (textBlock) {
    blocks.push(textBlock);
  }

  return {
    id: entryId,
    role: "assistant",
    ...(speakerLabel ? { speakerLabel } : {}),
    blocks,
  };
}

export function buildTaskCardTranscriptEntryFromStreamingTask(
  task: StreamingTask,
): TaskCardTranscriptEntry {
  const blocks: TaskCardTranscriptBlock[] = task.childToolCalls
    .filter((toolCall) => !toolCall.name.startsWith("result:toolu"))
    .map((toolCall) => ({
      type: "tool_call" as const,
      toolCall,
    }));

  if (task.status === "running") {
    blocks.push({ type: "activity", label: "Working…" });
  }

  const textBlock = makeTextBlock(task.textOutput);
  if (textBlock) {
    blocks.push(textBlock);
  }

  return {
    id: task.toolUseId,
    role: "assistant",
    blocks,
  };
}

export function buildTaskCardTranscriptEntriesFromConversation(
  messages: ChatMessageResponse[],
): TaskCardTranscriptEntry[] {
  const entries: Array<TaskCardTranscriptEntry | null> = messages
    .map((message) => {
      const { contentBlocks, toolCalls } = normalizeDelegationTranscriptPayload({
        contentBlocks: message.contentBlocks,
        toolCalls: message.toolCalls,
      });

      const blocks: TaskCardTranscriptBlock[] = [];

      if (contentBlocks.length > 0) {
        for (const block of contentBlocks) {
          if (block.type === "text" && block.text) {
            blocks.push({ type: "text", text: block.text });
            continue;
          }
          if (block.type === "tool_use" && block.name) {
            blocks.push({
              type: "tool_call",
              toolCall: {
                id: block.id ?? `${message.id}-tool`,
                name: block.name,
                arguments: block.arguments ?? {},
                ...(block.result != null ? { result: block.result } : {}),
                ...(block.parentToolUseId
                  ? { parentToolUseId: block.parentToolUseId }
                  : {}),
              },
            });
          }
        }
      } else {
        const textBlock = makeTextBlock(message.content);
        if (textBlock) {
          blocks.push(textBlock);
        }
        for (const toolCall of toolCalls) {
          blocks.push({ type: "tool_call", toolCall });
        }
      }

      if (blocks.length === 0) {
        return null;
      }

      return {
        id: message.id,
        role: message.role === "user" ? "user" : "assistant",
        speakerLabel: normalizeSpeakerLabel(message.role, message.sender),
        blocks,
      };
    });

  return entries.filter((entry): entry is TaskCardTranscriptEntry => entry != null);
}

function ActivityDots() {
  return (
    <div className="flex items-center gap-1" aria-hidden="true">
      <div
        className="w-1.5 h-1.5 rounded-full animate-pulse"
        style={{ backgroundColor: "var(--accent-primary)" }}
      />
      <div
        className="w-1.5 h-1.5 rounded-full animate-pulse"
        style={{ backgroundColor: "var(--accent-primary)", animationDelay: "0.15s" }}
      />
      <div
        className="w-1.5 h-1.5 rounded-full animate-pulse"
        style={{ backgroundColor: "var(--accent-primary)", animationDelay: "0.3s" }}
      />
    </div>
  );
}

export function TaskCardTranscriptView({
  entries,
  dataTestId,
  emptyState,
}: {
  entries: TaskCardTranscriptEntry[];
  dataTestId?: string | undefined;
  emptyState?: ReactNode;
}) {
  if (entries.length === 0) {
    return emptyState ?? null;
  }

  return (
    <div
      className="space-y-3"
      data-testid={dataTestId}
    >
      {entries.map((entry) => (
        <div key={entry.id} className="space-y-2" data-testid="task-card-transcript-message">
          {entry.speakerLabel && (
            <div
              className="text-[10px] uppercase tracking-[0.08em]"
              style={{ color: "var(--text-muted, hsl(220 10% 50%))" }}
            >
              {entry.speakerLabel}
            </div>
          )}

          <div className="space-y-2">
            {entry.blocks.map((block, index) => {
              if (block.type === "text") {
                return (
                  <TextBubble
                    key={`${entry.id}-text-${index}`}
                    text={block.text}
                    isUser={entry.role === "user"}
                  />
                );
              }

              if (block.type === "tool_call") {
                return (
                  <ToolCallIndicator
                    key={block.toolCall.id}
                    compact
                    toolCall={block.toolCall}
                  />
                );
              }

              return (
                <div
                  key={`${entry.id}-activity-${index}`}
                  className="flex items-center gap-2 text-xs"
                  data-testid="task-card-transcript-activity"
                >
                  <ActivityDots />
                  <span style={{ color: "var(--text-muted, hsl(220 10% 50%))" }}>
                    {block.label}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
