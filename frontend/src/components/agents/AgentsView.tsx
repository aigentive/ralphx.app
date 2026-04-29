import type React from "react";

import { AgentsShellLayout } from "./AgentsShellLayout";
import { AgentsConversationMainRegion } from "./AgentsConversationMainRegion";
import { AgentsConversationSideRegions } from "./AgentsConversationSideRegions";
import { useAgentsViewController } from "./useAgentsViewController";

interface AgentsViewProps {
  projectId: string;
  onCreateProject: () => void;
  footer?: React.ReactNode;
}

export function AgentsView({
  projectId,
  onCreateProject,
  footer,
}: AgentsViewProps) {
  const {
    mainRegionProps,
    shellProps,
    sideRegionProps,
  } = useAgentsViewController({
    projectId,
    onCreateProject,
  });

  return (
    <AgentsShellLayout {...shellProps} footer={footer}>
      <AgentsConversationMainRegion {...mainRegionProps} />
      <AgentsConversationSideRegions {...sideRegionProps} />
    </AgentsShellLayout>
  );
}
