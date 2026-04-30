import { Loader2, TriangleAlert } from "lucide-react";
import { ChildSessionWidget } from "./ChildSessionWidget";
import { InlineIndicator } from "./shared";
import {
  colors,
  parseMcpToolResult,
  type ToolCall,
  type ToolCallWidgetProps,
} from "./shared.constants";
import {
  canonicalProjectToolName,
  projectIdeationSessionId,
  shouldHideCompletedProjectOrchestrationToolCall,
} from "./ProjectOrchestrationWidget.utils";

function iconForState(state: "pending" | "error") {
  switch (state) {
    case "pending":
      return <Loader2 size={11} className="animate-spin" style={{ color: colors.accent }} />;
    case "error":
      return <TriangleAlert size={11} style={{ color: colors.error }} />;
  }
}

export function ProjectOrchestrationWidget({ toolCall }: ToolCallWidgetProps) {
  const canonicalName = canonicalProjectToolName(toolCall.name);
  const isSendingIdeationPrompt = canonicalName === "v1_send_ideation_message";
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

  if (shouldHideCompletedProjectOrchestrationToolCall(toolCall)) {
    return null;
  }

  const parsed = parseMcpToolResult(toolCall.result);
  const sessionId = projectIdeationSessionId(toolCall);

  if (isSendingIdeationPrompt && sessionId) {
    const childToolCall: ToolCall = {
      ...toolCall,
      name: "mcp__ralphx__v1_start_ideation",
      result: {
        ...parsed,
        sessionId,
        session_id: sessionId,
      },
    };
    return <ChildSessionWidget toolCall={childToolCall} />;
  }
  return null;
}
