import { CheckCircle2, Loader2, SendHorizontal, TriangleAlert } from "lucide-react";
import { InlineIndicator } from "./shared";
import {
  colors,
  getBool,
  getString,
  parseMcpToolResult,
  type ToolCallWidgetProps,
} from "./shared.constants";

function iconForState(state: "pending" | "sent" | "saved" | "error") {
  switch (state) {
    case "pending":
      return <Loader2 size={11} className="animate-spin" style={{ color: colors.accent }} />;
    case "saved":
      return <CheckCircle2 size={11} style={{ color: colors.success }} />;
    case "sent":
      return <SendHorizontal size={11} style={{ color: colors.accent }} />;
    case "error":
      return <TriangleAlert size={11} style={{ color: colors.error }} />;
  }
}

export function ProjectOrchestrationWidget({ toolCall }: ToolCallWidgetProps) {
  const normalizedName = toolCall.name.toLowerCase();
  const isSendingIdeationPrompt = normalizedName.includes("v1_send_ideation_message");
  const isPending = toolCall.result == null && !toolCall.error;

  if (toolCall.error) {
    return (
      <InlineIndicator
        icon={iconForState("error")}
        text="RalphX orchestration check failed"
      />
    );
  }

  if (isPending) {
    return (
      <InlineIndicator
        icon={iconForState("pending")}
        text={isSendingIdeationPrompt ? "Sending ideation prompt..." : "Syncing RalphX orchestration..."}
      />
    );
  }

  if (!isSendingIdeationPrompt) {
    return null;
  }

  const parsed = parseMcpToolResult(toolCall.result);
  const queuedAsPending =
    getBool(parsed, "queuedAsPending") ??
    getBool(parsed, "queued_as_pending") ??
    false;
  const nextAction =
    getString(parsed, "nextAction") ??
    getString(parsed, "next_action");
  const wasSavedForResume = queuedAsPending || nextAction === "wait_for_resume";

  return (
    <InlineIndicator
      icon={iconForState(wasSavedForResume ? "saved" : "sent")}
      text={wasSavedForResume ? "Ideation prompt saved" : "Ideation prompt sent"}
    />
  );
}
