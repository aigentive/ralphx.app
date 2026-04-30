import { memo } from "react";

import type { AgentConversationWorkspace } from "@/api/chat";
import { BranchBasePicker } from "@/components/shared/BranchBasePicker";
import type { BranchBaseOption } from "@/components/shared/branchBaseOptions";

export const AgentConversationBaseLine = memo(function AgentConversationBaseLine({
  workspace,
}: {
  workspace: AgentConversationWorkspace | null;
}) {
  if (!workspace) {
    return null;
  }

  const baseLabel = workspace.baseDisplayName ?? workspace.baseRef;
  const option: BranchBaseOption = {
    key: `${workspace.baseRefKind}:${workspace.baseRef}`,
    label: baseLabel,
    detail: workspace.baseDisplayName ? workspace.baseRef : undefined,
    source: "local",
    selection: {
      kind:
        workspace.baseRefKind === "project_default" ||
        workspace.baseRefKind === "current_branch" ||
        workspace.baseRefKind === "local_branch"
          ? workspace.baseRefKind
          : "local_branch",
      ref: workspace.baseRef,
      displayName: baseLabel,
    },
  };

  return (
    <div
      className="flex min-w-0 justify-end"
      data-testid="agents-conversation-base"
    >
      <BranchBasePicker
        value={option.key}
        onValueChange={() => undefined}
        options={[option]}
        placeholder="Base branch"
        readOnly
      />
    </div>
  );
});
